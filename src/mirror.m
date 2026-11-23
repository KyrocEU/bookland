#import <AVFoundation/AVFoundation.h>
#import <CoreVideo/CoreVideo.h>
#import <stdatomic.h>
#import <string.h>

static AVCaptureSession *mirrorSession;
static AVCaptureVideoDataOutput *mirrorOutput;
static id mirrorDelegate;
static dispatch_queue_t mirrorControlQueue;
static NSLock *mirrorFrameLock;
static NSData *mirrorFrame;
static int mirrorWidth;
static int mirrorHeight;
static uint64_t mirrorSequence;
static atomic_int mirrorState;
static atomic_bool mirrorPermissionRequested;
static atomic_uint mirrorGeneration;

@interface BooklandMirrorFrames : NSObject <AVCaptureVideoDataOutputSampleBufferDelegate>
@end

@implementation BooklandMirrorFrames
- (void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer
       fromConnection:(AVCaptureConnection *)connection {
    (void)output;
    (void)connection;
    CVPixelBufferRef pixel = CMSampleBufferGetImageBuffer(sampleBuffer);
    if (!pixel || CVPixelBufferGetPixelFormatType(pixel) != kCVPixelFormatType_32BGRA) return;
    if (CVPixelBufferLockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return;
    const size_t width = CVPixelBufferGetWidth(pixel);
    const size_t height = CVPixelBufferGetHeight(pixel);
    const size_t stride = CVPixelBufferGetBytesPerRow(pixel);
    const unsigned char *source = CVPixelBufferGetBaseAddress(pixel);
    if (source && width && height && width <= 1280 && height <= 720 && stride >= width * 4) {
        NSMutableData *next = [NSMutableData dataWithLength:width * height * 4];
        unsigned char *target = next.mutableBytes;
        for (size_t y = 0; y < height; y++) {
            const unsigned char *row = source + y * stride;
            unsigned char *destination = target + y * width * 4;
            for (size_t x = 0; x < width; x++) {
                const unsigned char *bgra = row + x * 4;
                unsigned char *rgba = destination + (width - 1 - x) * 4;
                rgba[0] = bgra[2];
                rgba[1] = bgra[1];
                rgba[2] = bgra[0];
                rgba[3] = 255;
            }
        }
        [mirrorFrameLock lock];
        mirrorFrame = next;
        mirrorWidth = (int)width;
        mirrorHeight = (int)height;
        mirrorSequence++;
        [mirrorFrameLock unlock];
    }
    CVPixelBufferUnlockBaseAddress(pixel, kCVPixelBufferLock_ReadOnly);
}
@end

int bookland_mirror_permission(void) {
    return (int)[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
}

bool bookland_mirror_camera_available(void) {
    return [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo] != nil;
}

int bookland_mirror_state(void) {
    return atomic_load(&mirrorState);
}

int bookland_mirror_start(void) {
    AVAuthorizationStatus permission = [AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeVideo];
    if (permission == AVAuthorizationStatusNotDetermined) {
        if (!atomic_exchange(&mirrorPermissionRequested, true)) {
            [AVCaptureDevice requestAccessForMediaType:AVMediaTypeVideo completionHandler:^(BOOL granted) {
                (void)granted;
                atomic_store(&mirrorPermissionRequested, false);
            }];
        }
        return 1;
    }
    if (permission != AVAuthorizationStatusAuthorized) return -1;
    if (mirrorSession) return atomic_load(&mirrorState);
    AVCaptureDevice *camera = [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeVideo];
    if (!camera) return -2;
    NSError *error = nil;
    AVCaptureDeviceInput *input = [AVCaptureDeviceInput deviceInputWithDevice:camera error:&error];
    if (!input || error) return -3;
    AVCaptureSession *session = [AVCaptureSession new];
    [session beginConfiguration];
    if ([session canSetSessionPreset:AVCaptureSessionPreset640x480])
        session.sessionPreset = AVCaptureSessionPreset640x480;
    if (![session canAddInput:input]) { [session commitConfiguration]; return -3; }
    [session addInput:input];
    AVCaptureVideoDataOutput *output = [AVCaptureVideoDataOutput new];
    output.videoSettings = @{(id)kCVPixelBufferPixelFormatTypeKey: @(kCVPixelFormatType_32BGRA)};
    output.alwaysDiscardsLateVideoFrames = YES;
    if (![session canAddOutput:output]) { [session commitConfiguration]; return -3; }
    [session addOutput:output];
    if (!mirrorFrameLock) mirrorFrameLock = [NSLock new];
    mirrorDelegate = [BooklandMirrorFrames new];
    dispatch_queue_t frameQueue = dispatch_queue_create("bookland.mirror.frames", DISPATCH_QUEUE_SERIAL);
    [output setSampleBufferDelegate:mirrorDelegate queue:frameQueue];
    [session commitConfiguration];
    mirrorOutput = output;
    mirrorSession = session;
    if (!mirrorControlQueue)
        mirrorControlQueue = dispatch_queue_create("bookland.mirror.control", DISPATCH_QUEUE_SERIAL);
    atomic_store(&mirrorState, 1);
    unsigned int generation = atomic_fetch_add(&mirrorGeneration, 1) + 1;
    dispatch_async(mirrorControlQueue, ^{
        [session startRunning];
        if (atomic_load(&mirrorGeneration) == generation)
            atomic_store(&mirrorState, session.isRunning ? 2 : -3);
    });
    return 1;
}

void bookland_mirror_stop(void) {
    atomic_fetch_add(&mirrorGeneration, 1);
    AVCaptureSession *session = mirrorSession;
    mirrorSession = nil;
    if (mirrorOutput) [mirrorOutput setSampleBufferDelegate:nil queue:NULL];
    mirrorOutput = nil;
    mirrorDelegate = nil;
    atomic_store(&mirrorState, 0);
    if (mirrorFrameLock) {
        [mirrorFrameLock lock];
        mirrorFrame = nil;
        mirrorWidth = 0;
        mirrorHeight = 0;
        [mirrorFrameLock unlock];
    }
    if (session && mirrorControlQueue) {
        dispatch_async(mirrorControlQueue, ^{ [session stopRunning]; });
    }
}

size_t bookland_mirror_copy(unsigned char *buffer, size_t capacity,
                            int *width, int *height, uint64_t *sequence) {
    if (!mirrorFrameLock || !buffer || !width || !height || !sequence) return 0;
    [mirrorFrameLock lock];
    size_t length = mirrorFrame.length;
    *width = mirrorWidth;
    *height = mirrorHeight;
    *sequence = mirrorSequence;
    if (length && capacity >= length) memcpy(buffer, mirrorFrame.bytes, length);
    [mirrorFrameLock unlock];
    return length;
}
