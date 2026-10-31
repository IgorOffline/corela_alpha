#ifndef CORELA_DRAW_INTERNAL_H
#define CORELA_DRAW_INTERNAL_H

#include <glib.h>

// Internal helpers shared by the shape and vector renderers.
// colors are packed as 0xRRGGBB
enum {
  COLOR_RED_SHIFT = 16,
  COLOR_GREEN_SHIFT = 8,
  COLOR_BLUE_SHIFT = 0,
  COLOR_CHANNEL_BITS = 8,
  COLOR_CHANNEL_MAX = 255,
};

static inline guint8 color_channel(guint32 color, gint shift) {
  return (color >> shift) & COLOR_CHANNEL_MAX;
}

// NanoVG owns its context; draw.c controls its place in the frame lifecycle.
void gfx_vector_init(void);
void gfx_vector_frame_begin(void);
void gfx_vector_frame_end(void);

#endif
