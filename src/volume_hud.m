#import <AppKit/AppKit.h>
#import <CoreAudio/CoreAudio.h>
#import <QuartzCore/QuartzCore.h>

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
static CAShapeLayer *booklandVolumeFill;
static CATextLayer *booklandVolumeText;
static Float32 booklandLastVolume = -1;
static BOOL booklandLastMuted;
static uint64_t booklandHudGeneration;

static AudioObjectPropertyAddress booklandAddress(AudioObjectPropertySelector selector,
                                                  AudioObjectPropertyElement element) {
    return (AudioObjectPropertyAddress) { selector, kAudioDevicePropertyScopeOutput, element };
}

static BOOL booklandReadScalar(AudioObjectID device, AudioObjectPropertyElement element,
                               Float32 *value) {
    AudioObjectPropertyAddress address = booklandAddress(kAudioDevicePropertyVolumeScalar, element);
    UInt32 size = sizeof(*value);
    return AudioObjectHasProperty(device, &address) &&
        AudioObjectGetPropertyData(device, &address, 0, NULL, &size, value) == noErr;
}

static Float32 booklandVolume(void) {
    if (booklandOutputDevice == kAudioObjectUnknown) return -1;
    Float32 value = 0;
    if (booklandReadScalar(booklandOutputDevice, kAudioObjectPropertyElementMain, &value)) {
        return fmaxf(0, fminf(1, value));
    }
    Float32 sum = 0;
    int count = 0;
    for (AudioObjectPropertyElement element = 1; element <= 2; element++) {
        if (booklandReadScalar(booklandOutputDevice, element, &value)) {
            sum += value;
            count++;
        }
    }
    return count ? fmaxf(0, fminf(1, sum / count)) : -1;
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

    CGMutablePathRef path = CGPathCreateMutable();
    CGPathMoveToPoint(path, NULL, 22, 16);
    CGPathAddLineToPoint(path, NULL, 208, 16);
    CAShapeLayer *track = [CAShapeLayer layer];
    track.path = path;
    track.fillColor = NSColor.clearColor.CGColor;
    track.strokeColor = [NSColor colorWithCalibratedWhite:1 alpha:0.23].CGColor;
    track.lineWidth = 5;
    track.lineCap = kCALineCapRound;
    [overlay.layer addSublayer:track];
    CAShapeLayer *fill = [CAShapeLayer layer];
    fill.path = path;
    fill.fillColor = NSColor.clearColor.CGColor;
    fill.strokeColor = [NSColor colorWithCalibratedRed:0.48 green:0.70 blue:1 alpha:1].CGColor;
    fill.lineWidth = 5;
    fill.lineCap = kCALineCapRound;
    fill.strokeEnd = 0;
    [overlay.layer addSublayer:fill];
    CGPathRelease(path);
    booklandVolumeWindow = window;
    booklandVolumeGlass = glass;
    booklandVolumeFill = fill;
    booklandVolumeText = text;
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
    CGFloat top = NSMaxY(screen.frame) - fmax(36, screen.safeAreaInsets.top) - 8;
    [booklandVolumeWindow setFrameOrigin:NSMakePoint(NSMidX(screen.frame) - 115, top - 56)];
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
    booklandVolumePosition();
    Float32 visible = muted ? 0 : volume;
    CGFloat old = ((CAShapeLayer *)booklandVolumeFill.presentationLayer).strokeEnd;
    if (!booklandVolumeFill.presentationLayer) old = booklandVolumeFill.strokeEnd;
    CABasicAnimation *bar = [CABasicAnimation animationWithKeyPath:@"strokeEnd"];
    bar.fromValue = @(old);
    bar.toValue = @(visible);
    bar.duration = 0.18;
    bar.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseOut];
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    booklandVolumeFill.strokeEnd = visible;
    [booklandVolumeFill addAnimation:bar forKey:@"bookland-volume-fill"];
    booklandVolumeText.string = muted ? @"Volume · Muted" :
        [NSString stringWithFormat:@"Volume · %d%%", (int)lroundf(volume * 100)];
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
    AudioObjectID device = kAudioObjectUnknown;
    UInt32 size = sizeof(device);
    AudioObjectPropertyAddress address = {
        kAudioHardwarePropertyDefaultOutputDevice,
        kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain
    };
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &address, 0, NULL,
                                   &size, &device) != noErr) device = kAudioObjectUnknown;
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

void bookland_volume_hud_set(int enabled) {
    BOOL desired = enabled != 0;
    if (desired == booklandVolumeEnabled) return;
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
