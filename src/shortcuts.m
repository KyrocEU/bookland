#import <Carbon/Carbon.h>
#include <stdatomic.h>

static EventHotKeyRef nookletHotKey;
static EventHotKeyRef clipboardHotKey;
static EventHandlerRef nookletHandler;
static atomic_int pendingToggle;
static atomic_int pendingClipboard;

static OSStatus onHotKey(EventHandlerCallRef next, EventRef event, void *context) {
    (void)next; (void)context;
    EventHotKeyID hotKeyID;
    if (GetEventParameter(event, kEventParamDirectObject, typeEventHotKeyID,
                          NULL, sizeof(hotKeyID), NULL, &hotKeyID) != noErr) return eventNotHandledErr;
    if (hotKeyID.id == 2) atomic_store(&pendingClipboard, 1);
    else if (hotKeyID.id == 1) atomic_store(&pendingToggle, 1);
    return noErr;
}

int nooklet_shortcut_set(int enabled) {
    if (!enabled) {
        if (nookletHotKey) {
            UnregisterEventHotKey(nookletHotKey);
            nookletHotKey = NULL;
        }
        return 1;
    }
    if (nookletHotKey) return 1;
    if (!nookletHandler) {
        EventTypeSpec eventType = { kEventClassKeyboard, kEventHotKeyPressed };
        if (InstallApplicationEventHandler(NewEventHandlerUPP(onHotKey), 1,
                                           &eventType, NULL, &nookletHandler) != noErr) return 0;
    }
    EventHotKeyID hotKeyID = { 'Nook', 1 };
    return RegisterEventHotKey(kVK_ANSI_N, cmdKey | optionKey, hotKeyID,
                               GetApplicationEventTarget(), 0, &nookletHotKey) == noErr;
}

int nooklet_shortcut_take(void) {
    return atomic_exchange(&pendingToggle, 0);
}

int bookland_clipboard_shortcut_set(int enabled) {
    if (!enabled) {
        if (clipboardHotKey) {
            UnregisterEventHotKey(clipboardHotKey);
            clipboardHotKey = NULL;
        }
        return 1;
    }
    if (clipboardHotKey) return 1;
    if (!nookletHandler) {
        EventTypeSpec eventType = { kEventClassKeyboard, kEventHotKeyPressed };
        if (InstallApplicationEventHandler(NewEventHandlerUPP(onHotKey), 1,
                                           &eventType, NULL, &nookletHandler) != noErr) return 0;
    }
    EventHotKeyID hotKeyID = { 'Nook', 2 };
    return RegisterEventHotKey(kVK_ANSI_6, cmdKey | controlKey, hotKeyID,
                               GetApplicationEventTarget(), 0, &clipboardHotKey) == noErr;
}

int bookland_clipboard_shortcut_take(void) {
    return atomic_exchange(&pendingClipboard, 0);
}
