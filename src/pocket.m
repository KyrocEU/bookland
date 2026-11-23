#import <AppKit/AppKit.h>

bool bookland_airdrop_file(const char *path) {
    if (!path) return false;
    NSString *name = [NSString stringWithUTF8String:path];
    if (!name || ![[NSFileManager defaultManager] fileExistsAtPath:name]) return false;
    NSURL *url = [NSURL fileURLWithPath:name];
    NSSharingService *service = [NSSharingService sharingServiceNamed:NSSharingServiceNameSendViaAirDrop];
    if (!service || ![service canPerformWithItems:@[url]]) return false;
    [service performWithItems:@[url]];
    return true;
}
