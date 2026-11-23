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
static float booklandSpectrumLow[BooklandSpectrumBands];
static float booklandSpectrumHigh[BooklandSpectrumBands];
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
    const AudioBuffer *buffer = &buffers->mBuffers[0];
    if (!buffer->mData || booklandSpectrumFormat.mBitsPerChannel != 32) return;
    BOOL planar = (booklandSpectrumFormat.mFormatFlags &
        kAudioFormatFlagIsNonInterleaved) != 0;
    uint32_t channels = planar ? 1 : MAX(1, booklandSpectrumFormat.mChannelsPerFrame);
    uint32_t frames = (uint32_t)(buffer->mDataByteSize / (sizeof(float) * channels));
    frames = MIN(frames, 8192);
    if (!frames) return;
    const float *samples = (const float *)buffer->mData;
    float sum[BooklandSpectrumBands] = {0};
    uint32_t count = 0;
    for (uint32_t frame = 0; frame < frames; frame += 2) {
        float value = samples[frame * channels];
        if (!planar && channels > 1) {
            value = (value + samples[frame * channels + 1]) * 0.5f;
        }
        if (!isfinite(value)) continue;
        count++;
        for (int band = 0; band < BooklandSpectrumBands; band++) {
            booklandSpectrumHigh[band] += booklandSpectrumHighCoefficient[band] *
                (value - booklandSpectrumHigh[band]);
            booklandSpectrumLow[band] += booklandSpectrumLowCoefficient[band] *
                (value - booklandSpectrumLow[band]);
            float output = booklandSpectrumHigh[band] - booklandSpectrumLow[band];
            sum[band] += output * output;
        }
    }
    if (!count) return;
    for (int band = 0; band < BooklandSpectrumBands; band++) {
        float rms = sqrtf(sum[band] / count);
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
        if (atomic_load_explicit(&booklandSpectrumWanted, memory_order_acquire)) {
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
void bookland_audio_spectrum_test_tone(float frequency, float *levels, size_t count) {
    booklandSpectrumFormat = (AudioStreamBasicDescription) {
        .mSampleRate = 48000, .mFormatID = kAudioFormatLinearPCM,
        .mFormatFlags = kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked,
        .mBytesPerPacket = 8, .mFramesPerPacket = 1,
        .mBytesPerFrame = 8, .mChannelsPerFrame = 2, .mBitsPerChannel = 32
    };
    booklandSpectrumPrepare(48000);
    float samples[1024 * 2];
    for (int block = 0; block < 32; block++) {
        for (int frame = 0; frame < 1024; frame++) {
            float value = sinf(2.0f * (float)M_PI * frequency *
                (float)(block * 1024 + frame) / 48000.0f) * 0.5f;
            samples[frame * 2] = value;
            samples[frame * 2 + 1] = value;
        }
        AudioBufferList buffer = { .mNumberBuffers = 1,
            .mBuffers = {{ .mNumberChannels = 2,
                .mDataByteSize = sizeof(samples), .mData = samples }} };
        booklandSpectrumConsume(&buffer);
    }
    if (levels) {
        for (size_t index = 0; index < count && index < BooklandSpectrumBands; index++) {
            uint32_t bits = atomic_load_explicit(&booklandSpectrumLevels[index],
                memory_order_relaxed);
            memcpy(&levels[index], &bits, sizeof(bits));
        }
    }
}
