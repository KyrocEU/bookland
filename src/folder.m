#import <AppKit/AppKit.h>
#include <string.h>

size_t bookland_choose_folder(char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.canChooseFiles = NO;
    panel.canChooseDirectories = YES;
    panel.allowsMultipleSelection = NO;
    panel.prompt = @"Watch folder";
    panel.message = @"Choose the folder Bookland should watch for downloads.";
    if ([panel runModal] != NSModalResponseOK) return 0;
    const char *path = panel.URL.path.UTF8String;
    if (!path) return 0;
    size_t length = strlen(path);
    if (length >= capacity) return 0;
    memcpy(buffer, path, length + 1);
    return length;
}

size_t bookland_choose_prompter_script(char *buffer, size_t capacity) {
    if (!buffer || capacity < 2) return 0;
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.canChooseFiles = YES;
    panel.canChooseDirectories = NO;
    panel.allowsMultipleSelection = NO;
    panel.prompt = @"Load script";
    panel.message = @"Choose a UTF-8 plain-text script for Bookland Prompter.";
    if ([panel runModal] != NSModalResponseOK) return 0;
    NSURL *url = panel.URL;
    NSDictionary *attributes = [NSFileManager.defaultManager attributesOfItemAtPath:url.path error:nil];
    unsigned long long bytes = [attributes fileSize];
    if (bytes == 0 || bytes >= capacity || bytes > 1000000) return 0;
    NSData *data = [NSData dataWithContentsOfURL:url options:NSDataReadingMappedIfSafe error:nil];
    if (!data || data.length == 0 || data.length >= capacity) return 0;
    NSString *text = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding];
    if (!text) return 0;
    memcpy(buffer, data.bytes, data.length);
    buffer[data.length] = 0;
    return data.length;
}

size_t bookland_choose_pocket_file(char *buffer, size_t capacity) {
    if (!buffer || capacity < 2) return 0;
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.canChooseFiles = YES;
    panel.canChooseDirectories = YES;
    panel.allowsMultipleSelection = NO;
    panel.prompt = @"Add to Pocket";
    panel.message = @"Choose a file or folder for Bookland Pocket.";
    if ([panel runModal] != NSModalResponseOK) return 0;
    const char *path = panel.URL.path.UTF8String;
    if (!path) return 0;
    size_t length = strlen(path);
    if (length >= capacity) return 0;
    memcpy(buffer, path, length + 1);
    return length;
}
