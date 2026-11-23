#import <Foundation/Foundation.h>
#import <CoreAudio/CoreAudio.h>
#import <CoreAudio/CATapDescription.h>
#import <CoreAudio/AudioHardwareTapping.h>
#include <math.h>
#include <stdatomic.h>
#include <string.h>

// Core Audio taps deliver real output samples without a video stream. Keep
// capture off until the user opens Music and opts into the spectrum.
enum { BooklandSpectrumBands = 16 };
static dispatch_queue_t booklandSpectrumControl;
static _Atomic bool booklandSpectrumWanted;
static _Atomic int booklandSpectrumState; // 0 off, 1 starting, 2 running, -1 unavailable
static _Atomic uint32_t booklandSpectrumLevels[BooklandSpectrumBands];
static AudioObjectID booklandSpectrumTap = kAudioObjectUnknown;
static AudioObjectID booklandSpectrumDevice = kAudioObjectUnknown;
static AudioDeviceIOProcID booklandSpectrumProc;
static AudioStreamBasicDescription booklandSpectrumFormat;
static float booklandSpectrumLow[2][BooklandSpectrumBands];
static float booklandSpectrumHigh[2][BooklandSpectrumBands];
static float booklandSpectrumLowCoefficient[BooklandSpectrumBands];
static float booklandSpectrumHighCoefficient[BooklandSpectrumBands];
static float booklandSpectrumSmoothed[BooklandSpectrumBands];

static AudioObjectPropertyAddress booklandSpectrumAddress(AudioObjectPropertySelector selector) {
    return (AudioObjectPropertyAddress) { selector, kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain };
}

static void booklandSpectrumClear(void) {
    memset(booklandSpectrumLow, 0, sizeof(booklandSpectrumLow));
    memset(booklandSpectrumHigh, 0, sizeof(booklandSpectrumHigh));
    memset(booklandSpectrumSmoothed, 0, sizeof(booklandSpectrumSmoothed));
    for (int index = 0; index < BooklandSpectrumBands; index++) {
        atomic_store_explicit(&booklandSpectrumLevels[index], 0, memory_order_relaxed);
    }
}

static void booklandSpectrumPrepare(float sourceSampleRate) {
    booklandSpectrumClear();
    float sampleRate = sourceSampleRate * 0.5f;
    for (int index = 0; index < BooklandSpectrumBands; index++) {
        float lower = 55.0f * powf(1.38f, index);
        float upper = fminf(sampleRate * 0.42f, lower * 1.55f);
        booklandSpectrumLowCoefficient[index] = 1.0f -
            expf(-2.0f * (float)M_PI * lower / sampleRate);
        booklandSpectrumHighCoefficient[index] = 1.0f -
            expf(-2.0f * (float)M_PI * upper / sampleRate);
    }
}

static void booklandSpectrumConsume(const AudioBufferList *buffers) {
    if (!buffers || buffers->mNumberBuffers == 0) return;
    if (booklandSpectrumFormat.mBitsPerChannel != 32) return;
    BOOL planar = (booklandSpectrumFormat.mFormatFlags &
        kAudioFormatFlagIsNonInterleaved) != 0;
    uint32_t stride = planar ? 1 : MAX(1, booklandSpectrumFormat.mChannelsPerFrame);
    uint32_t channels = planar ? MIN(2, buffers->mNumberBuffers) : MIN(2, stride);
    if (!channels) return;
    uint32_t frames = UINT32_MAX;
    for (uint32_t channel = 0; channel < (planar ? channels : 1); channel++) {
        const AudioBuffer *buffer = &buffers->mBuffers[channel];
        if (!buffer->mData) return;
        uint32_t available = (uint32_t)(buffer->mDataByteSize /
            (sizeof(float) * stride));
        frames = MIN(frames, available);
    }
    frames = MIN(frames, 8192);
    if (!frames) return;
    float sum[BooklandSpectrumBands] = {0};
    uint32_t count = 0;
    for (uint32_t frame = 0; frame < frames; frame += 2) {
        BOOL valid = YES;
        float values[2];
        for (uint32_t channel = 0; channel < channels; channel++) {
            const float *samples = (const float *)buffers->mBuffers[planar ? channel : 0].mData;
            values[channel] = samples[frame * stride + (planar ? 0 : channel)];
            if (!isfinite(values[channel])) valid = NO;
        }
        if (!valid) continue;
        count++;
        for (uint32_t channel = 0; channel < channels; channel++) {
            float value = values[channel];
            for (int band = 0; band < BooklandSpectrumBands; band++) {
                booklandSpectrumHigh[channel][band] +=
                    booklandSpectrumHighCoefficient[band] *
                    (value - booklandSpectrumHigh[channel][band]);
                booklandSpectrumLow[channel][band] +=
                    booklandSpectrumLowCoefficient[band] *
                    (value - booklandSpectrumLow[channel][band]);
                float output = booklandSpectrumHigh[channel][band] -
                    booklandSpectrumLow[channel][band];
                sum[band] += output * output;
            }
        }
    }
    if (!count) return;
    for (int band = 0; band < BooklandSpectrumBands; band++) {
        float rms = sqrtf(sum[band] / (count * channels));
        float decibels = 20.0f * log10f(rms + 0.000001f);
        float level = fmaxf(0, fminf(1, (decibels + 65.0f) / 47.0f));
        booklandSpectrumSmoothed[band] += 0.38f * (level - booklandSpectrumSmoothed[band]);
        uint32_t bits;
        memcpy(&bits, &booklandSpectrumSmoothed[band], sizeof(bits));
        atomic_store_explicit(&booklandSpectrumLevels[band], bits, memory_order_relaxed);
    }
}

static void booklandSpectrumStop(void) {
    if (booklandSpectrumDevice != kAudioObjectUnknown && booklandSpectrumProc) {
        AudioDeviceStop(booklandSpectrumDevice, booklandSpectrumProc);
        AudioDeviceDestroyIOProcID(booklandSpectrumDevice, booklandSpectrumProc);
        booklandSpectrumProc = NULL;
    }
    if (booklandSpectrumDevice != kAudioObjectUnknown) {
        AudioHardwareDestroyAggregateDevice(booklandSpectrumDevice);
        booklandSpectrumDevice = kAudioObjectUnknown;
    }
    if (booklandSpectrumTap != kAudioObjectUnknown) {
        AudioHardwareDestroyProcessTap(booklandSpectrumTap);
        booklandSpectrumTap = kAudioObjectUnknown;
    }
    booklandSpectrumClear();
    atomic_store_explicit(&booklandSpectrumState, 0, memory_order_release);
}

static BOOL booklandSpectrumConfigure(void) {
    CATapDescription *tap = [[CATapDescription alloc]
        initStereoGlobalTapButExcludeProcesses:@[]];
    tap.name = @"Bookland Music Spectrum";
    tap.privateTap = YES;
    if (AudioHardwareCreateProcessTap(tap, &booklandSpectrumTap) != noErr) return NO;

    AudioObjectPropertyAddress formatAddress = booklandSpectrumAddress(kAudioTapPropertyFormat);
    UInt32 size = sizeof(booklandSpectrumFormat);
    if (AudioObjectGetPropertyData(booklandSpectrumTap, &formatAddress, 0, NULL,
        &size, &booklandSpectrumFormat) != noErr ||
        booklandSpectrumFormat.mFormatID != kAudioFormatLinearPCM ||
        !(booklandSpectrumFormat.mFormatFlags & kAudioFormatFlagIsFloat) ||
        booklandSpectrumFormat.mBitsPerChannel != 32 ||
        booklandSpectrumFormat.mSampleRate < 8000) return NO;

    AudioObjectPropertyAddress uidAddress = booklandSpectrumAddress(kAudioTapPropertyUID);
    CFStringRef tapUID = NULL;
    size = sizeof(tapUID);
    if (AudioObjectGetPropertyData(booklandSpectrumTap, &uidAddress, 0, NULL,
        &size, &tapUID) != noErr || !tapUID) return NO;
    NSString *uid = (__bridge_transfer NSString *)tapUID;
    NSDictionary *description = @{
        [NSString stringWithUTF8String:kAudioAggregateDeviceNameKey]: @"Bookland Spectrum Input",
        [NSString stringWithUTF8String:kAudioAggregateDeviceUIDKey]: NSUUID.UUID.UUIDString,
        [NSString stringWithUTF8String:kAudioAggregateDeviceIsPrivateKey]: @YES,
        [NSString stringWithUTF8String:kAudioAggregateDeviceTapListKey]: @[
            @{[NSString stringWithUTF8String:kAudioSubTapUIDKey]: uid}]
    };
    if (AudioHardwareCreateAggregateDevice((__bridge CFDictionaryRef)description,
        &booklandSpectrumDevice) != noErr) return NO;

    booklandSpectrumPrepare((float)booklandSpectrumFormat.mSampleRate);
    OSStatus result = AudioDeviceCreateIOProcIDWithBlock(&booklandSpectrumProc,
        booklandSpectrumDevice, NULL,
        ^(const AudioTimeStamp *now, const AudioBufferList *input,
          const AudioTimeStamp *inputTime, AudioBufferList *output,
          const AudioTimeStamp *outputTime) {
            (void)now; (void)inputTime; (void)output; (void)outputTime;
            booklandSpectrumConsume(input);
        });
    return result == noErr;
}

static BOOL booklandSpectrumStart(void) {
    if (!booklandSpectrumConfigure()) return NO;
    // Starting the aggregate can trigger macOS System Audio Recording consent.
    return AudioDeviceStart(booklandSpectrumDevice, booklandSpectrumProc) == noErr;
}

void bookland_audio_spectrum_set(int enabled) {
    bool desired = enabled != 0;
    if (atomic_exchange_explicit(&booklandSpectrumWanted, desired,
        memory_order_acq_rel) == desired) return;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        booklandSpectrumControl = dispatch_queue_create("local.bookland.audio-spectrum",
            DISPATCH_QUEUE_SERIAL);
    });
    dispatch_async(booklandSpectrumControl, ^{
        if (desired) {
            if (!atomic_load_explicit(&booklandSpectrumWanted,
                memory_order_acquire)) return;
            // Several rapid hover changes can queue start blocks. A live tap
            // must be reused; creating another would leak the first device.
            if (atomic_load_explicit(&booklandSpectrumState,
                memory_order_acquire) != 0) return;
            atomic_store_explicit(&booklandSpectrumState, 1, memory_order_release);
            if (booklandSpectrumStart()) {
                atomic_store_explicit(&booklandSpectrumState, 2, memory_order_release);
            } else {
                booklandSpectrumStop();
                atomic_store_explicit(&booklandSpectrumState, -1, memory_order_release);
            }
            if (!atomic_load_explicit(&booklandSpectrumWanted, memory_order_acquire)) {
                booklandSpectrumStop();
            }
        } else {
            // Honor an off transition even if a later on was queued. This
            // clears a failed start before that later request can retry.
            booklandSpectrumStop();
        }
    });
}

int bookland_audio_spectrum_copy(float *levels, size_t count) {
    if (levels) {
        for (size_t index = 0; index < count && index < BooklandSpectrumBands; index++) {
            uint32_t bits = atomic_load_explicit(&booklandSpectrumLevels[index],
                memory_order_relaxed);
            memcpy(&levels[index], &bits, sizeof(bits));
        }
    }
    return atomic_load_explicit(&booklandSpectrumState, memory_order_acquire);
}

// A deterministic signal exercises the same buffer parser and filter bank
// without requesting macOS System Audio Recording in a unit-test process.
void bookland_audio_spectrum_test_tone(float frequency, int channelMode,
                                       float *levels, size_t count) {
    booklandSpectrumFormat = (AudioStreamBasicDescription) {
        .mSampleRate = 48000, .mFormatID = kAudioFormatLinearPCM,
        .mFormatFlags = kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked |
            (channelMode == 2 ? kAudioFormatFlagIsNonInterleaved : 0),
        .mBytesPerPacket = channelMode == 2 ? 4 : 8, .mFramesPerPacket = 1,
        .mBytesPerFrame = channelMode == 2 ? 4 : 8,
        .mChannelsPerFrame = 2, .mBitsPerChannel = 32
    };
    booklandSpectrumPrepare(48000);
    float samples[1024 * 2];
    float planar[2][1024];
    for (int block = 0; block < 32; block++) {
        for (int frame = 0; frame < 1024; frame++) {
            float value = sinf(2.0f * (float)M_PI * frequency *
                (float)(block * 1024 + frame) / 48000.0f) * 0.5f;
            samples[frame * 2] = value;
            samples[frame * 2 + 1] = channelMode == 1 ? -value : value;
            planar[0][frame] = value;
            planar[1][frame] = value;
        }
        AudioBufferList interleaved = { .mNumberBuffers = 1,
            .mBuffers = {{ .mNumberChannels = 2,
                .mDataByteSize = sizeof(samples), .mData = samples }} };
        struct { UInt32 mNumberBuffers; AudioBuffer mBuffers[2]; } separated = {
            .mNumberBuffers = 2,
            .mBuffers = {
                { .mNumberChannels = 1, .mDataByteSize = sizeof(planar[0]),
                    .mData = planar[0] },
                { .mNumberChannels = 1, .mDataByteSize = sizeof(planar[1]),
                    .mData = planar[1] }
            }
        };
        booklandSpectrumConsume(channelMode == 2
            ? (const AudioBufferList *)&separated : &interleaved);
    }
    if (levels) {
        for (size_t index = 0; index < count && index < BooklandSpectrumBands; index++) {
            uint32_t bits = atomic_load_explicit(&booklandSpectrumLevels[index],
                memory_order_relaxed);
            memcpy(&levels[index], &bits, sizeof(bits));
        }
    }
}
