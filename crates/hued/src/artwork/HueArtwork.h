#pragma once

#include <stdint.h>
#include <stddef.h>

typedef struct HueArtworkResult {
    int ok;
    uint16_t width;
    uint16_t height;
    size_t len;
    uint8_t *data;
} HueArtworkResult;

HueArtworkResult hue_artwork_process(const char *path, uint16_t size);
void hue_artwork_free(uint8_t *ptr, size_t len);
