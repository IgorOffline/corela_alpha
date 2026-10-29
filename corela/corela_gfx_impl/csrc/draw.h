#ifndef CORELA_DRAW_H
#define CORELA_DRAW_H

// `bool` (not gboolean) where Rust passes its own one-byte `bool`
#include <glib.h>
#include <stdbool.h>

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
  const guint8* verbs;
  gint32 verb_count;
  // one point per move/line, three per bezier, none per close
  const gfloat (*points)[2];
  gfloat view_box[VIEW_BOX_COUNT];
} gfx_icon;

void gfx_renderer_init(gfloat clear_r, gfloat clear_g, gfloat clear_b);
void gfx_renderer_cleanup(void);
void gfx_frame_begin(void);
void gfx_frame_end(void);
void gfx_draw_rectangle(gfloat x, gfloat y, gfloat width, gfloat height,
                        guint32 color);
void gfx_draw_hexagon(gfloat x, gfloat y, gfloat radius, guint32 color);
void gfx_set_clear(gfloat clear_r, gfloat clear_g, gfloat clear_b);
void gfx_draw_label(gint32 index, gfloat x, gfloat y, guint32 background,
                    guint32 text, bool selected);
void gfx_set_font(const guint8* data, gint32 length);
void gfx_draw_footer(const gchar* text, guint32 color);
void gfx_draw_circle(gfloat x, gfloat y, gfloat radius, guint32 color);
void gfx_draw_icon(const gfx_icon* icon, gfloat x, gfloat y, gfloat dx,
                   gfloat dy, gfloat size, guint32 color);
gfx_geometry gfx_get_geometry(void);

#endif
