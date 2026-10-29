#ifndef CORELA_DRAW_H
#define CORELA_DRAW_H

#include <stdbool.h>
#include <stdint.h>

#include "geometry.h"

// The shape shaders and the sokol_app GL context must agree on this version.
#define OPENGL_MAJOR_VERSION 3
#define OPENGL_MINOR_VERSION 3
#define GLSL_VERSION_DIRECTIVE "#version 330\n"

// Mirrored by `corela_gfx::icon::Verb`; keep the order in sync.
enum {
  GFX_ICON_MOVE_TO,
  GFX_ICON_LINE_TO,
  GFX_ICON_BEZIER_TO,
  GFX_ICON_CLOSE_SOLID,
  GFX_ICON_CLOSE_HOLE,
};

// indices into gfx_icon.view_box, in SVG viewBox order
enum {
  VIEW_BOX_MIN_X,
  VIEW_BOX_MIN_Y,
  VIEW_BOX_WIDTH,
  VIEW_BOX_HEIGHT,
  VIEW_BOX_COUNT,
};

// Mirrored by `corela_gfx::icon::IconDesc`; keep the field order in sync.
typedef struct {
  const uint8_t* verbs;
  int32_t verb_count;
  // one point per move/line, three per bezier, none per close
  const float (*points)[2];
  float view_box[VIEW_BOX_COUNT];
} gfx_icon;

void gfx_renderer_init(float clear_r, float clear_g, float clear_b);
void gfx_renderer_cleanup(void);
void gfx_frame_begin(void);
void gfx_frame_end(void);
void gfx_draw_rectangle(float x, float y, float width, float height,
                        uint32_t color);
void gfx_draw_hexagon(float x, float y, float radius, uint32_t color);
void gfx_set_clear(float clear_r, float clear_g, float clear_b);
void gfx_draw_label(int32_t index, float x, float y, uint32_t background,
                    uint32_t text, bool selected);
void gfx_set_font(const uint8_t* data, int32_t length);
void gfx_draw_footer(const char* text, uint32_t color);
void gfx_draw_circle(float x, float y, float radius, uint32_t color);
void gfx_draw_icon(const gfx_icon* icon, float x, float y, float dx, float dy,
                   float size, uint32_t color);
gfx_geometry gfx_get_geometry(void);

#endif
