#import <AVFoundation/AVFoundation.h>
#import <Speech/Speech.h>
#import <string.h>

static BOOL bookland_locale_supported(NSString *identifier) {
    NSLocale *requested = [NSLocale localeWithLocaleIdentifier:identifier];
    for (NSLocale *locale in SFSpeechRecognizer.supportedLocales) {
        if ([locale.localeIdentifier isEqualToString:requested.localeIdentifier]) return YES;
    }
    return NO;
}

// All mutable state lives on the main queue. Speech and permission callbacks
// dispatch there before changing it; the Rust UI also calls these functions on
// the main thread. Audio buffers are forwarded by AVAudioEngine's tap.
@interface BooklandSpeech : NSObject
@property(nonatomic, strong) AVAudioEngine *engine;
@property(nonatomic, strong) SFSpeechRecognizer *recognizer;
@property(nonatomic, strong) SFSpeechAudioBufferRecognitionRequest *request;
@property(nonatomic, strong) SFSpeechRecognitionTask *task;
@property(nonatomic, copy) NSString *transcript;
@property(nonatomic, copy) NSString *message;
@property(nonatomic, copy) NSString *localeID;
@property(nonatomic) NSInteger state;
@property(nonatomic) BOOL desired;
@property(nonatomic) BOOL allowNetwork;
@property(nonatomic) BOOL tapInstalled;
@property(nonatomic) NSUInteger generation;
- (void)begin;
- (void)stop;
@end

@implementation BooklandSpeech
- (instancetype)init {
    if ((self = [super init])) {
        _transcript = @"";
        _message = @"Voice mode is ready";
        _localeID = @"en-US";
        _state = 0;
    }
    return self;
}

- (void)releaseAudio {
    if (self.engine) {
        if (self.tapInstalled) [self.engine.inputNode removeTapOnBus:0];
        [self.engine stop];
    }
    self.tapInstalled = NO;
    [self.request endAudio];
    [self.task cancel];
    self.engine = nil;
    self.request = nil;
    self.task = nil;
    self.recognizer = nil;
}

- (void)stop {
    self.desired = NO;
    self.generation += 1;
    [self releaseAudio];
    self.state = 0;
    self.message = @"Voice mode stopped";
}

- (void)begin {
    if (!self.desired) return;
    NSUInteger current = self.generation;
    SFSpeechRecognizerAuthorizationStatus speech = SFSpeechRecognizer.authorizationStatus;
    if (speech == SFSpeechRecognizerAuthorizationStatusNotDetermined) {
        self.state = 1;
        self.message = @"Waiting for Speech Recognition permission…";
        [SFSpeechRecognizer requestAuthorization:^(SFSpeechRecognizerAuthorizationStatus ignored) {
            (void)ignored;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (self.desired && self.generation == current) [self begin];
            });
        }];
        return;
    }
    if (speech != SFSpeechRecognizerAuthorizationStatusAuthorized) {
        self.state = -1;
        self.message = @"Speech Recognition access was denied in System Settings";
        self.desired = NO;
        return;
    }
    AVAudioApplicationRecordPermission microphone = AVAudioApplication.sharedInstance.recordPermission;
    if (microphone == AVAudioApplicationRecordPermissionUndetermined) {
        self.state = 2;
        self.message = @"Waiting for microphone permission…";
        [AVAudioApplication requestRecordPermissionWithCompletionHandler:^(BOOL ignored) {
            (void)ignored;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (self.desired && self.generation == current) [self begin];
            });
        }];
        return;
    }
    if (microphone != AVAudioApplicationRecordPermissionGranted) {
        self.state = -1;
        self.message = @"Microphone access was denied in System Settings";
        self.desired = NO;
        return;
    }
    if (!bookland_locale_supported(self.localeID)) {
        self.state = -2;
        self.message = @"This speech language code is not supported on this Mac";
        self.desired = NO;
        return;
    }
    self.recognizer = [[SFSpeechRecognizer alloc] initWithLocale:
        [NSLocale localeWithLocaleIdentifier:self.localeID]];
    if (!self.recognizer || !self.recognizer.isAvailable) {
        self.state = -2;
        self.message = @"Speech recognition is unavailable for this language";
        self.desired = NO;
        return;
    }
    if (!self.allowNetwork && !self.recognizer.supportsOnDeviceRecognition) {
        self.state = -2;
        self.message = @"On-device recognition is unavailable for this language";
        self.desired = NO;
        return;
    }
    self.request = [[SFSpeechAudioBufferRecognitionRequest alloc] init];
    self.request.shouldReportPartialResults = YES;
    self.request.requiresOnDeviceRecognition = !self.allowNetwork;
    self.request.taskHint = SFSpeechRecognitionTaskHintDictation;
    self.engine = [[AVAudioEngine alloc] init];
    AVAudioInputNode *input = self.engine.inputNode;
    AVAudioFormat *format = [input outputFormatForBus:0];
    if (!input || !format || format.channelCount == 0) {
        self.state = -3;
        self.message = @"No microphone input is available";
        [self releaseAudio];
        self.desired = NO;
        return;
    }
    SFSpeechAudioBufferRecognitionRequest *request = self.request;
    [input installTapOnBus:0 bufferSize:1024 format:format
        block:^(AVAudioPCMBuffer *buffer, AVAudioTime *when) {
            (void)when;
            [request appendAudioPCMBuffer:buffer];
        }];
    self.tapInstalled = YES;
    self.state = 3;
    self.message = @"Starting microphone…";
    __weak BooklandSpeech *weakSelf = self;
    self.task = [self.recognizer recognitionTaskWithRequest:self.request
        resultHandler:^(SFSpeechRecognitionResult *result, NSError *error) {
            dispatch_async(dispatch_get_main_queue(), ^{
                BooklandSpeech *strongSelf = weakSelf;
                if (!strongSelf || strongSelf.generation != current || !strongSelf.desired) return;
                if (result) {
                    NSString *spoken = result.bestTranscription.formattedString ?: @"";
                    if (![strongSelf.transcript isEqualToString:spoken]) strongSelf.transcript = spoken;
                }
                if (error || result.isFinal) {
                    strongSelf.message = error ? error.localizedDescription : @"Voice session ended. Start again to continue.";
                    strongSelf.state = error ? -3 : 5;
                    strongSelf.desired = NO;
                    [strongSelf releaseAudio];
                }
            });
        }];
    NSError *startError = nil;
    [self.engine prepare];
    if (![self.engine startAndReturnError:&startError]) {
        self.state = -3;
        self.message = startError.localizedDescription ?: @"Could not start the microphone";
        self.desired = NO;
        [self releaseAudio];
        return;
    }
    self.state = 4;
    self.message = self.allowNetwork ? @"Listening · Apple speech service may use the network"
                                     : @"Listening · on-device recognition";
}
@end

static BooklandSpeech *bookland_speech(void) {
    static BooklandSpeech *speech;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ speech = [[BooklandSpeech alloc] init]; });
    return speech;
}

int bookland_speech_start(const char *locale, int allowNetwork) {
    if (!locale) return 0;
    NSString *identifier = [NSString stringWithUTF8String:locale];
    if (!identifier || identifier.length == 0 || identifier.length > 64) return 0;
    BooklandSpeech *speech = bookland_speech();
    [speech stop];
    speech.localeID = identifier;
    speech.allowNetwork = allowNetwork != 0;
    speech.transcript = @"";
    speech.desired = YES;
    [speech begin];
    return 1;
}

void bookland_speech_stop(void) { [bookland_speech() stop]; }
int bookland_speech_state(void) { return (int)bookland_speech().state; }

static size_t copy_string(NSString *value, char *buffer, size_t capacity) {
    if (!buffer || capacity == 0) return 0;
    NSData *data = [value dataUsingEncoding:NSUTF8StringEncoding];
    if (!data || data.length >= capacity) return 0;
    memcpy(buffer, data.bytes, data.length);
    buffer[data.length] = 0;
    return data.length;
}
size_t bookland_speech_transcript(char *buffer, size_t capacity) {
    return copy_string(bookland_speech().transcript, buffer, capacity);
}
size_t bookland_speech_message(char *buffer, size_t capacity) {
    return copy_string(bookland_speech().message, buffer, capacity);
}

// Read-only capability probe; it never requests microphone or speech access.
int bookland_speech_capabilities(const char *locale) {
    if (!locale) return 0;
    NSString *identifier = [NSString stringWithUTF8String:locale];
    if (!identifier || !bookland_locale_supported(identifier)) return 0;
    SFSpeechRecognizer *recognizer = [[SFSpeechRecognizer alloc]
        initWithLocale:[NSLocale localeWithLocaleIdentifier:identifier]];
    if (!recognizer) return 0;
    return 1 | (recognizer.supportsOnDeviceRecognition ? 2 : 0)
        | (recognizer.isAvailable ? 4 : 0);
}
