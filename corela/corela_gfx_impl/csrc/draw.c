#include "draw.h"

#include <math.h>

#define GL_GLEXT_PROTOTYPES
#include <GL/gl.h>

#include "font_data.h"
#include "sokol_app.h"
#include "sokol_gfx.h"
#include "sokol_glue.h"
#define NANOVG_GL3_IMPLEMENTATION
#include "nanovg.h"
#include "nanovg_gl.h"

// colors are packed as 0xRRGGBB
enum {
  COLOR_RED_SHIFT = 16,
  COLOR_GREEN_SHIFT = 8,
  COLOR_BLUE_SHIFT = 0,
  COLOR_CHANNEL_BITS = 8,
  COLOR_CHANNEL_MAX = 255,
};
#define OPAQUE_ALPHA 1.0f

#define FULL_TURN_RADIANS ((gfloat)(2.0 * G_PI))
enum { CIRCLE_SEGMENT_COUNT = 64 };
enum { VERTICES_PER_TRIANGLE = 3 };

// label glyphs are pre-rendered as one-pixel-tall spans of anti-aliased
// coverage, blended between the tile color and the label text color
#define LABEL_SELECTED_OPACITY 1.0f
#define LABEL_UNSELECTED_OPACITY 0.16f
#define LABEL_SPAN_HEIGHT 1.0f
#define LABEL_COVERAGE_MAX 255.0f

static NVGcontext* gfx_vg;

void gfx_renderer_cleanup(void) { g_clear_pointer(&gfx_vg, nvgDeleteGL3); }

static guint8 color_channel(guint32 color, gint shift) {
  return (color >> shift) & COLOR_CHANNEL_MAX;
}

// attribute locations match the layout(location = N) inputs of VS_SRC
enum { SHAPE_ATTR_POSITION = 0, SHAPE_ATTR_COLOR = 1 };

static const gchar* const VS_SRC = GLSL_VERSION_DIRECTIVE
    "layout(location = 0) in vec2 a_pos;\n"
    "layout(location = 1) in vec3 a_color;\n"
    "out vec3 v_color;\n"
    "void main() {\n"
    "  v_color = a_color;\n"
    "  gl_Position = vec4(a_pos, 0.0, 1.0);\n"
    "}\n";

static const gchar* const FS_SRC = GLSL_VERSION_DIRECTIVE
    "in vec3 v_color;\n"
    "out vec4 frag_color;\n"
    "void main() {\n"
    "  frag_color = vec4(v_color, 1.0);\n"
    "}\n";

typedef struct {
  gfloat x, y;  // normalized device coordinates
  gfloat r, g, b;
} shape_vertex;

static sg_pipeline gfx_pip;
static sg_buffer gfx_shape_buffer;
enum { MAX_SHAPE_VERTICES = 256 * 1024 };
static shape_vertex gfx_shape_vertices[MAX_SHAPE_VERTICES];
static gint gfx_shape_vertex_count;
static sg_pass_action gfx_pass_action;

void gfx_renderer_init(gfloat clear_r, gfloat clear_g, gfloat clear_b) {
  gfx_shape_buffer = sg_make_buffer(&(sg_buffer_desc){
      .size = sizeof(gfx_shape_vertices),
      .usage.write_transient = true,
      .label = "shape-vertices",
  });
  sg_shader shd = sg_make_shader(&(sg_shader_desc){
      .vertex_func.source = VS_SRC,
      .fragment_func.source = FS_SRC,
      .label = "shape-shader",
  });
  gfx_pip = sg_make_pipeline(&(sg_pipeline_desc){
      .shader = shd,
      .layout.attrs =
          {
              [SHAPE_ATTR_POSITION].format = SG_VERTEXFORMAT_FLOAT2,
              [SHAPE_ATTR_COLOR].format = SG_VERTEXFORMAT_FLOAT3,
          },
      .label = "shape-pipeline",
  });
  gfx_pass_action = (sg_pass_action){
      .colors[0] = {.load_action = SG_LOADACTION_CLEAR,
                    .clear_value = {clear_r, clear_g, clear_b, OPAQUE_ALPHA}},
  };
  gfx_vg = nvgCreateGL3(NVG_ANTIALIAS);
  if (gfx_vg == NULL) g_error("nanovg context creation failed");
}

void gfx_frame_begin(void) {
  gfx_shape_vertex_count = 0;
  const gfloat dpi = sapp_dpi_scale();
  nvgBeginFrame(gfx_vg, (gfloat)sapp_width() / dpi, (gfloat)sapp_height() / dpi,
                dpi);
}

// Triangulates a convex polygon as a fan around points[0].
static void draw_polygon(const gfloat points[][2], gint count, guint32 color) {
  const gfloat dpi = sapp_dpi_scale();
  const gfloat window_width = (gfloat)sapp_width() / dpi;
  const gfloat window_height = (gfloat)sapp_height() / dpi;
  if (window_width <= 0 || window_height <= 0) return;
  const gint triangle_count = count - 2;
  const gint vertex_count = triangle_count * VERTICES_PER_TRIANGLE;
  g_return_if_fail(gfx_shape_vertex_count + vertex_count <= MAX_SHAPE_VERTICES);
  const gfloat r =
      color_channel(color, COLOR_RED_SHIFT) / (gfloat)COLOR_CHANNEL_MAX;
  const gfloat g =
      color_channel(color, COLOR_GREEN_SHIFT) / (gfloat)COLOR_CHANNEL_MAX;
  const gfloat b =
      color_channel(color, COLOR_BLUE_SHIFT) / (gfloat)COLOR_CHANNEL_MAX;
  for (gint i = 1; i < count - 1; i++) {
    const gint indices[VERTICES_PER_TRIANGLE] = {0, i, i + 1};
    for (gint j = 0; j < VERTICES_PER_TRIANGLE; j++) {
      const gfloat* point = points[indices[j]];
      shape_vertex* vertex = &gfx_shape_vertices[gfx_shape_vertex_count++];
      // pixels (y down) -> normalized device coordinates (-1..1, y up)
      vertex->x = 2.0f * point[0] / window_width - 1.0f;
      vertex->y = 1.0f - 2.0f * point[1] / window_height;
      vertex->r = r;
      vertex->g = g;
      vertex->b = b;
    }
  }
}

void gfx_draw_rectangle(gfloat x, gfloat y, gfloat width, gfloat height,
                        guint32 color) {
  const gfloat points[4][2] = {
      {x, y},
      {x, y + height},
      {x + width, y + height},
      {x + width, y},
  };
  draw_polygon(points, 4, color);
}

void gfx_draw_hexagon(gfloat x, gfloat y, gfloat radius, guint32 color) {
  const gfloat half_height = radius * HEX_HALF_HEIGHT_TO_RADIUS_RATIO;
  const gfloat half_edge = radius * HEX_HALF_EDGE_TO_RADIUS_RATIO;
  const gfloat points[HEX_CORNER_COUNT][2] = {
      {x - radius, y},
      {x - half_edge, y + half_height},
      {x + half_edge, y + half_height},
      {x + radius, y},
      {x + half_edge, y - half_height},
      {x - half_edge, y - half_height},
  };
  draw_polygon(points, HEX_CORNER_COUNT, color);
}

void gfx_set_clear(gfloat clear_r, gfloat clear_g, gfloat clear_b) {
  gfx_pass_action.colors[0].clear_value =
      (sg_color){clear_r, clear_g, clear_b, OPAQUE_ALPHA};
}

void gfx_draw_label(gint32 index, gfloat x, gfloat y, guint32 background,
                    guint32 text, bool selected) {
  if (index < 0 || index >= (gint)G_N_ELEMENTS(LABELS)) return;
  const label_info label = LABELS[index];
  const gfloat opacity =
      selected ? LABEL_SELECTED_OPACITY : LABEL_UNSELECTED_OPACITY;
  for (gint i = label.start; i < label.start + label.count; i++) {
    const label_span span = LABEL_SPANS[i];
    const gfloat alpha = opacity * span.coverage / LABEL_COVERAGE_MAX;
    guint32 color = 0;
    for (gint shift = COLOR_BLUE_SHIFT; shift <= COLOR_RED_SHIFT;
         shift += COLOR_CHANNEL_BITS) {
      const gfloat bg = color_channel(background, shift);
      const gfloat fg = color_channel(text, shift);
      color |= (guint32)roundf(bg + (fg - bg) * alpha) << shift;
    }
    gfx_draw_rectangle(roundf(x - label.width * 0.5f) + span.x,
                       roundf(y - label.height * 0.5f) + span.y, span.width,
                       LABEL_SPAN_HEIGHT, color);
  }
}

void gfx_set_font(const guint8* data, gsize length) {
  // nanovg takes the length as an int
  g_return_if_fail(length <= G_MAXINT);
  // Rust owns these static bytes for the lifetime of the renderer.
  const gint font =
      nvgCreateFontMem(gfx_vg, "ui", (guchar*)data, (gint)length, 0);
  if (font < 0) g_error("UI font creation failed");
}

void gfx_draw_footer(const gchar* text, guint32 color) {
  const gfloat margin = 16.0f;
  const gfloat height = (gfloat)sapp_height() / sapp_dpi_scale();
  nvgSave(gfx_vg);
  nvgFontFace(gfx_vg, "ui");
  nvgFontSize(gfx_vg, 20.0f);
  nvgTextAlign(gfx_vg, NVG_ALIGN_LEFT | NVG_ALIGN_BOTTOM);
  nvgFillColor(gfx_vg, nvgRGB(color_channel(color, COLOR_RED_SHIFT),
                              color_channel(color, COLOR_GREEN_SHIFT),
                              color_channel(color, COLOR_BLUE_SHIFT)));
  nvgText(gfx_vg, margin, height - margin, text, NULL);
  nvgRestore(gfx_vg);
}

void gfx_draw_circle(gfloat x, gfloat y, gfloat radius, guint32 color) {
  gfloat points[CIRCLE_SEGMENT_COUNT][2];
  for (gint i = 0; i < CIRCLE_SEGMENT_COUNT; i++) {
    // negative angles run counter-clockwise on screen, like gfx_draw_hexagon
    const gfloat angle = -FULL_TURN_RADIANS * i / CIRCLE_SEGMENT_COUNT;
    points[i][0] = x + cosf(angle) * radius;
    points[i][1] = y + sinf(angle) * radius;
  }
  draw_polygon(points, CIRCLE_SEGMENT_COUNT, color);
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

void gfx_frame_end(void) {
  if (gfx_shape_vertex_count > 0) {
    sg_write_buffer_transient(&(sg_write_buffer_desc){
        .src.data = {.ptr = gfx_shape_vertices,
                     .size =
                         (gsize)gfx_shape_vertex_count * sizeof(shape_vertex)},
        .dst.buffer = gfx_shape_buffer,
    });
  }
  sg_begin_pass(
      &(sg_pass){.action = gfx_pass_action, .swapchain = sglue_swapchain()});
  sg_apply_pipeline(gfx_pip);
  if (gfx_shape_vertex_count > 0) {
    const sg_bindings bindings = {.vertex_buffers[0] = gfx_shape_buffer};
    sg_apply_bindings(&bindings);
    sg_draw(0, gfx_shape_vertex_count, 1);
  }
  // nanovg issues raw GL calls on top of the sokol shapes
  nvgEndFrame(gfx_vg);
  sg_reset_state_cache();
  sg_end_pass();
  sg_commit();
}

gfx_geometry gfx_get_geometry(void) {
  return (gfx_geometry){
      .cell_marker_outer_radius = CELL_MARKER_OUTER_RADIUS,
      .cell_marker_inner_radius = CELL_MARKER_INNER_RADIUS,
      .chevron_size = CHEVRON_SIZE,
      .hex_radius = HEX_RADIUS,
      .hex_size = HEX_SIZE,
      .hex_tile_inset = HEX_TILE_INSET,
  };
}
