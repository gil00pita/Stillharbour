// Read-only, byte-buffer-only bridge. All output storage belongs to the caller.
#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <ImageIO/ImageIO.h>
#include <ColorSync/ColorSync.h>
#include <stdint.h>
#include <string.h>

typedef struct {
    uint32_t source_width, source_height, width, height, orientation;
} SHPreview;

static int allowed_type(CFStringRef type) {
    const CFStringRef types[] = {
        CFSTR("public.heic"), CFSTR("public.heif"), CFSTR("public.heics"),
        CFSTR("public.avif"), CFSTR("public.tiff"), CFSTR("com.compuserve.gif"),
        CFSTR("com.microsoft.bmp"), CFSTR("public.jpeg"), CFSTR("public.png"),
        CFSTR("org.webmproject.webp"), CFSTR("com.adobe.raw-image"),
        CFSTR("com.canon.cr2-raw-image"), CFSTR("com.canon.cr3-raw-image"),
        CFSTR("com.canon.crw-raw-image"), CFSTR("com.nikon.raw-image"),
        CFSTR("com.sony.arw-raw-image"), CFSTR("com.sony.raw-image"),
        CFSTR("com.fuji.raw-image"), CFSTR("com.olympus.raw-image"),
        CFSTR("com.panasonic.raw-image"), CFSTR("com.pentax.raw-image")
    };
    for (size_t i = 0; i < sizeof(types) / sizeof(types[0]); i++) {
        if (CFEqual(type, types[i])) return 1;
    }
    return 0;
}

static int number(CFDictionaryRef values, CFStringRef key, int64_t *out) {
    CFTypeRef value = CFDictionaryGetValue(values, key);
    return value && CFGetTypeID(value) == CFNumberGetTypeID()
        && CFNumberGetValue((CFNumberRef)value, kCFNumberSInt64Type, out);
}

// 1 unsupported type, 2 invalid/decode failure, 3 budget, 4 color conversion setup.
int sh_imageio_preview(const uint8_t *input, size_t length, uint32_t edge, uint32_t hint,
                       uint8_t *rgba, size_t capacity, SHPreview *result) {
    if (hint > 10) return 1;
    if (!input || !rgba || !result || length == 0 || length > 32u * 1024u * 1024u
        || (edge != 256 && edge != 512) || capacity < (size_t)edge * edge * 4u) return 3;
    int status = 2;
    CFDataRef data = NULL;
    CFDictionaryRef source_options = NULL, properties = NULL, options = NULL;
    CFNumberRef size = NULL;
    CGImageSourceRef source = NULL;
    CGImageRef thumbnail = NULL;
    CGColorSpaceRef srgb = NULL;
    CGContextRef context = NULL;
    memset(result, 0, sizeof(*result));
    data = CFDataCreateWithBytesNoCopy(NULL, input, (CFIndex)length, kCFAllocatorNull);
    if (!data) goto done;
    // The backend passes a validated numeric enum, never an arbitrary UTI/path.
    // TIFF-based RAW files (notably ARW) need a hint to avoid the generic TIFF codec.
    const CFStringRef hints[] = {NULL, CFSTR("com.canon.cr2-raw-image"),
        CFSTR("com.canon.cr3-raw-image"), CFSTR("com.nikon.raw-image"),
        CFSTR("com.sony.arw-raw-image"), CFSTR("com.adobe.raw-image"),
        CFSTR("com.fuji.raw-image"), CFSTR("com.olympus.raw-image"),
        CFSTR("com.panasonic.raw-image"), CFSTR("com.canon.crw-raw-image"),
        CFSTR("com.pentax.raw-image")};
    const void *source_keys[] = {kCGImageSourceShouldCache, kCGImageSourceShouldAllowFloat, kCGImageSourceTypeIdentifierHint};
    const void *source_values[] = {kCFBooleanFalse, kCFBooleanFalse, hints[hint]};
    source_options = CFDictionaryCreate(NULL, source_keys, source_values, hint ? 3 : 2,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    if (!source_options) goto done;
    source = CGImageSourceCreateWithData(data, source_options);
    if (!source || CGImageSourceGetCount(source) == 0) goto done;
    CFStringRef type = CGImageSourceGetType(source); // borrowed, never release
    if (!type || !allowed_type(type)) { status = 1; goto done; }
    size_t primary = CGImageSourceGetPrimaryImageIndex(source);
    if (primary >= CGImageSourceGetCount(source)) goto done;
    properties = CGImageSourceCopyPropertiesAtIndex(source, primary, source_options);
    if (!properties) { status = 6; goto done; }
    int64_t width = 0, height = 0, orientation = 1;
    if (!number(properties, kCGImagePropertyPixelWidth, &width)
        || !number(properties, kCGImagePropertyPixelHeight, &height)) { status = 6; goto done; }
    if (width <= 0 || height <= 0 || width > 12000 || height > 12000
        || (uint64_t)width * (uint64_t)height * 4u > 128u * 1024u * 1024u) {
        status = 3; goto done;
    }
    if (CFDictionaryContainsKey(properties, kCGImagePropertyOrientation)
        && (!number(properties, kCGImagePropertyOrientation, &orientation)
            || orientation < 1 || orientation > 8)) goto done;
    int64_t maximum = width > height ? width : height;
    if (maximum > edge) maximum = edge; // never upscale small originals
    size = CFNumberCreate(NULL, kCFNumberSInt64Type, &maximum);
    if (!size) goto done;
    const void *keys[] = {kCGImageSourceCreateThumbnailFromImageIfAbsent,
        kCGImageSourceCreateThumbnailWithTransform, kCGImageSourceThumbnailMaxPixelSize,
        kCGImageSourceShouldCache, kCGImageSourceShouldAllowFloat};
    const void *values[] = {kCFBooleanTrue, kCFBooleanTrue, size,
        kCFBooleanFalse, kCFBooleanFalse};
    options = CFDictionaryCreate(NULL, keys, values, 5,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    if (!options) goto done;
    thumbnail = CGImageSourceCreateThumbnailAtIndex(source, primary, options);
    if (!thumbnail) {
        // Some camera variants have no usable embedded thumbnail. Request a
        // rendered preview under the same input/pixel/time/RSS budgets.
        CFMutableDictionaryRef fallback = CFDictionaryCreateMutableCopy(NULL, 0, options);
        if (fallback) {
            CFDictionaryRemoveValue(fallback, kCGImageSourceCreateThumbnailFromImageIfAbsent);
            CFDictionarySetValue(fallback, kCGImageSourceCreateThumbnailFromImageAlways, kCFBooleanTrue);
            thumbnail = CGImageSourceCreateThumbnailAtIndex(source, primary, fallback);
            CFRelease(fallback);
        }
    }
    if (!thumbnail) { status = 5; goto done; }
    size_t tw = CGImageGetWidth(thumbnail), th = CGImageGetHeight(thumbnail);
    if (!tw || !th || tw > edge || th > edge || tw * th * 4u > capacity) {
        status = 3; goto done;
    }
    srgb = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    if (!srgb) { status = 4; goto done; }
    context = CGBitmapContextCreate(rgba, tw, th, 8, tw * 4u, srgb,
        kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big);
    if (!context) { status = 4; goto done; }
    CGContextSetBlendMode(context, kCGBlendModeCopy);
    CGContextDrawImage(context, CGRectMake(0, 0, tw, th), thumbnail);
    // PNG stores straight alpha; Quartz's explicit RGBA8 context is premultiplied.
    for (size_t pixel = 0; pixel < tw * th; pixel++) {
        uint8_t *p = rgba + pixel * 4u;
        for (size_t channel = 0; channel < 3; channel++) {
            unsigned value = p[3] ? ((unsigned)p[channel] * 255u + p[3] / 2u) / p[3] : 0;
            p[channel] = (uint8_t)(value > 255u ? 255u : value);
        }
    }
    result->source_width = (uint32_t)width; result->source_height = (uint32_t)height;
    result->width = (uint32_t)tw; result->height = (uint32_t)th;
    result->orientation = (uint32_t)orientation;
    status = 0;
done:
    if (context) CGContextRelease(context);
    if (srgb) CGColorSpaceRelease(srgb);
    if (thumbnail) CGImageRelease(thumbnail);
    if (options) CFRelease(options);
    if (size) CFRelease(size);
    if (properties) CFRelease(properties);
    if (source) CFRelease(source);
    if (source_options) CFRelease(source_options);
    if (data) CFRelease(data);
    return status;
}

// Validate explicit baseline ICC data before allowing a native color fallback.
// A native decoder may otherwise silently ignore a malformed profile.
int sh_valid_icc(const uint8_t *bytes, size_t length) {
    if (!bytes || length < 128 || length > 4u * 1024u * 1024u) return 0;
    CFDataRef data = CFDataCreateWithBytesNoCopy(NULL, bytes, (CFIndex)length, kCFAllocatorNull);
    if (!data) return 0;
    CFErrorRef error = NULL, warnings = NULL;
    ColorSyncProfileRef profile = ColorSyncProfileCreate(data, &error);
    if (error) { CFRelease(error); error = NULL; }
    int valid = profile && ColorSyncProfileVerify(profile, &error, &warnings);
    if (warnings) CFRelease(warnings);
    if (error) CFRelease(error);
    if (profile) CFRelease(profile);
    CFRelease(data);
    return valid;
}
