#import <IOBluetooth/IOBluetooth.h>
#import <CoreBluetooth/CoreBluetooth.h>
#import <stdatomic.h>

static BOOL booklandAirPodsWatching;
static IOBluetoothUserNotification *booklandConnectNotification;
static NSMutableDictionary<NSString *, IOBluetoothUserNotification *> *booklandDisconnectNotifications;
static atomic_bool booklandAirPodsChanged;
static CBCentralManager *booklandPermissionManager;
int bookland_airpods_watch_set(int enabled);

@interface BooklandAirPodsObserver : NSObject
- (void)deviceConnected:(IOBluetoothUserNotification *)notification
                device:(IOBluetoothDevice *)device;
- (void)deviceDisconnected:(IOBluetoothUserNotification *)notification
                   device:(IOBluetoothDevice *)device;
@end

static BooklandAirPodsObserver *booklandAirPodsObserver;

@interface BooklandBluetoothPermissionObserver : NSObject <CBCentralManagerDelegate>
@end
static BooklandBluetoothPermissionObserver *booklandPermissionObserver;

@implementation BooklandBluetoothPermissionObserver
- (void)centralManagerDidUpdateState:(CBCentralManager *)central {
    (void)central;
    if (CBManager.authorization == CBManagerAuthorizationAllowedAlways) {
        bookland_airpods_watch_set(1);
        atomic_store(&booklandAirPodsChanged, true);
    }
    if (CBManager.authorization != CBManagerAuthorizationNotDetermined) {
        dispatch_async(dispatch_get_main_queue(), ^{
            booklandPermissionManager.delegate = nil;
            booklandPermissionManager = nil;
            booklandPermissionObserver = nil;
        });
    }
}
@end

// 0: not decided, 1: authorized, 2: denied or restricted.
int bookland_airpods_authorization(void) {
    CBManagerAuthorization authorization = CBManager.authorization;
    if (authorization == CBManagerAuthorizationAllowedAlways) return 1;
    return authorization == CBManagerAuthorizationNotDetermined ? 0 : 2;
}

int bookland_airpods_request_access(void) {
    int authorization = bookland_airpods_authorization();
    if (authorization == 1) return bookland_airpods_watch_set(1);
    if (authorization == 2) return 0;
    if (!booklandPermissionManager) {
        booklandPermissionObserver = [BooklandBluetoothPermissionObserver new];
        booklandPermissionManager = [[CBCentralManager alloc]
            initWithDelegate:booklandPermissionObserver queue:dispatch_get_main_queue()
                    options:@{ CBCentralManagerOptionShowPowerAlertKey: @NO }];
    }
    return booklandPermissionManager != nil;
}

int bookland_airpods_watch_status(void) {
    return booklandAirPodsWatching ? 1 : 0;
}

static void booklandWatchDisconnect(IOBluetoothDevice *device) {
    NSString *key = device.addressString;
    if (!key || booklandDisconnectNotifications[key]) return;
    IOBluetoothUserNotification *notification =
        [device registerForDisconnectNotification:booklandAirPodsObserver
                                         selector:@selector(deviceDisconnected:device:)];
    if (notification) booklandDisconnectNotifications[key] = notification;
}

@implementation BooklandAirPodsObserver
- (void)deviceConnected:(IOBluetoothUserNotification *)notification
                device:(IOBluetoothDevice *)device {
    (void)notification;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!booklandAirPodsWatching) return;
        booklandWatchDisconnect(device);
        atomic_store(&booklandAirPodsChanged, true);
    });
}
- (void)deviceDisconnected:(IOBluetoothUserNotification *)notification
                   device:(IOBluetoothDevice *)device {
    (void)notification;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (!booklandAirPodsWatching) return;
        NSString *key = device.addressString;
        IOBluetoothUserNotification *registered = booklandDisconnectNotifications[key];
        [registered unregister];
        if (key) [booklandDisconnectNotifications removeObjectForKey:key];
        atomic_store(&booklandAirPodsChanged, true);
    });
}
@end

// Connection callbacks mark the status dirty. Rust checks this flag on its
// existing idle repaint, so Bluetooth changes add no continuous redraw loop.
int bookland_airpods_watch_set(int enabled) {
    if (!enabled) {
        booklandAirPodsWatching = NO;
        booklandPermissionManager.delegate = nil;
        booklandPermissionManager = nil;
        booklandPermissionObserver = nil;
        [booklandConnectNotification unregister];
        booklandConnectNotification = nil;
        for (IOBluetoothUserNotification *notification in
             booklandDisconnectNotifications.allValues) [notification unregister];
        [booklandDisconnectNotifications removeAllObjects];
        booklandAirPodsObserver = nil;
        atomic_store(&booklandAirPodsChanged, false);
        return 1;
    }
    if (booklandAirPodsWatching) return 1;
    // Querying this class property does not create a manager or prompt. A
    // fresh privacy request inside IOBluetooth's synchronous registration can
    // block the island's first frame, so keep the existing report fallback.
    if (CBManager.authorization != CBManagerAuthorizationAllowedAlways) return 0;
    booklandAirPodsObserver = [BooklandAirPodsObserver new];
    booklandDisconnectNotifications = [NSMutableDictionary new];
    booklandConnectNotification = [IOBluetoothDevice
        registerForConnectNotifications:booklandAirPodsObserver
                              selector:@selector(deviceConnected:device:)];
    if (!booklandConnectNotification) {
        booklandAirPodsObserver = nil;
        booklandDisconnectNotifications = nil;
        return 0;
    }
    booklandAirPodsWatching = YES;
    for (IOBluetoothDevice *device in IOBluetoothDevice.pairedDevices) {
        if (device.isConnected) booklandWatchDisconnect(device);
    }
    return 1;
}

int bookland_airpods_events_take(void) {
    return atomic_exchange(&booklandAirPodsChanged, false) ? 1 : 0;
}
