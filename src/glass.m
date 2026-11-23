#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>
#import <os/log.h>

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
    int rate = fps <= 30 ? 30 : 60;
    if (rate == currentRate) return;
    booklandAnimationLink.preferredFrameRateRange = CAFrameRateRangeMake(rate, rate, rate);
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

void nooklet_anchor_host_to_screen_top(int enabled) {
    if (!nookletHostWindow || !enabled) return;
    // Winit positions borderless windows below the menu bar on this Mac even
    // when asked for y=0. The real notch is in the menu-bar band.
    nookletHostWindow.level = NSStatusWindowLevel;
    NSScreen *screen = nookletHostWindow.screen ?: NSScreen.mainScreen;
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
        nookletMorphView.hidden = NO;
        return;
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

void nooklet_progress_ring_set(int enabled, double x, double y) {
    if (!nookletHostWindow) return;
    if (!enabled) {
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
        CGMutablePathRef path = CGPathCreateMutable();
        CGPathAddEllipseInRect(path, NULL, CGRectMake(7, 7, 14, 14));
        nookletProgressTrack = [CAShapeLayer layer];
        nookletProgressTrack.bounds = CGRectMake(0, 0, 28, 28);
        nookletProgressTrack.path = path;
        nookletProgressTrack.fillColor = NSColor.clearColor.CGColor;
        nookletProgressTrack.strokeColor = [NSColor colorWithCalibratedRed:0.204
            green:0.365 blue:0.612 alpha:0.43].CGColor;
        nookletProgressTrack.lineWidth = 1.5;
        [content.layer addSublayer:nookletProgressTrack];
        CGPathRelease(path);

        path = CGPathCreateMutable();
        CGPathAddArc(path, NULL, 14, 14, 7, 0, -M_PI_2, true);
        nookletProgressArc = [CAShapeLayer layer];
        nookletProgressArc.bounds = CGRectMake(0, 0, 28, 28);
        nookletProgressArc.path = path;
        nookletProgressArc.fillColor = NSColor.clearColor.CGColor;
        nookletProgressArc.strokeColor = [NSColor colorWithCalibratedRed:0.294
            green:0.545 blue:0.984 alpha:1.0].CGColor;
        nookletProgressArc.lineWidth = 2.0;
        nookletProgressArc.lineCap = kCALineCapRound;
        [content.layer addSublayer:nookletProgressArc];
        CGPathRelease(path);
        CABasicAnimation *spin = [CABasicAnimation animationWithKeyPath:@"transform.rotation.z"];
        spin.fromValue = @0;
        spin.toValue = @(-2.0 * M_PI);
        spin.duration = 3.0;
        spin.repeatCount = HUGE_VALF;
        spin.timingFunction = [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionLinear];
        [nookletProgressArc addAnimation:spin forKey:@"bookland-progress-spin"];
        [nookletHostWindow addChildWindow:window ordered:NSWindowAbove];
        nookletProgressWindow = window;
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
