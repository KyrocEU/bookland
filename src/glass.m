#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#import <QuickLookThumbnailing/QuickLookThumbnailing.h>
#import <os/log.h>
#include <math.h>
#include <string.h>

// Keep winit's content view untouched: it owns keyboard and IME state.
// A non-interactive child window carries native glass behind its transparent pixels.
static NSGlassEffectView *nookletGlass[8];
static CGFloat nookletGlassTintAlpha[8];
static NSView *nookletContainer;
static NSWindow *nookletHostWindow;
static NSWindow *nookletGlassWindow;
static NSView *nookletMorphView;
static CAShapeLayer *nookletMorphLayer;
static BOOL nookletMorphAnimating;
static NSWindow *nookletProgressWindow;
static CAShapeLayer *nookletProgressTrack;
static CAShapeLayer *nookletProgressArc;
static int nookletProgressStyle = -1;
static CGFloat booklandProgressSpinAngle;
static NSWindow *booklandActivityWindow;
static NSGlassEffectView *booklandActivityGlass;
static NSTextField *booklandActivityLabel;
static unsigned long long booklandActivityGeneration;
static BOOL booklandActivityDismissing;
static NSWindow *booklandConversionWindow;
static NSGlassEffectView *booklandConversionGlass;
static NSImageView *booklandConversionPreview;
static NSTextField *booklandConversionName;
static NSTextField *booklandConversionStatus;
static NSURL *booklandConversionURL;
static QLThumbnailGenerationRequest *booklandConversionRequest;
static unsigned long long booklandConversionGeneration;
static BOOL booklandConversionDismissing;
static id booklandScrollMonitor;
static double booklandScrollDeltaX;
static double booklandScrollDeltaY;
static BOOL booklandScrollPending;
static NSTrackingArea *booklandHoverTracking;
static NSView *booklandHoverView;
static void *booklandHoverContext;
static void (*booklandHoverWake)(void *);
static CFAbsoluteTime booklandLastHoverWake;
static NSPoint booklandHoverScreenPoint;
static CFAbsoluteTime booklandHoverScreenPointAt;
// -1: no event yet; 0: exited; 1: inside the host view.
static int booklandHoverPointerState = -1;
static CADisplayLink *booklandAnimationLink;
static id booklandAnimationObserver;
static void *booklandAnimationContext;
static void (*booklandAnimationWake)(void *);

@interface BooklandAnimationObserver : NSObject
@end

@implementation BooklandAnimationObserver
- (void)displayFrame:(CADisplayLink *)link {
    (void)link;
    if (booklandAnimationWake) booklandAnimationWake(booklandAnimationContext);
}
@end

// The link tracks the host window's display. Keep it paused outside the short
// open/close morph so the closed island has no continuous animation wakeups.
int nooklet_animation_clock_set(int enabled, void *context, void (*wake)(void *)) {
    if (!enabled) {
        if (booklandAnimationLink) booklandAnimationLink.paused = YES;
        booklandAnimationContext = NULL;
        booklandAnimationWake = NULL;
        return 1;
    }
    if (!nookletHostWindow) return 0;
    if (!booklandAnimationLink) {
        booklandAnimationObserver = [BooklandAnimationObserver new];
        booklandAnimationLink = [nookletHostWindow
            displayLinkWithTarget:booklandAnimationObserver
                       selector:@selector(displayFrame:)];
        if (!booklandAnimationLink) {
            booklandAnimationObserver = nil;
            return 0;
        }
        booklandAnimationLink.preferredFrameRateRange = CAFrameRateRangeMake(60, 60, 60);
        booklandAnimationLink.paused = YES;
        [booklandAnimationLink addToRunLoop:NSRunLoop.mainRunLoop
                                   forMode:NSRunLoopCommonModes];
    }
    booklandAnimationContext = context;
    booklandAnimationWake = wake;
    booklandAnimationLink.paused = NO;
    return 1;
}

void nooklet_animation_clock_rate(int fps) {
    if (!booklandAnimationLink) return;
    static int currentRate = 60;
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    int maximum = screen ? (int)screen.maximumFramesPerSecond : 60;
    int rate = fps <= 30 ? 30 : (fps >= 120 && maximum >= 120 ? 120 : 60);
    if (rate == currentRate) return;
    // Allow the system to use 60 Hz on ProMotion displays when its power or
    // thermal policy prefers it. The link is paused once the morph settles.
    booklandAnimationLink.preferredFrameRateRange = CAFrameRateRangeMake(
        rate == 120 ? 60 : rate, rate, rate);
    currentRate = rate;
}

int nooklet_reduce_motion(void) {
    return NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion ? 1 : 0;
}

int nooklet_physical_notch(double *width, double *height) {
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    if (!screen) return 0;
    NSRect left = screen.auxiliaryTopLeftArea;
    NSRect right = screen.auxiliaryTopRightArea;
    double gap = NSMinX(right) - NSMaxX(left);
    double top = screen.safeAreaInsets.top;
    if (NSIsEmptyRect(left) || NSIsEmptyRect(right) || gap <= 0 || top <= 0) return 0;
    if (width) *width = gap;
    if (height) *height = top;
    return 1;
}

int bookland_scroll_take(double *deltaX, double *deltaY) {
    if (!booklandScrollPending) return 0;
    if (deltaX) *deltaX = booklandScrollDeltaX;
    if (deltaY) *deltaY = booklandScrollDeltaY;
    booklandScrollDeltaX = 0;
    booklandScrollDeltaY = 0;
    booklandScrollPending = NO;
    return 1;
}

void bookland_haptic_feedback(void) {
    [[NSHapticFeedbackManager defaultPerformer]
        performFeedbackPattern:NSHapticFeedbackPatternAlignment
        performanceTime:NSHapticFeedbackPerformanceTimeNow];
}

@interface BooklandHoverObserver : NSObject
@end

@implementation BooklandHoverObserver
- (void)mouseEntered:(NSEvent *)event {
    booklandHoverScreenPoint = [event.window convertPointToScreen:event.locationInWindow];
    booklandHoverScreenPointAt = CFAbsoluteTimeGetCurrent();
    booklandHoverPointerState = 1;
    booklandLastHoverWake = booklandHoverScreenPointAt;
    if (booklandHoverWake) booklandHoverWake(booklandHoverContext);
}
- (void)mouseExited:(NSEvent *)event {
    (void)event;
    booklandHoverPointerState = 0;
    booklandLastHoverWake = CFAbsoluteTimeGetCurrent();
    if (booklandHoverWake) booklandHoverWake(booklandHoverContext);
}
- (void)mouseMoved:(NSEvent *)event {
    booklandHoverScreenPoint = [event.window convertPointToScreen:event.locationInWindow];
    booklandHoverScreenPointAt = CFAbsoluteTimeGetCurrent();
    booklandHoverPointerState = 1;
    CFAbsoluteTime now = booklandHoverScreenPointAt;
    if (now - booklandLastHoverWake < 0.016) return;
    booklandLastHoverWake = now;
    if (booklandHoverWake) booklandHoverWake(booklandHoverContext);
}
@end

static BooklandHoverObserver *booklandHoverObserver;

static NSGlassEffectView *nookletPrepareGlassView(int slot) {
    if (!nookletContainer || slot < 0 || slot >= 8) return nil;
    NSGlassEffectView *view = nookletGlass[slot];
    if (view) return view;
    view = [[NSGlassEffectView alloc] initWithFrame:NSZeroRect];
    view.autoresizingMask = NSViewNotSizable;
    view.style = NSGlassEffectViewStyleRegular;
    view.hidden = YES;
    view.alphaValue = 0;
    [nookletContainer addSubview:view positioned:NSWindowBelow
                     relativeTo:nookletContainer.subviews.lastObject];
    nookletGlass[slot] = view;
    return view;
}

int nooklet_hover_pointer_screen(double *x, double *y) {
    // A tracking area can remain "inside" after a synthetic pointer jump
    // within the transparent host. Trust the event point only briefly; the
    // global mouse location is authoritative once the pointer is stationary.
    if (booklandHoverPointerState == 1 && x && y &&
        CFAbsoluteTimeGetCurrent() - booklandHoverScreenPointAt <= 0.25) {
        *x = booklandHoverScreenPoint.x;
        *y = booklandHoverScreenPoint.y;
        return 1;
    }
    return 0;
}

void nooklet_hover_monitor_set(int enabled, void *context, void (*wake)(void *)) {
    if (!enabled) {
        if (booklandHoverTracking && booklandHoverView) {
            [booklandHoverView removeTrackingArea:booklandHoverTracking];
        }
        booklandHoverTracking = nil;
        booklandHoverView = nil;
        booklandHoverObserver = nil;
        booklandHoverContext = NULL;
        booklandHoverWake = NULL;
        booklandHoverPointerState = -1;
        booklandHoverScreenPointAt = 0;
        return;
    }
    if (!nookletHostWindow || !nookletHostWindow.contentView) return;
    booklandHoverContext = context;
    booklandHoverWake = wake;
    if (booklandHoverTracking) return;
    booklandHoverPointerState = -1;
    booklandHoverView = nookletHostWindow.contentView;
    booklandHoverObserver = [BooklandHoverObserver new];
    booklandHoverTracking = [[NSTrackingArea alloc] initWithRect:NSZeroRect
        options:NSTrackingMouseEnteredAndExited | NSTrackingMouseMoved |
                NSTrackingActiveAlways | NSTrackingInVisibleRect
        owner:booklandHoverObserver userInfo:nil];
    [booklandHoverView addTrackingArea:booklandHoverTracking];
    nookletHostWindow.acceptsMouseMovedEvents = YES;
}

int nooklet_install_glass(void *rawView) {
    if (nookletContainer) return 1;
    if (!rawView) return 0;
    NSView *inputView = (__bridge NSView *)rawView;
    NSWindow *window = inputView.window;
    if (!window) return 0;
    // Keep the notch overlay present across Spaces and over full-screen apps.
    window.collectionBehavior |= NSWindowCollectionBehaviorCanJoinAllSpaces |
                                 NSWindowCollectionBehaviorFullScreenAuxiliary;

    NSWindow *background = [[NSWindow alloc]
        initWithContentRect:window.frame
                  styleMask:NSWindowStyleMaskBorderless
                    backing:NSBackingStoreBuffered
                      defer:NO];
    background.opaque = NO;
    background.backgroundColor = NSColor.clearColor;
    background.hasShadow = NO;
    background.ignoresMouseEvents = YES;
    background.collectionBehavior = window.collectionBehavior;
    NSGlassEffectContainerView *group = [[NSGlassEffectContainerView alloc]
        initWithFrame:background.contentView.bounds];
    group.wantsLayer = YES;
    group.layer.backgroundColor = NSColor.clearColor.CGColor;
    group.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    group.spacing = 0;
    NSView *container = [[NSView alloc] initWithFrame:group.bounds];
    container.wantsLayer = YES;
    container.layer.backgroundColor = NSColor.clearColor.CGColor;
    container.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    group.contentView = container;
    background.contentView = group;
    window.opaque = NO;
    window.backgroundColor = NSColor.clearColor;
    window.hasShadow = NO;
    // The island is anchored to the screen edge. A drag across transparent
    // margins must not turn the borderless host into a movable window.
    window.movable = NO;
    window.movableByWindowBackground = NO;
    [window addChildWindow:background ordered:NSWindowBelow];
    [background orderFront:nil];
    nookletContainer = container;
    nookletHostWindow = window;
    nookletGlassWindow = background;
    // Prepare the island and its floating menu/card glass before the first
    // hover. Hidden views do not render; this keeps allocation off the morph.
    for (int slot = 0; slot <= 5; slot++) nookletPrepareGlassView(slot);
    // Keep the moving black silhouette in a native Core Animation layer.
    // Only this path is visible during the morph; the final workspace stays
    // in the existing adaptive glass view.
    nookletMorphView = [[NSView alloc] initWithFrame:container.bounds];
    nookletMorphView.wantsLayer = YES;
    nookletMorphView.layer.backgroundColor = NSColor.clearColor.CGColor;
    nookletMorphView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    nookletMorphView.hidden = YES;
    nookletMorphLayer = [CAShapeLayer layer];
    nookletMorphLayer.fillColor = NSColor.blackColor.CGColor;
    [nookletMorphView.layer addSublayer:nookletMorphLayer];
    [container addSubview:nookletMorphView positioned:NSWindowAbove relativeTo:nil];
    if (!booklandScrollMonitor) {
        // NotchNook's local scroll monitor ignores momentum events. Keeping
        // those out of the gesture accumulator avoids a second media action
        // after the user's fingers have left the trackpad.
        booklandScrollMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskScrollWheel
            handler:^NSEvent *(NSEvent *event) {
                if (event.window != nookletHostWindow ||
                    event.momentumPhase != NSEventPhaseNone ||
                    event.phase == NSEventPhaseBegan) return event;
                booklandScrollDeltaX += event.scrollingDeltaX;
                booklandScrollDeltaY += event.scrollingDeltaY;
                booklandScrollPending = YES;
                return event;
            }];
    }
    return 1;
}

size_t nooklet_displays_copy(char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    NSMutableArray *rows = [NSMutableArray array];
    for (NSScreen *screen in NSScreen.screens) {
        NSNumber *number = screen.deviceDescription[@"NSScreenNumber"];
        if (!number) continue;
        NSRect left = screen.auxiliaryTopLeftArea;
        NSRect right = screen.auxiliaryTopRightArea;
        BOOL notch = !NSIsEmptyRect(left) && !NSIsEmptyRect(right)
            && NSMinX(right) > NSMaxX(left) && screen.safeAreaInsets.top > 0;
        BOOL isMain = number.unsignedIntValue == CGMainDisplayID();
        [rows addObject:@{
            @"id": number,
            @"name": screen.localizedName ?: @"Display",
            @"main": @(isMain),
            @"notch": @(notch)
        }];
    }
    NSData *json = [NSJSONSerialization dataWithJSONObject:rows options:0 error:nil];
    if (!json || json.length >= capacity) { buffer[0] = '\0'; return 0; }
    memcpy(buffer, json.bytes, json.length);
    buffer[json.length] = '\0';
    return json.length;
}

void nooklet_anchor_host_to_screen_top(int enabled, unsigned int displayID) {
    if (!nookletHostWindow || !enabled) return;
    // Winit positions borderless windows below the menu bar on this Mac even
    // when asked for y=0. The real notch is in the menu-bar band.
    nookletHostWindow.level = NSStatusWindowLevel;
    NSScreen *screen = nil;
    if (displayID != 0) {
        for (NSScreen *candidate in NSScreen.screens) {
            if ([candidate.deviceDescription[@"NSScreenNumber"] unsignedIntValue]
                == displayID) { screen = candidate; break; }
        }
    }
    if (!screen) screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    if (!screen) return;
    NSRect frame = nookletHostWindow.frame;
    NSRect display = screen.frame;
    NSPoint origin = NSMakePoint(NSMidX(display) - frame.size.width / 2.0,
                                 NSMaxY(display) - frame.size.height);
    if (fabs(frame.origin.x - origin.x) > 0.5 ||
        fabs(frame.origin.y - origin.y) > 0.5) {
        [nookletHostWindow setFrameOrigin:origin];
    }
}

void nooklet_set_glass(int slot, double x, double y, double width, double height,
                       double radius, double level, double opacity, int enabled) {
    if (!nookletContainer || slot < 0 || slot >= 8) return;
    if (!NSEqualRects(nookletGlassWindow.frame, nookletHostWindow.frame)) {
        [nookletGlassWindow setFrame:nookletHostWindow.frame display:NO];
    }
    NSGlassEffectView *view = nookletGlass[slot];
    if (!enabled || width <= 0 || height <= 0) {
        if (view && !view.hidden) view.hidden = YES;
        return;
    }
    view = nookletPrepareGlassView(slot);
    double flippedY = nookletContainer.bounds.size.height - y - height;
    NSRect frame = NSMakeRect(x, flippedY, width, height);
    CGFloat alpha = fmax(0.0, fmin(1.0, opacity));
    CGFloat tintAlpha = fmax(0.03, fmin(0.12, level * 0.08));
    BOOL frameChanged = !NSEqualRects(view.frame, frame);
    BOOL radiusChanged = fabs(view.cornerRadius - radius) > 0.001;
    BOOL tintChanged = fabs(nookletGlassTintAlpha[slot] - tintAlpha) > 0.0001;
    BOOL alphaChanged = fabs(view.alphaValue - alpha) > 0.001;
    if (!view.hidden && !frameChanged && !radiusChanged && !tintChanged
        && !alphaChanged) return;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    if (frameChanged) view.frame = frame;
    if (radiusChanged) view.cornerRadius = radius;
    if (tintChanged) {
        view.tintColor = [NSColor colorWithCalibratedRed:0.46 green:0.50 blue:0.58
                                                  alpha:tintAlpha];
        nookletGlassTintAlpha[slot] = tintAlpha;
    }
    if (alphaChanged) view.alphaValue = alpha;
    view.hidden = NO;
    [CATransaction commit];
}

typedef struct {
    double x, y, width, height, top_radius, bottom_radius, opacity;
} BooklandMorphFrame;

static CGPathRef booklandMorphPath(BooklandMorphFrame frame, CGFloat containerHeight) {
    CGFloat x = frame.x, w = frame.width, h = frame.height;
    CGFloat y = containerHeight - frame.y - h;
    CGFloat top = fmax(0, fmin(frame.top_radius, fmin(w, h) * 0.5));
    CGFloat bottom = fmax(0, fmin(frame.bottom_radius, fmin(w, h) * 0.5));
    CGMutablePathRef path = CGPathCreateMutable();
    CGPathMoveToPoint(path, NULL, x + top, y + h);
    CGPathAddLineToPoint(path, NULL, x + w - top, y + h);
    CGPathAddQuadCurveToPoint(path, NULL, x + w, y + h, x + w, y + h - top);
    CGPathAddLineToPoint(path, NULL, x + w, y + bottom);
    CGPathAddQuadCurveToPoint(path, NULL, x + w, y, x + w - bottom, y);
    CGPathAddLineToPoint(path, NULL, x + bottom, y);
    CGPathAddQuadCurveToPoint(path, NULL, x, y, x, y + bottom);
    CGPathAddLineToPoint(path, NULL, x, y + h - top);
    CGPathAddQuadCurveToPoint(path, NULL, x, y + h, x + top, y + h);
    CGPathCloseSubpath(path);
    return path;
}

int nooklet_morph_shell_animate(const BooklandMorphFrame *frames, size_t count,
                               double duration, double minimumWidth,
                               double minimumHeight) {
    if (!nookletMorphView || !nookletMorphLayer || !frames || count < 2) return 0;
    if (!NSEqualRects(nookletGlassWindow.frame, nookletHostWindow.frame)) {
        [nookletGlassWindow setFrame:nookletHostWindow.frame display:NO];
    }
    nookletMorphView.frame = nookletContainer.bounds;
    NSSize size = nookletMorphView.bounds.size;
    if (size.width < minimumWidth || size.height < minimumHeight) return 0;
    CAShapeLayer *presented = (CAShapeLayer *)nookletMorphLayer.presentationLayer;
    CGPathRef previous = nookletMorphAnimating && presented.path
        ? CGPathCreateCopy(presented.path) : NULL;
    float previousOpacity = nookletMorphAnimating ? presented.opacity : 0;
    NSMutableArray *paths = [NSMutableArray arrayWithCapacity:count];
    NSMutableArray<NSNumber *> *opacities = [NSMutableArray arrayWithCapacity:count];
    for (size_t index = 0; index < count; index++) {
        CGPathRef path = index == 0 && previous ? CGPathCreateCopy(previous)
            : booklandMorphPath(frames[index], size.height);
        [paths addObject:(__bridge id)path];
        [opacities addObject:@(index == 0 && previous
            ? previousOpacity : frames[index].opacity)];
        CGPathRelease(path);
    }
    if (previous) CGPathRelease(previous);
    CAKeyframeAnimation *shape = [CAKeyframeAnimation animationWithKeyPath:@"path"];
    shape.values = paths;
    shape.calculationMode = kCAAnimationLinear;
    shape.duration = duration;
    CAKeyframeAnimation *fade = [CAKeyframeAnimation animationWithKeyPath:@"opacity"];
    fade.values = opacities;
    fade.calculationMode = kCAAnimationLinear;
    fade.duration = duration;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    [nookletMorphLayer removeAllAnimations];
    nookletMorphLayer.frame = nookletMorphView.bounds;
    nookletMorphLayer.path = (__bridge CGPathRef)paths.lastObject;
    nookletMorphLayer.opacity = (float)frames[count - 1].opacity;
    nookletMorphView.hidden = NO;
    [nookletMorphLayer addAnimation:shape forKey:@"booklandMorphPath"];
    [nookletMorphLayer addAnimation:fade forKey:@"booklandMorphOpacity"];
    [CATransaction commit];
    nookletMorphAnimating = YES;
    os_log_debug(OS_LOG_DEFAULT, "Bookland native morph started (%zu frames)", count);
    return 1;
}

void nooklet_morph_shell_set(int enabled, double x, double y, double width,
                            double height, double topRadius, double bottomRadius,
                            double opacity) {
    if (!nookletMorphView || !nookletMorphLayer) return;
    if (!enabled) {
        [nookletMorphLayer removeAllAnimations];
        nookletMorphAnimating = NO;
        nookletMorphView.hidden = YES;
        return;
    }
    if (nookletMorphAnimating) {
        // Once Core Animation removes the completed keyframes, resume
        // following the live island geometry. Otherwise a workspace resize
        // can leave the shell at the dimensions of its last hover morph.
        if ([nookletMorphLayer animationForKey:@"booklandMorphPath"] ||
            [nookletMorphLayer animationForKey:@"booklandMorphOpacity"]) {
            nookletMorphView.hidden = NO;
            return;
        }
        nookletMorphAnimating = NO;
    }
    if (width <= 0 || height <= 0 || opacity <= 0) {
        nookletMorphView.hidden = YES;
        return;
    }
    if (!NSEqualRects(nookletGlassWindow.frame, nookletHostWindow.frame)) {
        [nookletGlassWindow setFrame:nookletHostWindow.frame display:NO];
    }
    CGFloat w = width, h = height;
    CGFloat top = fmax(0, fmin(topRadius, fmin(w, h) * 0.5));
    CGFloat bottom = fmax(0, fmin(bottomRadius, fmin(w, h) * 0.5));
    CGMutablePathRef path = CGPathCreateMutable();
    CGPathMoveToPoint(path, NULL, top, h);
    CGPathAddLineToPoint(path, NULL, w - top, h);
    CGPathAddQuadCurveToPoint(path, NULL, w, h, w, h - top);
    CGPathAddLineToPoint(path, NULL, w, bottom);
    CGPathAddQuadCurveToPoint(path, NULL, w, 0, w - bottom, 0);
    CGPathAddLineToPoint(path, NULL, bottom, 0);
    CGPathAddQuadCurveToPoint(path, NULL, 0, 0, 0, bottom);
    CGPathAddLineToPoint(path, NULL, 0, h - top);
    CGPathAddQuadCurveToPoint(path, NULL, 0, h, top, h);
    CGPathCloseSubpath(path);
    CGFloat flippedY = nookletMorphView.bounds.size.height - y - h;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    nookletMorphLayer.frame = CGRectMake(x, flippedY, w, h);
    nookletMorphLayer.path = path;
    nookletMorphLayer.opacity = fmax(0, fmin(1, opacity));
    nookletMorphView.hidden = NO;
    [CATransaction commit];
    CGPathRelease(path);
}

static void booklandConfigureProgressRing(int usageStyle) {
    if (!nookletProgressTrack || !nookletProgressArc
        || nookletProgressStyle == usageStyle) return;
    CGFloat radius = usageStyle ? 33.0 : 7.0;
    CGFloat size = usageStyle ? 72.0 : 28.0;
    CGFloat midpoint = size * 0.5;
    CGMutablePathRef track = CGPathCreateMutable();
    CGPathAddEllipseInRect(track, NULL, CGRectMake(midpoint - radius,
        midpoint - radius, radius * 2.0, radius * 2.0));
    CGMutablePathRef arc = CGPathCreateMutable();
    CGPathAddArc(arc, NULL, midpoint, midpoint, radius, 0,
        usageStyle ? -2.0 * M_PI * 0.39 : -M_PI_2, true);
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    nookletProgressTrack.bounds = CGRectMake(0, 0, size, size);
    nookletProgressTrack.path = track;
    nookletProgressTrack.lineWidth = usageStyle ? 3.0 : 1.5;
    nookletProgressTrack.strokeColor = usageStyle
        ? [NSColor colorWithCalibratedRed:0.106 green:0.188 blue:0.322 alpha:1.0].CGColor
        : [NSColor colorWithCalibratedRed:0.204 green:0.365 blue:0.612 alpha:0.43].CGColor;
    nookletProgressArc.bounds = CGRectMake(0, 0, size, size);
    nookletProgressArc.path = arc;
    nookletProgressArc.lineWidth = usageStyle ? 3.0 : 2.0;
    nookletProgressArc.strokeColor = usageStyle
        ? [NSColor colorWithCalibratedRed:0.341 green:0.569 blue:0.973 alpha:1.0].CGColor
        : [NSColor colorWithCalibratedRed:0.294 green:0.545 blue:0.984 alpha:1.0].CGColor;
    [CATransaction commit];
    CGPathRelease(track);
    CGPathRelease(arc);
    nookletProgressStyle = usageStyle;
}

static void booklandPauseProgressSpin(void) {
    if (![nookletProgressArc animationForKey:@"bookland-progress-spin"]) return;
    CALayer *presented = nookletProgressArc.presentationLayer;
    if (presented) {
        booklandProgressSpinAngle = atan2(presented.transform.m12,
                                         presented.transform.m11);
    }
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    [nookletProgressArc removeAnimationForKey:@"bookland-progress-spin"];
    nookletProgressArc.transform = CATransform3DMakeRotation(
        booklandProgressSpinAngle, 0, 0, 1);
    [CATransaction commit];
}

void nooklet_progress_ring_set(int enabled, double x, double y, int usageStyle) {
    if (!nookletHostWindow) return;
    if (!enabled) {
        booklandPauseProgressSpin();
        if (nookletProgressWindow.isVisible) [nookletProgressWindow orderOut:nil];
        return;
    }
    if (!nookletProgressWindow) {
        NSWindow *window = [[NSWindow alloc]
            initWithContentRect:nookletHostWindow.frame
                      styleMask:NSWindowStyleMaskBorderless
                        backing:NSBackingStoreBuffered defer:NO];
        window.opaque = NO;
        window.backgroundColor = NSColor.clearColor;
        window.hasShadow = NO;
        window.ignoresMouseEvents = YES;
        window.collectionBehavior = nookletHostWindow.collectionBehavior;
        NSView *content = [[NSView alloc] initWithFrame:window.contentView.bounds];
        content.wantsLayer = YES;
        content.layer.backgroundColor = NSColor.clearColor.CGColor;
        content.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
        window.contentView = content;
        nookletProgressTrack = [CAShapeLayer layer];
        nookletProgressTrack.fillColor = NSColor.clearColor.CGColor;
        [content.layer addSublayer:nookletProgressTrack];
        nookletProgressArc = [CAShapeLayer layer];
        nookletProgressArc.fillColor = NSColor.clearColor.CGColor;
        nookletProgressArc.lineCap = kCALineCapRound;
        [content.layer addSublayer:nookletProgressArc];
        [nookletHostWindow addChildWindow:window ordered:NSWindowAbove];
        nookletProgressWindow = window;
    }
    booklandConfigureProgressRing(usageStyle != 0);
    if (nooklet_reduce_motion()) {
        booklandPauseProgressSpin();
    } else if (![nookletProgressArc animationForKey:@"bookland-progress-spin"]) {
        CABasicAnimation *spin = [CABasicAnimation animationWithKeyPath:@"transform.rotation.z"];
        spin.fromValue = @(booklandProgressSpinAngle);
        spin.toValue = @(booklandProgressSpinAngle - 2.0 * M_PI);
        spin.duration = 3.0;
        spin.repeatCount = HUGE_VALF;
        spin.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionLinear];
        [nookletProgressArc addAnimation:spin forKey:@"bookland-progress-spin"];
    }
    if (!NSEqualRects(nookletProgressWindow.frame, nookletHostWindow.frame)) {
        [nookletProgressWindow setFrame:nookletHostWindow.frame display:NO];
    }
    CGPoint center = CGPointMake(x, nookletProgressWindow.contentView.bounds.size.height - y);
    if (!CGPointEqualToPoint(nookletProgressTrack.position, center)
        || !CGPointEqualToPoint(nookletProgressArc.position, center)) {
        [CATransaction begin];
        [CATransaction setDisableActions:YES];
        nookletProgressTrack.position = center;
        nookletProgressArc.position = center;
        [CATransaction commit];
    }
    if (!nookletProgressWindow.isVisible) [nookletProgressWindow orderFront:nil];
}

// A short-lived, non-interactive activity capsule sits beside the camera.
// Its entry and exit run entirely in Core Animation; there is no idle tick.
static void booklandPrepareActivityWindow(void) {
    if (booklandActivityWindow || !nookletHostWindow) return;
    NSWindow *window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, 120, 30)
                  styleMask:NSWindowStyleMaskBorderless
                    backing:NSBackingStoreBuffered defer:NO];
    window.opaque = NO;
    window.backgroundColor = NSColor.clearColor;
    window.hasShadow = YES;
    window.ignoresMouseEvents = YES;
    window.collectionBehavior = nookletHostWindow.collectionBehavior;
    window.level = NSStatusWindowLevel;
    NSGlassEffectView *glass = [[NSGlassEffectView alloc]
        initWithFrame:NSMakeRect(0, 0, 120, 30)];
    glass.wantsLayer = YES;
    glass.style = NSGlassEffectViewStyleRegular;
    glass.cornerRadius = 15;
    // Match the open island's regular glass tint. The material still samples
    // the wallpaper beneath this separate window.
    glass.tintColor = [NSColor colorWithCalibratedRed:0.46 green:0.50
                                                 blue:0.58 alpha:0.08];
    NSTextField *label = [[NSTextField alloc] initWithFrame:NSMakeRect(12, 6, 96, 18)];
    label.editable = NO;
    label.selectable = NO;
    label.bezeled = NO;
    label.drawsBackground = NO;
    label.lineBreakMode = NSLineBreakByTruncatingTail;
    label.alignment = NSTextAlignmentCenter;
    label.textColor = NSColor.whiteColor;
    label.font = [NSFont systemFontOfSize:12 weight:NSFontWeightMedium];
    NSShadow *textShadow = [NSShadow new];
    textShadow.shadowColor = [NSColor colorWithWhite:0 alpha:0.70];
    textShadow.shadowBlurRadius = 3;
    textShadow.shadowOffset = NSMakeSize(0, -1);
    label.shadow = textShadow;
    [glass addSubview:label];
    window.contentView = glass;
    [nookletHostWindow addChildWindow:window ordered:NSWindowAbove];
    booklandActivityWindow = window;
    booklandActivityGlass = glass;
    booklandActivityLabel = label;
}

void bookland_activity_bubble_hide(void) {
    if (!booklandActivityWindow.isVisible || booklandActivityDismissing) return;
    booklandActivityDismissing = YES;
    unsigned long long generation = booklandActivityGeneration;
    CALayer *layer = booklandActivityGlass.layer;
    if (nooklet_reduce_motion()) {
        [booklandActivityWindow orderOut:nil];
        booklandActivityDismissing = NO;
        return;
    }
    CALayer *presented = (CALayer *)layer.presentationLayer;
    CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
    fade.fromValue = @(presented ? presented.opacity : layer.opacity);
    fade.toValue = @0;
    fade.duration = 0.19;
    fade.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseIn];
    CABasicAnimation *slide = [CABasicAnimation
        animationWithKeyPath:@"transform.translation.x"];
    slide.fromValue = @(presented ? presented.transform.m41 : 0);
    slide.toValue = @(-10);
    slide.duration = fade.duration;
    slide.timingFunction = fade.timingFunction;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    layer.opacity = 0;
    [layer addAnimation:fade forKey:@"booklandActivityFade"];
    [layer addAnimation:slide forKey:@"booklandActivitySlide"];
    [CATransaction commit];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.20 * NSEC_PER_SEC)),
        dispatch_get_main_queue(), ^{
            if (generation != booklandActivityGeneration) return;
            [booklandActivityWindow orderOut:nil];
            booklandActivityDismissing = NO;
        });
}

void bookland_activity_bubble_show(const char *utf8, double closedWidth) {
    if (!utf8 || !*utf8 || !nookletHostWindow) return;
    NSString *message = [NSString stringWithUTF8String:utf8];
    if (!message.length) return;
    booklandPrepareActivityWindow();
    if (!booklandActivityWindow) return;
    ++booklandActivityGeneration;
    booklandActivityDismissing = NO;
    booklandActivityLabel.stringValue = message;
    CGFloat labelWidth = [message sizeWithAttributes:@{
        NSFontAttributeName: booklandActivityLabel.font
    }].width;
    CGFloat width = fmin(260, fmax(106, ceil(labelWidth) + 28));
    NSRect frame = NSMakeRect(0, 0, width, 30);
    booklandActivityGlass.frame = frame;
    booklandActivityLabel.frame = NSMakeRect(12, 6, width - 24, 18);
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    if (!screen) return;
    CGFloat x = NSMidX(screen.frame) + fmax(90, closedWidth * 0.5) + 12;
    x = fmin(x, NSMaxX(screen.frame) - width - 8);
    CGFloat top = NSMaxY(screen.frame) - fmax(32, screen.safeAreaInsets.top) - 8;
    [booklandActivityWindow setFrame:NSMakeRect(x, top - 30, width, 30) display:NO];
    CALayer *layer = booklandActivityGlass.layer;
    [layer removeAllAnimations];
    if (!booklandActivityWindow.isVisible) [booklandActivityWindow orderFront:nil];
    if (nooklet_reduce_motion()) {
        layer.opacity = 1;
    } else {
        CASpringAnimation *enter = [CASpringAnimation
            animationWithKeyPath:@"transform.translation.x"];
        enter.fromValue = @(-15);
        enter.toValue = @0;
        enter.mass = 1;
        enter.stiffness = 140;
        enter.damping = 18;
        enter.initialVelocity = 0;
        enter.duration = fmin(0.55, enter.settlingDuration);
        CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
        fade.fromValue = @0;
        fade.toValue = @1;
        fade.duration = 0.13;
        [CATransaction begin];
        [CATransaction setDisableActions:YES];
        layer.opacity = 1;
        [layer addAnimation:enter forKey:@"booklandActivityEnter"];
        [layer addAnimation:fade forKey:@"booklandActivityFade"];
        [CATransaction commit];
    }
    unsigned long long generation = booklandActivityGeneration;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(3.1 * NSEC_PER_SEC)),
        dispatch_get_main_queue(), ^{
            if (generation == booklandActivityGeneration) bookland_activity_bubble_hide();
        });
}

void bookland_conversion_card_hide(void);

@interface BooklandConversionCardAction : NSObject
- (void)showInFinder:(id)sender;
@end

@implementation BooklandConversionCardAction
- (void)showInFinder:(id)sender {
    (void)sender;
    if (!booklandConversionURL) return;
    [NSWorkspace.sharedWorkspace activateFileViewerSelectingURLs:@[booklandConversionURL]];
    bookland_conversion_card_hide();
}
@end

static id booklandConversionAction;

static NSTextField *booklandConversionText(NSRect frame, CGFloat size,
                                           NSFontWeight weight, NSColor *color) {
    NSTextField *label = [[NSTextField alloc] initWithFrame:frame];
    label.editable = NO;
    label.selectable = NO;
    label.bezeled = NO;
    label.drawsBackground = NO;
    label.lineBreakMode = NSLineBreakByTruncatingMiddle;
    label.textColor = color;
    label.font = [NSFont systemFontOfSize:size weight:weight];
    NSShadow *shadow = [NSShadow new];
    shadow.shadowColor = [NSColor colorWithWhite:0 alpha:0.7];
    shadow.shadowBlurRadius = 3;
    label.shadow = shadow;
    return label;
}

static void booklandPrepareConversionCard(void) {
    if (booklandConversionWindow || !nookletHostWindow) return;
    const CGFloat width = 336, height = 94;
    NSWindow *window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, width, height)
                  styleMask:NSWindowStyleMaskBorderless
                    backing:NSBackingStoreBuffered defer:NO];
    window.opaque = NO;
    window.backgroundColor = NSColor.clearColor;
    window.hasShadow = YES;
    window.ignoresMouseEvents = NO;
    window.collectionBehavior = nookletHostWindow.collectionBehavior;
    window.level = NSStatusWindowLevel;
    NSGlassEffectView *glass = [[NSGlassEffectView alloc]
        initWithFrame:NSMakeRect(0, 0, width, height)];
    glass.wantsLayer = YES;
    glass.style = NSGlassEffectViewStyleRegular;
    glass.cornerRadius = 20;
    glass.tintColor = [NSColor colorWithCalibratedRed:0.46 green:0.50
                                                 blue:0.58 alpha:0.08];
    NSImageView *preview = [[NSImageView alloc]
        initWithFrame:NSMakeRect(17, 19, 56, 56)];
    preview.imageScaling = NSImageScaleProportionallyUpOrDown;
    preview.wantsLayer = YES;
    preview.layer.cornerRadius = 9;
    preview.layer.masksToBounds = YES;
    [glass addSubview:preview];
    NSTextField *name = booklandConversionText(NSMakeRect(87, 54, 232, 20),
        13, NSFontWeightSemibold, NSColor.whiteColor);
    [glass addSubview:name];
    NSTextField *status = booklandConversionText(NSMakeRect(87, 34, 232, 17),
        11, NSFontWeightRegular,
        [NSColor colorWithCalibratedRed:0.82 green:0.89 blue:0.98 alpha:1]);
    status.stringValue = @"Converted in Pocket";
    [glass addSubview:status];
    NSTextField *hint = booklandConversionText(NSMakeRect(87, 14, 232, 17),
        11, NSFontWeightMedium,
        [NSColor colorWithCalibratedRed:0.62 green:0.79 blue:1 alpha:1]);
    hint.stringValue = @"Show in Finder";
    [glass addSubview:hint];
    booklandConversionAction = [BooklandConversionCardAction new];
    NSButton *button = [[NSButton alloc]
        initWithFrame:NSMakeRect(0, 0, width, height)];
    button.bordered = NO;
    button.transparent = YES;
    button.title = @"";
    button.target = booklandConversionAction;
    button.action = @selector(showInFinder:);
    [button setAccessibilityElement:YES];
    button.accessibilityLabel = @"Show converted file in Finder";
    [glass addSubview:button];
    window.contentView = glass;
    [nookletHostWindow addChildWindow:window ordered:NSWindowAbove];
    booklandConversionWindow = window;
    booklandConversionGlass = glass;
    booklandConversionPreview = preview;
    booklandConversionName = name;
    booklandConversionStatus = status;
}

void bookland_conversion_card_hide(void) {
    if (!booklandConversionWindow.isVisible || booklandConversionDismissing) return;
    booklandConversionDismissing = YES;
    ++booklandConversionGeneration;
    unsigned long long generation = booklandConversionGeneration;
    if (booklandConversionRequest) {
        [QLThumbnailGenerator.sharedGenerator cancelRequest:booklandConversionRequest];
        booklandConversionRequest = nil;
    }
    CALayer *layer = booklandConversionGlass.layer;
    if (nooklet_reduce_motion()) {
        [booklandConversionWindow orderOut:nil];
        booklandConversionDismissing = NO;
        return;
    }
    CALayer *presented = (CALayer *)layer.presentationLayer;
    CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
    fade.fromValue = @(presented ? presented.opacity : layer.opacity);
    fade.toValue = @0;
    fade.duration = 0.19;
    fade.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseIn];
    CABasicAnimation *slide = [CABasicAnimation
        animationWithKeyPath:@"transform.translation.y"];
    slide.fromValue = @(presented ? presented.transform.m42 : 0);
    slide.toValue = @12;
    slide.duration = fade.duration;
    slide.timingFunction = fade.timingFunction;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    layer.opacity = 0;
    [layer addAnimation:fade forKey:@"booklandConversionFade"];
    [layer addAnimation:slide forKey:@"booklandConversionSlide"];
    [CATransaction commit];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.20 * NSEC_PER_SEC)),
        dispatch_get_main_queue(), ^{
            if (generation != booklandConversionGeneration) return;
            [booklandConversionWindow orderOut:nil];
            booklandConversionDismissing = NO;
        });
}

static void booklandPositionConversionCard(double panelBottom, BOOL animated) {
    if (!booklandConversionWindow || !nookletHostWindow) return;
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    if (!screen) return;
    CGFloat width = booklandConversionWindow.frame.size.width;
    CGFloat height = booklandConversionWindow.frame.size.height;
    CGFloat inset = fmax(32, screen.safeAreaInsets.top) + 8;
    if (isfinite(panelBottom) && panelBottom > 0)
        inset = fmax(inset, panelBottom + 12);
    CGFloat y = fmax(NSMinY(screen.visibleFrame) + 8,
                     NSMaxY(screen.frame) - inset - height);
    NSRect frame = NSMakeRect(NSMidX(screen.frame) - width / 2, y, width, height);
    NSRect previous = booklandConversionWindow.frame;
    if (NSEqualRects(previous, frame)) return;
    [booklandConversionWindow setFrame:frame display:NO];
    if (animated && booklandConversionWindow.isVisible && !nooklet_reduce_motion()
        && fabs(previous.origin.y - y) > 4) {
        CABasicAnimation *move = [CABasicAnimation
            animationWithKeyPath:@"transform.translation.y"];
        move.fromValue = @(previous.origin.y - y);
        move.toValue = @0;
        move.duration = 0.22;
        move.timingFunction = [CAMediaTimingFunction
            functionWithName:kCAMediaTimingFunctionEaseOut];
        [booklandConversionGlass.layer addAnimation:move forKey:@"booklandConversionMove"];
    }
}

void bookland_result_card_position(double panelBottom) {
    if (booklandConversionWindow.isVisible)
        booklandPositionConversionCard(panelBottom, YES);
}

void bookland_result_card_show(const char *utf8Path,
                               const char *utf8Subtitle, double seconds,
                               double panelBottom) {
    if (!utf8Path || !*utf8Path || !nookletHostWindow) return;
    NSString *path = [NSString stringWithUTF8String:utf8Path];
    if (!path.length || ![NSFileManager.defaultManager fileExistsAtPath:path]) return;
    booklandPrepareConversionCard();
    if (!booklandConversionWindow) return;
    ++booklandConversionGeneration;
    unsigned long long generation = booklandConversionGeneration;
    booklandConversionDismissing = NO;
    NSURL *url = [NSURL fileURLWithPath:path];
    booklandConversionURL = url;
    booklandConversionName.stringValue = url.lastPathComponent ?: @"Converted file";
    NSString *subtitle = utf8Subtitle ? [NSString stringWithUTF8String:utf8Subtitle] : nil;
    booklandConversionStatus.stringValue = subtitle.length ? subtitle : @"File ready";
    booklandConversionPreview.image = [NSWorkspace.sharedWorkspace iconForFile:path];
    if (booklandConversionRequest) {
        [QLThumbnailGenerator.sharedGenerator cancelRequest:booklandConversionRequest];
    }
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
    if (!screen) return;
    booklandPositionConversionCard(panelBottom, NO);
    CALayer *layer = booklandConversionGlass.layer;
    [layer removeAllAnimations];
    if (!booklandConversionWindow.isVisible) [booklandConversionWindow orderFront:nil];
    if (nooklet_reduce_motion()) {
        layer.opacity = 1;
    } else {
        CASpringAnimation *enter = [CASpringAnimation
            animationWithKeyPath:@"transform.translation.y"];
        enter.fromValue = @14;
        enter.toValue = @0;
        enter.mass = 1;
        enter.stiffness = 140;
        enter.damping = 18;
        enter.duration = fmin(0.55, enter.settlingDuration);
        CABasicAnimation *fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
        fade.fromValue = @0;
        fade.toValue = @1;
        fade.duration = 0.13;
        [CATransaction begin];
        [CATransaction setDisableActions:YES];
        layer.opacity = 1;
        [layer addAnimation:enter forKey:@"booklandConversionEnter"];
        [layer addAnimation:fade forKey:@"booklandConversionFade"];
        [CATransaction commit];
    }
    QLThumbnailGenerationRequest *request = [[QLThumbnailGenerationRequest alloc]
        initWithFileAtURL:url size:CGSizeMake(56, 56)
                    scale:screen.backingScaleFactor
      representationTypes:QLThumbnailGenerationRequestRepresentationTypeThumbnail];
    booklandConversionRequest = request;
    [QLThumbnailGenerator.sharedGenerator generateBestRepresentationForRequest:request
        completionHandler:^(QLThumbnailRepresentation *thumbnail, NSError *error) {
            (void)error;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (generation != booklandConversionGeneration) return;
                if (thumbnail.NSImage) booklandConversionPreview.image = thumbnail.NSImage;
                booklandConversionRequest = nil;
            });
        }];
    double lifetime = isfinite(seconds) ? fmax(1, fmin(seconds, 10)) : 3;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(lifetime * NSEC_PER_SEC)),
        dispatch_get_main_queue(), ^{
            if (generation == booklandConversionGeneration)
                bookland_conversion_card_hide();
        });
}
