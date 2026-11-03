#include "draw.h"
#include "draw_internal.h"

#include <math.h>

#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>

#include "sokol_app.h"
#define NANOVG_GL3_IMPLEMENTATION
#include "nanovg.h"
#include "nanovg_gl.h"

static NVGcontext* gfx_vg;

void gfx_vector_init(void) {
  gfx_vg = nvgCreateGL3(NVG_ANTIALIAS);
  if (gfx_vg == NULL) g_error("nanovg context creation failed");
}

void gfx_renderer_cleanup(void) { g_clear_pointer(&gfx_vg, nvgDeleteGL3); }

void gfx_vector_frame_begin(void) {
  const gfloat dpi = sapp_dpi_scale();
  nvgBeginFrame(gfx_vg, (gfloat)sapp_width() / dpi, (gfloat)sapp_height() / dpi,
                dpi);
}

void gfx_vector_frame_end(void) { nvgEndFrame(gfx_vg); }

void gfx_set_font(const guint8* data, gsize length) {
  // nanovg takes the length as an int
  g_return_if_fail(length <= G_MAXINT);
  // Rust owns these static bytes for the lifetime of the renderer.
  const gint font =
      nvgCreateFontMem(gfx_vg, "ui", (guchar*)data, (gint)length, 0);
  if (font < 0) g_error("UI font creation failed");
}

#define TEXT_MARGIN 16.0f
#define TEXT_FONT_SIZE 20.0f

// one line of UI text at the left edge of the window
static void draw_ui_text(const gchar* text, gfloat y, gint vertical_align,
                         guint32 color) {
  nvgSave(gfx_vg);
  nvgFontFace(gfx_vg, "ui");
  nvgFontSize(gfx_vg, TEXT_FONT_SIZE);
  nvgTextAlign(gfx_vg, NVG_ALIGN_LEFT | vertical_align);
  nvgFillColor(gfx_vg, nvgRGB(color_channel(color, COLOR_RED_SHIFT),
                              color_channel(color, COLOR_GREEN_SHIFT),
                              color_channel(color, COLOR_BLUE_SHIFT)));
  nvgText(gfx_vg, TEXT_MARGIN, y, text, NULL);
  nvgRestore(gfx_vg);
}

void gfx_draw_header(const gchar* text, guint32 color) {
  draw_ui_text(text, TEXT_MARGIN, NVG_ALIGN_TOP, color);
}

void gfx_draw_footer(const gchar* text, guint32 color) {
  const gfloat height = (gfloat)sapp_height() / sapp_dpi_scale();
  draw_ui_text(text, height - TEXT_MARGIN, NVG_ALIGN_BOTTOM, color);
}

void gfx_draw_icon(const gfx_icon* icon, gfloat x, gfloat y, gfloat dx,
                   gfloat dy, gfloat size, guint32 color) {
  const gfloat width = icon->view_box[VIEW_BOX_WIDTH];
  const gfloat height = icon->view_box[VIEW_BOX_HEIGHT];
  const gfloat scale = size / MAX(width, height);
  nvgSave(gfx_vg);
  nvgTranslate(gfx_vg, x, y);
  nvgRotate(gfx_vg, atan2f(dy, dx));
  nvgScale(gfx_vg, scale, scale);
  nvgTranslate(gfx_vg, -(icon->view_box[VIEW_BOX_MIN_X] + width * 0.5f),
               -(icon->view_box[VIEW_BOX_MIN_Y] + height * 0.5f));
  nvgBeginPath(gfx_vg);
  const gfloat(*p)[2] = icon->points;
  for (gint i = 0; i < icon->verb_count; i++) {
    switch (icon->verbs[i]) {
      case GFX_ICON_MOVE_TO:
        nvgMoveTo(gfx_vg, p[0][0], p[0][1]);
        p += 1;
        break;
      case GFX_ICON_LINE_TO:
        nvgLineTo(gfx_vg, p[0][0], p[0][1]);
        p += 1;
        break;
      case GFX_ICON_BEZIER_TO:
        nvgBezierTo(gfx_vg, p[0][0], p[0][1], p[1][0], p[1][1], p[2][0],
                    p[2][1]);
        p += 3;
        break;
      case GFX_ICON_CLOSE_SOLID:
        nvgClosePath(gfx_vg);
        nvgPathWinding(gfx_vg, NVG_SOLID);
        break;
      case GFX_ICON_CLOSE_HOLE:
        nvgClosePath(gfx_vg);
        nvgPathWinding(gfx_vg, NVG_HOLE);
        break;
    }
  }
  nvgFillColor(gfx_vg, nvgRGB(color_channel(color, COLOR_RED_SHIFT),
                              color_channel(color, COLOR_GREEN_SHIFT),
                              color_channel(color, COLOR_BLUE_SHIFT)));
  nvgFill(gfx_vg);
  nvgRestore(gfx_vg);
}
