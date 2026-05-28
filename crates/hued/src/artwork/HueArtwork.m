#import "HueArtwork.h"
#import <AppKit/AppKit.h>

static uint16_t rgb888_to_rgb565(uint8_t r, uint8_t g, uint8_t b) {
    return ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3);
}

HueArtworkResult hue_artwork_process(const char *path, uint16_t size) {
    HueArtworkResult fail = {0, 0, 0, 0, NULL};

    if (path == NULL || size == 0) {
        return fail;
    }

    NSString *nsPath = [NSString stringWithUTF8String:path];
    NSImage *image = [[NSImage alloc] initWithContentsOfFile:nsPath];

    if (image == nil) {
        return fail;
    }

    NSRect srcRect = NSMakeRect(0, 0, image.size.width, image.size.height);

    CGFloat side = MIN(image.size.width, image.size.height);
    srcRect.origin.x = (image.size.width - side) / 2.0;
    srcRect.origin.y = (image.size.height - side) / 2.0;
    srcRect.size.width = side;
    srcRect.size.height = side;

    size_t width = size;
    size_t height = size;
    size_t rgbaLen = width * height * 4;

    uint8_t *rgba = calloc(rgbaLen, 1);
    if (rgba == NULL) {
        return fail;
    }

    CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();

    CGContextRef ctx = CGBitmapContextCreate(
        rgba,
        width,
        height,
        8,
        width * 4,
        colorSpace,
        kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big
    );

    CGColorSpaceRelease(colorSpace);

    if (ctx == NULL) {
        free(rgba);
        return fail;
    }

    NSGraphicsContext *gc = [NSGraphicsContext graphicsContextWithCGContext:ctx flipped:NO];
    [NSGraphicsContext saveGraphicsState];
    [NSGraphicsContext setCurrentContext:gc];

    [image drawInRect:NSMakeRect(0, 0, width, height)
             fromRect:srcRect
            operation:NSCompositingOperationCopy
             fraction:1.0];

    [NSGraphicsContext restoreGraphicsState];
    CGContextRelease(ctx);

    size_t rgb565Len = width * height * 2;
    uint8_t *rgb565 = malloc(rgb565Len);

    if (rgb565 == NULL) {
        free(rgba);
        return fail;
    }

    for (size_t i = 0; i < width * height; i++) {
        uint8_t r = rgba[i * 4 + 0];
        uint8_t g = rgba[i * 4 + 1];
        uint8_t b = rgba[i * 4 + 2];

        uint16_t px = rgb888_to_rgb565(r, g, b);

        rgb565[i * 2 + 0] = px & 0xFF;
        rgb565[i * 2 + 1] = px >> 8;
    }

    free(rgba);

    HueArtworkResult result;
    result.ok = 1;
    result.width = size;
    result.height = size;
    result.len = rgb565Len;
    result.data = rgb565;

    return result;
}

void hue_artwork_free(uint8_t *ptr, size_t len) {
    (void)len;
    free(ptr);
}
