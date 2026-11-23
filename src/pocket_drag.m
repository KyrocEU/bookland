#import <AppKit/AppKit.h>
#import <math.h>
// Polling mouse position avoids an Accessibility or Input Monitoring permission.
// The drag pasteboard's change count distinguishes a current file drag from
// file URLs left on that pasteboard by an earlier drag.
static NSTimer *monitorTimer;
static NSInteger idleDragChange;
static BOOL buttonWasDown;
static BOOL firedForDrag;
static BOOL pending;
static double pendingX;
static double pendingY;
static void *wakeContext;
static void (*wakeCallback)(void *);
static bool (*shakeSample)(double, bool);
extern int nooklet_hover_pointer_screen(double *x, double *y);

static bool pocketAnchorNearMouse(NSPoint mouse, double *outX, double *outY) {
    NSArray<NSScreen *> *screens = NSScreen.screens;
    NSScreen *screen = screens.firstObject;
    for (NSScreen *candidate in screens) {
        if (NSPointInRect(mouse, candidate.frame)) { screen = candidate; break; }
    }
    if (!screen || !outX || !outY) return false;
    NSRect visible = screen.visibleFrame;
    const CGFloat width = 430.0;
    const CGFloat height = 295.0;
    CGFloat left = mouse.x + 20.0;
    if (left + width > NSMaxX(visible)) left = mouse.x - width - 20.0;
    left = fmax(NSMinX(visible) + 8.0,
                fmin(left, NSMaxX(visible) - width - 8.0));
    CGFloat bottom = mouse.y - height - 18.0;
    if (bottom < NSMinY(visible)) bottom = mouse.y + 18.0;
    bottom = fmax(NSMinY(visible) + 8.0,
                  fmin(bottom, NSMaxY(visible) - height - 8.0));
    *outX = left;
    *outY = NSMaxY(screens.firstObject.frame) - (bottom + height);
    return true;
}

static void placePocketNearMouse(NSPoint mouse) {
    if (!pocketAnchorNearMouse(mouse, &pendingX, &pendingY)) return;
    pending = YES;
    if (wakeCallback) wakeCallback(wakeContext);
}

bool bookland_pocket_anchor_at_cursor(double *x, double *y) {
    return pocketAnchorNearMouse(NSEvent.mouseLocation, x, y);
}

bool bookland_pointer_over_island_rect(double x, double y, double width,
                                      double height, double margin,
                                      bool requireReleased) {
    if (requireReleased && [NSEvent pressedMouseButtons] != 0) return false;
    NSPoint mouse = NSEvent.mouseLocation;
    double eventX = 0.0, eventY = 0.0;
    int eventState = nooklet_hover_pointer_screen(&eventX, &eventY);
    // Mouse-exit tracking can lag a window resize or a synthetic pointer move.
    // The live screen position still gives the correct hit test in that case.
    if (eventState == 1) mouse = NSMakePoint(eventX, eventY);
    for (NSWindow *window in NSApp.windows) {
        if (![window.title isEqualToString:@"Bookland"] || !window.isVisible) continue;
        NSPoint local = [window convertPointFromScreen:mouse];
        CGFloat topY = window.contentView.bounds.size.height - local.y;
        NSRect target = NSMakeRect(x - margin, y - margin,
                                   width + margin * 2.0, height + margin * 2.0);
        return NSPointInRect(NSMakePoint(local.x, topY), target);
    }
    return false;
}

static void sampleDrag(void) {
    NSPasteboard *drag = [NSPasteboard pasteboardWithName:NSPasteboardNameDrag];
    BOOL down = ([NSEvent pressedMouseButtons] & 1) != 0;
    if (!down) {
        idleDragChange = drag.changeCount;
        if (buttonWasDown) {
            firedForDrag = NO;
            if (shakeSample) shakeSample(0.0, false);
        }
        buttonWasDown = NO;
        return;
    }
    buttonWasDown = YES;
    if (firedForDrag || drag.changeCount == idleDragChange ||
        ![drag.types containsObject:NSPasteboardTypeFileURL] || !shakeSample) return;
    NSPoint mouse = NSEvent.mouseLocation;
    if (shakeSample(mouse.x, true)) {
        firedForDrag = YES;
        placePocketNearMouse(mouse);
    }
}

void bookland_pocket_drag_monitor_set(bool enabled, void *context,
                                      void (*wake)(void *),
                                      bool (*sample)(double, bool)) {
    if (enabled && !monitorTimer) {
        wakeContext = context;
        wakeCallback = wake;
        shakeSample = sample;
        idleDragChange = [NSPasteboard pasteboardWithName:NSPasteboardNameDrag].changeCount;
        buttonWasDown = NO;
        firedForDrag = NO;
        monitorTimer = [NSTimer timerWithTimeInterval:0.025 repeats:YES
            block:^(NSTimer *timer) { (void)timer; sampleDrag(); }];
        [[NSRunLoop mainRunLoop] addTimer:monitorTimer forMode:NSRunLoopCommonModes];
    } else if (!enabled && monitorTimer) {
        [monitorTimer invalidate];
        monitorTimer = nil;
        pending = NO;
        wakeContext = NULL;
        wakeCallback = NULL;
        if (shakeSample) shakeSample(0.0, false);
        shakeSample = NULL;
    }
}

bool bookland_pocket_drag_monitor_take(double *x, double *y) {
    if (!pending || !x || !y) return false;
    *x = pendingX;
    *y = pendingY;
    pending = NO;
    return true;
}
