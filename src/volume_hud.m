#import <AppKit/AppKit.h>
#import <CoreAudio/CoreAudio.h>
#import <QuartzCore/QuartzCore.h>
#import <math.h>

// A volume activity card uses Core Audio notifications and Core Animation.
// It has no timer or rendering work while hidden.
static BOOL booklandVolumeEnabled;
static AudioObjectID booklandOutputDevice = kAudioObjectUnknown;
static BOOL booklandVolumeListening[3];
static BOOL booklandMuteListening[3];
static AudioObjectPropertyListenerBlock booklandDeviceListener;
static AudioObjectPropertyListenerBlock booklandLevelListener;
static NSWindow *booklandVolumeWindow;
static NSGlassEffectView *booklandVolumeGlass;
static NSView *booklandVolumeOverlay;
static BOOL booklandVolumeInline;
static CGFloat booklandVolumeClosedWidth = 255;
enum { BooklandVolumeSegmentCount = 16 };
static CALayer *booklandVolumeTrack[BooklandVolumeSegmentCount];
static CALayer *booklandVolumeFill[BooklandVolumeSegmentCount];
static CATextLayer *booklandVolumeText;
static Float32 booklandLastVolume = -1;
static BOOL booklandLastMuted;
static uint64_t booklandHudGeneration;

static AudioObjectPropertyAddress booklandAddress(AudioObjectPropertySelector selector,
                                                  AudioObjectPropertyElement element) {
    return (AudioObjectPropertyAddress) { selector, kAudioDevicePropertyScopeOutput, element };
}

static AudioObjectID booklandDefaultOutputDevice(void) {
    AudioObjectID device = kAudioObjectUnknown;
    UInt32 size = sizeof(device);
    AudioObjectPropertyAddress address = {
        kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain
    };
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL,
                                   &size, &device) != noErr) return kAudioObjectUnknown;
    return device;
}

static BOOL booklandReadScalar(AudioObjectID device, AudioObjectPropertyElement element,
                               Float32 *value) {
    AudioObjectPropertyAddress address = booklandAddress(kAudioDevicePropertyVolumeScalar, element);
    UInt32 size = sizeof(*value);
    return AudioObjectHasProperty(device, &address) &&
        AudioObjectGetPropertyData(device, &address, 0, NULL, &size, value) == noErr;
}

static Float32 booklandVolumeForDevice(AudioObjectID device) {
    if (device == kAudioObjectUnknown) return -1;
    Float32 value = 0;
    if (booklandReadScalar(device, kAudioObjectPropertyElementMain, &value)) {
        return fmaxf(0, fminf(1, value));
    }
    Float32 sum = 0;
    int count = 0;
    for (AudioObjectPropertyElement element = 1; element <= 2; element++) {
        if (booklandReadScalar(device, element, &value)) {
            sum += value;
            count++;
        }
    }
    return count ? fmaxf(0, fminf(1, sum / count)) : -1;
}

static Float32 booklandVolume(void) {
    return booklandVolumeForDevice(booklandOutputDevice);
}

float bookland_system_volume_get(void) {
    return booklandVolumeForDevice(booklandDefaultOutputDevice());
}

int bookland_system_volume_set(float value) {
    if (!isfinite(value)) return 0;
    AudioObjectID device = booklandDefaultOutputDevice();
    if (device == kAudioObjectUnknown) return 0;
    Float32 bounded = fmaxf(0, fminf(1, value));
    BOOL changed = NO;
    for (AudioObjectPropertyElement element = 0; element <= 2; element++) {
        AudioObjectPropertyAddress address = booklandAddress(
            kAudioDevicePropertyVolumeScalar, element);
        Boolean settable = false;
        if (!AudioObjectHasProperty(device, &address) ||
            AudioObjectIsPropertySettable(device, &address, &settable) != noErr || !settable) {
            continue;
        }
        if (AudioObjectSetPropertyData(device, &address, 0, NULL,
                                       sizeof(bounded), &bounded) == noErr) {
            changed = YES;
            if (element == kAudioObjectPropertyElementMain) break;
        }
    }
    return changed ? 1 : 0;
}

static BOOL booklandMuted(void) {
    if (booklandOutputDevice == kAudioObjectUnknown) return NO;
    for (AudioObjectPropertyElement element = 0; element <= 2; element++) {
        AudioObjectPropertyAddress address = booklandAddress(kAudioDevicePropertyMute, element);
        UInt32 mute = 0;
        UInt32 size = sizeof(mute);
        if (AudioObjectHasProperty(booklandOutputDevice, &address) &&
            AudioObjectGetPropertyData(booklandOutputDevice, &address, 0, NULL, &size, &mute) == noErr) {
            return mute != 0;
        }
    }
    return NO;
}

static void booklandPrepareVolumeWindow(void) {
    if (booklandVolumeWindow) return;
    NSWindow *window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, 230, 56)
                  styleMask:NSWindowStyleMaskBorderless
                    backing:NSBackingStoreBuffered defer:NO];
    window.opaque = NO;
    window.backgroundColor = NSColor.clearColor;
    window.hasShadow = YES;
    window.ignoresMouseEvents = YES;
    window.level = NSStatusWindowLevel;
    window.collectionBehavior = NSWindowCollectionBehaviorCanJoinAllSpaces |
                                NSWindowCollectionBehaviorFullScreenAuxiliary;
    NSGlassEffectView *glass = [[NSGlassEffectView alloc]
        initWithFrame:NSMakeRect(0, 0, 230, 56)];
    glass.wantsLayer = YES;
    glass.style = NSGlassEffectViewStyleRegular;
    glass.cornerRadius = 25;
    glass.tintColor = [NSColor colorWithCalibratedRed:0.35 green:0.42 blue:0.55 alpha:0.06];
    NSView *overlay = [[NSView alloc] initWithFrame:glass.bounds];
    overlay.wantsLayer = YES;
    overlay.layer.backgroundColor = NSColor.clearColor.CGColor;
    [glass addSubview:overlay];
    window.contentView = glass;

    CATextLayer *text = [CATextLayer layer];
    text.frame = CGRectMake(20, 28, 190, 18);
    text.contentsScale = NSScreen.mainScreen.backingScaleFactor ?: 2.0;
    text.font = (__bridge CFTypeRef)[NSFont systemFontOfSize:12 weight:NSFontWeightSemibold];
    text.fontSize = 12;
    text.foregroundColor = NSColor.whiteColor.CGColor;
    [overlay.layer addSublayer:text];

    // Native HUDs use separate illuminated ticks. Keep these layers allocated
    // while hidden; a level change animates only ticks that crossed a boundary.
    for (int index = 0; index < BooklandVolumeSegmentCount; index++) {
        CGRect segmentFrame = CGRectMake(21 + index * 11.75, 13, 8, 7);
        CALayer *track = [CALayer layer];
        track.frame = segmentFrame;
        track.cornerRadius = 2.5;
        track.backgroundColor = [NSColor colorWithCalibratedWhite:1 alpha:0.23].CGColor;
        [overlay.layer addSublayer:track];
        booklandVolumeTrack[index] = track;
        CALayer *fill = [CALayer layer];
        fill.frame = segmentFrame;
        fill.cornerRadius = 2.5;
        fill.backgroundColor = [NSColor colorWithCalibratedRed:0.48 green:0.70 blue:1 alpha:1].CGColor;
        fill.opacity = 0;
        [overlay.layer addSublayer:fill];
        booklandVolumeFill[index] = fill;
    }
    booklandVolumeWindow = window;
    booklandVolumeGlass = glass;
    booklandVolumeOverlay = overlay;
    booklandVolumeText = text;
}

static void booklandLayoutVolume(void) {
    if (!booklandVolumeWindow) return;
    CGFloat width = booklandVolumeInline ? 211 : 230;
    CGFloat height = booklandVolumeInline ? 33 : 56;
    [booklandVolumeWindow setContentSize:NSMakeSize(width, height)];
    booklandVolumeGlass.frame = NSMakeRect(0, 0, width, height);
    booklandVolumeGlass.cornerRadius = booklandVolumeInline ? 15 : 25;
    booklandVolumeOverlay.frame = booklandVolumeGlass.bounds;
    booklandVolumeText.frame = booklandVolumeInline
        ? CGRectMake(10, 9, 58, 16) : CGRectMake(20, 28, 190, 18);
    booklandVolumeText.fontSize = booklandVolumeInline ? 10 : 12;
    for (int index = 0; index < BooklandVolumeSegmentCount; index++) {
        CGRect frame = booklandVolumeInline
            ? CGRectMake(70 + index * 8.2, 13, 5.5, 7)
            : CGRectMake(21 + index * 11.75, 13, 8, 7);
        booklandVolumeTrack[index].frame = frame;
        booklandVolumeFill[index].frame = frame;
        booklandVolumeTrack[index].cornerRadius = booklandVolumeInline ? 2 : 2.5;
        booklandVolumeFill[index].cornerRadius = booklandVolumeInline ? 2 : 2.5;
    }
}

static void booklandVolumePosition(void) {
    NSScreen *screen = nil;
    for (NSWindow *window in NSApp.windows) {
        if ([window.title isEqualToString:@"Bookland"] && window.isVisible) {
            screen = window.screen;
            break;
        }
    }
    if (!screen) screen = NSScreen.mainScreen;
    if (!screen || !booklandVolumeWindow) return;
    if (booklandVolumeInline) {
        CGFloat x = NSMidX(screen.frame) - booklandVolumeClosedWidth * 0.5 - 217;
        if (x < NSMinX(screen.frame) + 8) {
            x = NSMidX(screen.frame) + booklandVolumeClosedWidth * 0.5 + 6;
        }
        [booklandVolumeWindow setFrameOrigin:NSMakePoint(x, NSMaxY(screen.frame) - 33)];
    } else {
        CGFloat top = NSMaxY(screen.frame) - fmax(36, screen.safeAreaInsets.top) - 8;
        [booklandVolumeWindow setFrameOrigin:NSMakePoint(NSMidX(screen.frame) - 115, top - 56)];
    }
}

static void booklandFadeVolume(CGFloat target, CFTimeInterval duration) {
    CALayer *layer = booklandVolumeGlass.layer;
    CGFloat from = ((CALayer *)layer.presentationLayer).opacity;
    if (!layer.presentationLayer) from = layer.opacity;
    CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
    fade.fromValue = @(from);
    fade.toValue = @(target);
    fade.duration = duration;
    fade.timingFunction = [CAMediaTimingFunction
        functionWithName:kCAMediaTimingFunctionEaseOut];
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    layer.opacity = target;
    [layer addAnimation:fade forKey:@"bookland-volume-opacity"];
    [CATransaction commit];
}

static void booklandShowVolume(Float32 volume, BOOL muted) {
    booklandPrepareVolumeWindow();
    booklandLayoutVolume();
    booklandVolumePosition();
    Float32 visible = muted ? 0 : volume;
    int lit = (int)lroundf(visible * BooklandVolumeSegmentCount);
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    for (int index = 0; index < BooklandVolumeSegmentCount; index++) {
        CALayer *fill = booklandVolumeFill[index];
        float target = index < lit ? 1 : 0;
        if (fill.opacity == target) continue;
        float previous = fill.presentationLayer ? fill.presentationLayer.opacity : fill.opacity;
        CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
        fade.fromValue = @(previous);
        fade.toValue = @(target);
        fade.duration = 0.16;
        fade.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseOut];
        fill.opacity = target;
        [fill addAnimation:fade forKey:@"bookland-volume-tick"];
    }
    booklandVolumeText.string = booklandVolumeInline
        ? (muted ? @"MUTE" : [NSString stringWithFormat:@"VOL %d", (int)lroundf(volume * 100)])
        : (muted ? @"Volume · Muted" :
            [NSString stringWithFormat:@"Volume · %d%%", (int)lroundf(volume * 100)]);
    [CATransaction commit];
    if (!booklandVolumeWindow.isVisible) {
        booklandVolumeGlass.layer.opacity = 0;
        [booklandVolumeWindow orderFront:nil];
    }
    booklandFadeVolume(1, 0.13);
    uint64_t generation = ++booklandHudGeneration;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(1.25 * NSEC_PER_SEC)),
        dispatch_get_main_queue(), ^{
            if (generation != booklandHudGeneration || !booklandVolumeEnabled) return;
            booklandFadeVolume(0, 0.22);
            dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.23 * NSEC_PER_SEC)),
                dispatch_get_main_queue(), ^{
                    if (generation == booklandHudGeneration) [booklandVolumeWindow orderOut:nil];
                });
        });
}

static void booklandVolumeChanged(void) {
    Float32 volume = booklandVolume();
    BOOL muted = booklandMuted();
    if (volume < 0) return;
    if (booklandLastVolume < 0) {
        booklandLastVolume = volume;
        booklandLastMuted = muted;
        return;
    }
    if (fabsf(booklandLastVolume - volume) < 0.005 && booklandLastMuted == muted) return;
    booklandLastVolume = volume;
    booklandLastMuted = muted;
    booklandShowVolume(volume, muted);
}

static void booklandUnbindOutputDevice(void) {
    if (booklandOutputDevice != kAudioObjectUnknown) {
        for (AudioObjectPropertyElement element = 0; element <= 2; element++) {
            AudioObjectPropertyAddress volume = booklandAddress(kAudioDevicePropertyVolumeScalar, element);
            AudioObjectPropertyAddress mute = booklandAddress(kAudioDevicePropertyMute, element);
            if (booklandVolumeListening[element])
                AudioObjectRemovePropertyListenerBlock(booklandOutputDevice, &volume,
                    dispatch_get_main_queue(), booklandLevelListener);
            if (booklandMuteListening[element])
                AudioObjectRemovePropertyListenerBlock(booklandOutputDevice, &mute,
                    dispatch_get_main_queue(), booklandLevelListener);
            booklandVolumeListening[element] = NO;
            booklandMuteListening[element] = NO;
        }
    }
    booklandOutputDevice = kAudioObjectUnknown;
}

static void booklandBindOutputDevice(void) {
    if (!booklandLevelListener) {
        booklandLevelListener = ^(UInt32 count, const AudioObjectPropertyAddress *addresses) {
            (void)count; (void)addresses;
            if (booklandVolumeEnabled) booklandVolumeChanged();
        };
    }
    booklandUnbindOutputDevice();
    AudioObjectID device = booklandDefaultOutputDevice();
    booklandOutputDevice = device;
    if (device == kAudioObjectUnknown) return;
    for (AudioObjectPropertyElement element = 0; element <= 2; element++) {
        AudioObjectPropertyAddress volume = booklandAddress(kAudioDevicePropertyVolumeScalar, element);
        AudioObjectPropertyAddress mute = booklandAddress(kAudioDevicePropertyMute, element);
        if (AudioObjectHasProperty(device, &volume))
            booklandVolumeListening[element] =
                AudioObjectAddPropertyListenerBlock(device, &volume,
                    dispatch_get_main_queue(), booklandLevelListener) == noErr;
        if (AudioObjectHasProperty(device, &mute))
            booklandMuteListening[element] =
                AudioObjectAddPropertyListenerBlock(device, &mute,
                    dispatch_get_main_queue(), booklandLevelListener) == noErr;
    }
    booklandLastVolume = booklandVolume();
    booklandLastMuted = booklandMuted();
}

void bookland_volume_hud_set(int enabled, int inlineMode, double closedWidth) {
    BOOL desired = enabled != 0;
    BOOL inlineDesired = inlineMode != 0;
    CGFloat width = isfinite(closedWidth) ? fmax(120, fmin(700, closedWidth)) : 255;
    if (desired == booklandVolumeEnabled && inlineDesired == booklandVolumeInline
        && fabs(width - booklandVolumeClosedWidth) < 0.5) return;
    BOOL listeningChanged = desired != booklandVolumeEnabled;
    booklandVolumeInline = inlineDesired;
    booklandVolumeClosedWidth = width;
    if (!listeningChanged) {
        if (booklandVolumeWindow.isVisible) {
            booklandLayoutVolume();
            booklandVolumePosition();
        }
        return;
    }
    booklandVolumeEnabled = desired;
    if (desired) {
        booklandDeviceListener = ^(UInt32 count, const AudioObjectPropertyAddress *addresses) {
            (void)count; (void)addresses;
            if (booklandVolumeEnabled) booklandBindOutputDevice();
        };
        AudioObjectPropertyAddress address = {
            kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain
        };
        AudioObjectAddPropertyListenerBlock(kAudioObjectSystemObject, &address,
            dispatch_get_main_queue(), booklandDeviceListener);
        booklandBindOutputDevice();
    } else {
        ++booklandHudGeneration;
        [booklandVolumeWindow orderOut:nil];
        booklandUnbindOutputDevice();
        AudioObjectPropertyAddress address = {
            kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain
        };
        AudioObjectRemovePropertyListenerBlock(kAudioObjectSystemObject, &address,
            dispatch_get_main_queue(), booklandDeviceListener);
        booklandDeviceListener = nil;
        booklandLastVolume = -1;
    }
}
