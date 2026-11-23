#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

// Keep winit's content view untouched: it owns keyboard and IME state.
// A non-interactive child window carries native glass behind its transparent pixels.
static NSGlassEffectView *nookletGlass[8];
static NSView *nookletContainer;
static NSWindow *nookletHostWindow;
static NSWindow *nookletGlassWindow;
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
    NSView *container = [[NSView alloc] initWithFrame:background.contentView.bounds];
    container.wantsLayer = YES;
    container.layer.backgroundColor = NSColor.clearColor.CGColor;
    container.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    background.contentView = container;
    window.opaque = NO;
    window.backgroundColor = NSColor.clearColor;
    // The island is anchored to the screen edge. A drag across transparent
    // margins must not turn the borderless host into a movable window.
    window.movable = NO;
    window.movableByWindowBackground = NO;
    [window addChildWindow:background ordered:NSWindowBelow];
    [background orderFront:nil];
    nookletContainer = container;
    nookletHostWindow = window;
    nookletGlassWindow = background;
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
                       double radius, double level, int enabled) {
    if (!nookletContainer || slot < 0 || slot >= 8) return;
    if (!NSEqualRects(nookletGlassWindow.frame, nookletHostWindow.frame)) {
        [nookletGlassWindow setFrame:nookletHostWindow.frame display:NO];
    }
    NSGlassEffectView *view = nookletGlass[slot];
    if (!enabled || width <= 0 || height <= 0) {
        view.hidden = YES;
        return;
    }
    if (!view) {
        view = [[NSGlassEffectView alloc] initWithFrame:NSZeroRect];
        view.autoresizingMask = NSViewNotSizable;
        view.style = NSGlassEffectViewStyleRegular;
        [nookletContainer addSubview:view positioned:NSWindowBelow relativeTo:nookletContainer.subviews.lastObject];
        nookletGlass[slot] = view;
    }
    double flippedY = nookletContainer.bounds.size.height - y - height;
    view.frame = NSMakeRect(x, flippedY, width, height);
    view.cornerRadius = radius;
    view.tintColor = [NSColor colorWithCalibratedRed:0.055 green:0.063 blue:0.078
                                              alpha:fmax(0.03, fmin(0.18, level * 0.14))];
    view.hidden = NO;
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
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    nookletProgressTrack.position = center;
    nookletProgressArc.position = center;
    [CATransaction commit];
    if (!nookletProgressWindow.isVisible) [nookletProgressWindow orderFront:nil];
}
