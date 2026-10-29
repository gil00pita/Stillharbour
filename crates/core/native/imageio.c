// Read-only buffer/authorised-handle bridge. All output storage belongs to the caller.
#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <ImageIO/ImageIO.h>
#include <ColorSync/ColorSync.h>
#include <stdint.h>
#include <string.h>
#include <stdatomic.h>
#include <sys/stat.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <stdlib.h>

typedef struct {
    uint32_t source_width, source_height, width, height, orientation;
    uint64_t bytes_read;
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

// The file provider borrows only the already-authorised, read-only stdin
// descriptor. It never receives a pathname, maps the entire file, or writes.
#define SH_FILE_LIMIT (512ull * 1024ull * 1024ull)
#define SH_READ_LIMIT (480ull * 1024ull * 1024ull) // 32 MiB reserved for header/ICC validation
struct SHReader {
    int fd;
    uint64_t length;
    _Atomic uint64_t charged, actual;
    _Atomic int exhausted;
};
static size_t sh_read_at(void *info, void *buffer, off_t position, size_t count) {
    struct SHReader *reader = info;
    if (position < 0 || (uint64_t)position >= reader->length) return 0;
    uint64_t available = reader->length - (uint64_t)position;
    if (count > available) count = (size_t)available;
    uint64_t charged = atomic_load(&reader->charged);
    do {
        if (count > SH_READ_LIMIT - charged) {
            atomic_store(&reader->exhausted, 1); return 0;
        }
    } while (!atomic_compare_exchange_weak(&reader->charged, &charged, charged + count));
    ssize_t result;
    do { result = pread(reader->fd, buffer, count, position); } while (result < 0 && errno == EINTR);
    if (result > 0) atomic_fetch_add(&reader->actual, (uint64_t)result);
    return result > 0 ? (size_t)result : 0;
}

static void sh_release_reader(void *info) {
    struct SHReader *reader = info;
    if (reader) { close(reader->fd); free(reader); }
}

// 1 unsupported type, 2 invalid/decode failure, 3 budget, 4 color conversion setup.
static int sh_preview(const uint8_t *input, size_t length, int fd, uint32_t edge, uint32_t hint,
                       uint8_t *rgba, size_t capacity, SHPreview *result) {
    if (hint > 10) return 1;
    if (!rgba || !result || (edge != 32 && edge != 256 && edge != 512 && edge != 1024 && edge != 2048)
        || capacity < (size_t)edge * edge * 4u) return 3;
    struct SHReader *reader = NULL;
    struct stat details;
    if (fd < 0) {
        if (!input || length == 0 || length > 32u * 1024u * 1024u) return 3;
    } else {
        int flags = fcntl(fd, F_GETFL);
        if (flags < 0 || (flags & O_ACCMODE) != O_RDONLY || fstat(fd, &details) != 0
            || !S_ISREG(details.st_mode) || details.st_size <= 0
            || (uint64_t)details.st_size > SH_FILE_LIMIT) return 3;
        reader = calloc(1, sizeof(*reader));
        if (!reader) return 3;
        reader->fd = fcntl(fd, F_DUPFD_CLOEXEC, 0);
        if (reader->fd < 0) { free(reader); return 2; }
        reader->length = (uint64_t)details.st_size;
    }
    int status = 2;
    CFDataRef data = NULL;
    CGDataProviderRef provider = NULL;
    CFDictionaryRef source_options = NULL, properties = NULL, options = NULL;
    CFNumberRef size = NULL;
    CGImageSourceRef source = NULL;
    CGImageRef thumbnail = NULL;
    CGColorSpaceRef srgb = NULL;
    CGContextRef context = NULL;
    memset(result, 0, sizeof(*result));
    if (fd < 0) {
        data = CFDataCreateWithBytesNoCopy(NULL, input, (CFIndex)length, kCFAllocatorNull);
        if (!data) goto done;
    } else {
        const CGDataProviderDirectCallbacks callbacks = {0, NULL, NULL, sh_read_at, sh_release_reader};
        provider = CGDataProviderCreateDirect(reader, (off_t)reader->length, &callbacks);
        if (!provider) goto done;
    }
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
    source = provider ? CGImageSourceCreateWithDataProvider(provider, source_options)
        : CGImageSourceCreateWithData(data, source_options);
    if (!source || CGImageSourceGetCount(source) == 0) goto done;
    CFStringRef type = CGImageSourceGetType(source); // borrowed, never release
    if (!type || !allowed_type(type)) { status = 1; goto done; }
    int downsample = fd >= 0 && (CFEqual(type, CFSTR("public.jpeg"))
        || CFEqual(type, CFSTR("public.png")) || CFEqual(type, CFSTR("public.tiff")));
    if (fd >= 0 && !downsample && reader->length > 32u * 1024u * 1024u) { status = 3; goto done; }
    size_t primary = CGImageSourceGetPrimaryImageIndex(source);
    if (primary >= CGImageSourceGetCount(source)) goto done;
    properties = CGImageSourceCopyPropertiesAtIndex(source, primary, source_options);
    if (!properties) { status = 6; goto done; }
    int64_t width = 0, height = 0, orientation = 1;
    if (!number(properties, kCGImagePropertyPixelWidth, &width)
        || !number(properties, kCGImagePropertyPixelHeight, &height)) { status = 6; goto done; }
    // Match the Rust header/frame envelope: only file-backed JPEG gets a
    // longer side; retain the previous 12000×12000 total source-pixel ceiling.
    int64_t side_limit = downsample && CFEqual(type, CFSTR("public.jpeg")) ? 16384 : 12000;
    if (width <= 0 || height <= 0 || width > side_limit || height > side_limit
        || (uint64_t)width * (uint64_t)height > 144000000ull
        || (!downsample && (uint64_t)width * (uint64_t)height * 4u > 128u * 1024u * 1024u)) {
        status = 3; goto done;
    }
    if (CFDictionaryContainsKey(properties, kCGImagePropertyOrientation)
        && (!number(properties, kCGImagePropertyOrientation, &orientation)
            || orientation < 1 || orientation > 8)) goto done;
    int64_t maximum = width > height ? width : height;
    if (maximum > edge) maximum = edge; // never upscale small originals
    size = CFNumberCreate(NULL, kCFNumberSInt64Type, &maximum);
    if (!size) goto done;
    const void *keys[] = {downsample ? kCGImageSourceCreateThumbnailFromImageAlways : kCGImageSourceCreateThumbnailFromImageIfAbsent,
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
    if (reader) {
        result->bytes_read = atomic_load(&reader->actual);
        if (atomic_load(&reader->exhausted)) status = 3;
    }
    if (provider) CGDataProviderRelease(provider);
    else if (reader) sh_release_reader(reader);
    if (data) CFRelease(data);
    return status;
}

int sh_imageio_preview(const uint8_t *input, size_t length, uint32_t edge, uint32_t hint,
                      uint8_t *rgba, size_t capacity, SHPreview *result) {
    return sh_preview(input, length, -1, edge, hint, rgba, capacity, result);
}
int sh_imageio_file_preview(int fd, uint32_t edge, uint32_t hint,
                           uint8_t *rgba, size_t capacity, SHPreview *result) {
    return sh_preview(NULL, 0, fd, edge, hint, rgba, capacity, result);
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
