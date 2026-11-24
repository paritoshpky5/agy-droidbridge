#include <stdio.h>

/* The ARM64 musl loader uses 64-bit offsets for the base stdio functions. */
FILE *fopen64(const char *path, const char *mode) {
    return fopen(path, mode);
}

int fseeko64(FILE *stream, off64_t offset, int whence) {
    return fseeko(stream, (off_t)offset, whence);
}

off64_t ftello64(FILE *stream) {
    return ftello(stream);
}
