#include "draw.h"

#include <assert.h>
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

#define ARRAY_COUNT(array) (sizeof(array) / sizeof((array)[0]))

// colors are packed as 0xRRGGBB
enum {
  COLOR_RED_SHIFT = 16,
  COLOR_GREEN_SHIFT = 8,
  COLOR_BLUE_SHIFT = 0,
  COLOR_CHANNEL_BITS = 8,
  COLOR_CHANNEL_MAX = 255,
};
#define OPAQUE_ALPHA 1.0f

#define FULL_TURN_RADIANS 6.28318530718f
enum { CIRCLE_SEGMENT_COUNT = 64 };
enum { VERTICES_PER_TRIANGLE = 3 };

// label glyphs are pre-rendered as one-pixel-tall spans of anti-aliased
// coverage, blended between the tile color and LABEL_TEXT_COLOR
#define LABEL_TEXT_COLOR 0x232634
#define LABEL_SELECTED_OPACITY 1.0f
#define LABEL_UNSELECTED_OPACITY 0.16f
#define LABEL_SPAN_HEIGHT 1.0f
#define LABEL_COVERAGE_MAX 255.0f

static NVGcontext* g_vg;

void gfx_renderer_cleanup(void) {
  if (g_vg) nvgDeleteGL3(g_vg);
  g_vg = NULL;
}

static uint8_t color_channel(uint32_t color, int shift) {
  return (color >> shift) & COLOR_CHANNEL_MAX;
}

// attribute locations match the layout(location = N) inputs of VS_SRC
enum { SHAPE_ATTR_POSITION = 0, SHAPE_ATTR_COLOR = 1 };

static const char* const VS_SRC = GLSL_VERSION_DIRECTIVE
    "layout(location = 0) in vec2 a_pos;\n"
    "layout(location = 1) in vec3 a_color;\n"
    "out vec3 v_color;\n"
    "void main() {\n"
    "  v_color = a_color;\n"
    "  gl_Position = vec4(a_pos, 0.0, 1.0);\n"
    "}\n";

static const char* const FS_SRC = GLSL_VERSION_DIRECTIVE
    "in vec3 v_color;\n"
    "out vec4 frag_color;\n"
    "void main() {\n"
    "  frag_color = vec4(v_color, 1.0);\n"
    "}\n";

typedef struct {
  float x, y;  // normalized device coordinates
  float r, g, b;
} shape_vertex;

static sg_pipeline g_pip;
static sg_buffer g_shape_buffer;
enum { MAX_SHAPE_VERTICES = 256 * 1024 };
static shape_vertex g_shape_vertices[MAX_SHAPE_VERTICES];
static int g_shape_vertex_count;
static sg_pass_action g_pass_action;

void gfx_renderer_init(float clear_r, float clear_g, float clear_b) {
  g_shape_buffer = sg_make_buffer(&(sg_buffer_desc){
      .size = sizeof(g_shape_vertices),
      .usage.write_transient = true,
      .label = "shape-vertices",
  });
  sg_shader shd = sg_make_shader(&(sg_shader_desc){
      .vertex_func.source = VS_SRC,
      .fragment_func.source = FS_SRC,
      .label = "shape-shader",
  });
  g_pip = sg_make_pipeline(&(sg_pipeline_desc){
      .shader = shd,
      .layout.attrs =
          {
              [SHAPE_ATTR_POSITION].format = SG_VERTEXFORMAT_FLOAT2,
              [SHAPE_ATTR_COLOR].format = SG_VERTEXFORMAT_FLOAT3,
          },
      .label = "shape-pipeline",
  });
  g_pass_action = (sg_pass_action){
      .colors[0] = {.load_action = SG_LOADACTION_CLEAR,
                    .clear_value = {clear_r, clear_g, clear_b, OPAQUE_ALPHA}},
  };
  g_vg = nvgCreateGL3(NVG_ANTIALIAS);
  assert(g_vg && "nanovg context creation failed");
}

void gfx_frame_begin(void) {
  g_shape_vertex_count = 0;
  const float dpi = sapp_dpi_scale();
  nvgBeginFrame(g_vg, (float)sapp_width() / dpi, (float)sapp_height() / dpi,
                dpi);
}

// Triangulates a convex polygon as a fan around points[0].
static void draw_polygon(const float points[][2], int count, uint32_t color) {
  const float dpi = sapp_dpi_scale();
  const float window_width = (float)sapp_width() / dpi;
  const float window_height = (float)sapp_height() / dpi;
  if (window_width <= 0 || window_height <= 0) return;
  const int triangle_count = count - 2;
  const int vertex_count = triangle_count * VERTICES_PER_TRIANGLE;
  if (g_shape_vertex_count + vertex_count > MAX_SHAPE_VERTICES) {
    assert(false && "shape vertex capacity exceeded");
    return;
  }
  const float r =
      color_channel(color, COLOR_RED_SHIFT) / (float)COLOR_CHANNEL_MAX;
  const float g =
      color_channel(color, COLOR_GREEN_SHIFT) / (float)COLOR_CHANNEL_MAX;
  const float b =
      color_channel(color, COLOR_BLUE_SHIFT) / (float)COLOR_CHANNEL_MAX;
  for (int i = 1; i < count - 1; i++) {
    const int indices[VERTICES_PER_TRIANGLE] = {0, i, i + 1};
    for (int j = 0; j < VERTICES_PER_TRIANGLE; j++) {
      const float* point = points[indices[j]];
      shape_vertex* vertex = &g_shape_vertices[g_shape_vertex_count++];
      // pixels (y down) -> normalized device coordinates (-1..1, y up)
      vertex->x = 2.0f * point[0] / window_width - 1.0f;
      vertex->y = 1.0f - 2.0f * point[1] / window_height;
      vertex->r = r;
      vertex->g = g;
      vertex->b = b;
    }
  }
}

void gfx_draw_rectangle(float x, float y, float width, float height,
                        uint32_t color) {
  const float points[4][2] = {
      {x, y},
      {x, y + height},
      {x + width, y + height},
      {x + width, y},
  };
  draw_polygon(points, 4, color);
}

void gfx_draw_hexagon(float x, float y, float radius, uint32_t color) {
  const float half_height = radius * HEX_HALF_HEIGHT_TO_RADIUS_RATIO;
  const float half_edge = radius * HEX_HALF_EDGE_TO_RADIUS_RATIO;
  const float points[HEX_CORNER_COUNT][2] = {
      {x - radius, y},
      {x - half_edge, y + half_height},
      {x + half_edge, y + half_height},
      {x + radius, y},
      {x + half_edge, y - half_height},
      {x - half_edge, y - half_height},
  };
  draw_polygon(points, HEX_CORNER_COUNT, color);
}

void gfx_draw_label(int32_t index, float x, float y, uint32_t background,
                    bool selected) {
  if (index < 0 || index >= (int)ARRAY_COUNT(LABELS)) return;
  const label_info label = LABELS[index];
  const float opacity =
      selected ? LABEL_SELECTED_OPACITY : LABEL_UNSELECTED_OPACITY;
  for (int i = label.start; i < label.start + label.count; i++) {
    const label_span span = LABEL_SPANS[i];
    const float alpha = opacity * span.coverage / LABEL_COVERAGE_MAX;
    uint32_t color = 0;
    for (int shift = COLOR_BLUE_SHIFT; shift <= COLOR_RED_SHIFT;
         shift += COLOR_CHANNEL_BITS) {
      const float bg = color_channel(background, shift);
      const float fg = color_channel(LABEL_TEXT_COLOR, shift);
      color |= (uint32_t)roundf(bg + (fg - bg) * alpha) << shift;
    }
    gfx_draw_rectangle(roundf(x - label.width * 0.5f) + span.x,
                       roundf(y - label.height * 0.5f) + span.y, span.width,
                       LABEL_SPAN_HEIGHT, color);
  }
}

void gfx_draw_circle(float x, float y, float radius, uint32_t color) {
  float points[CIRCLE_SEGMENT_COUNT][2];
  for (int i = 0; i < CIRCLE_SEGMENT_COUNT; i++) {
    // negative angles run counter-clockwise on screen, like gfx_draw_hexagon
    const float angle = -FULL_TURN_RADIANS * i / CIRCLE_SEGMENT_COUNT;
    points[i][0] = x + cosf(angle) * radius;
    points[i][1] = y + sinf(angle) * radius;
  }
  draw_polygon(points, CIRCLE_SEGMENT_COUNT, color);
}

void gfx_draw_icon(const gfx_icon* icon, float x, float y, float dx, float dy,
                   float size, uint32_t color) {
  const float width = icon->view_box[VIEW_BOX_WIDTH];
  const float height = icon->view_box[VIEW_BOX_HEIGHT];
  const float scale = size / fmaxf(width, height);
  nvgSave(g_vg);
  nvgTranslate(g_vg, x, y);
  nvgRotate(g_vg, atan2f(dy, dx));
  nvgScale(g_vg, scale, scale);
  nvgTranslate(g_vg, -(icon->view_box[VIEW_BOX_MIN_X] + width * 0.5f),
               -(icon->view_box[VIEW_BOX_MIN_Y] + height * 0.5f));
  nvgBeginPath(g_vg);
  const float(*p)[2] = icon->points;
  for (int i = 0; i < icon->verb_count; i++) {
    switch (icon->verbs[i]) {
      case GFX_ICON_MOVE_TO:
        nvgMoveTo(g_vg, p[0][0], p[0][1]);
        p += 1;
        break;
      case GFX_ICON_LINE_TO:
        nvgLineTo(g_vg, p[0][0], p[0][1]);
        p += 1;
        break;
      case GFX_ICON_BEZIER_TO:
        nvgBezierTo(g_vg, p[0][0], p[0][1], p[1][0], p[1][1], p[2][0], p[2][1]);
        p += 3;
        break;
      case GFX_ICON_CLOSE_SOLID:
        nvgClosePath(g_vg);
        nvgPathWinding(g_vg, NVG_SOLID);
        break;
      case GFX_ICON_CLOSE_HOLE:
        nvgClosePath(g_vg);
        nvgPathWinding(g_vg, NVG_HOLE);
        break;
    }
  }
  nvgFillColor(g_vg, nvgRGB(color_channel(color, COLOR_RED_SHIFT),
                            color_channel(color, COLOR_GREEN_SHIFT),
                            color_channel(color, COLOR_BLUE_SHIFT)));
  nvgFill(g_vg);
  nvgRestore(g_vg);
}

void gfx_frame_end(void) {
  if (g_shape_vertex_count > 0) {
    sg_write_buffer_transient(&(sg_write_buffer_desc){
        .src.data = {.ptr = g_shape_vertices,
                     .size =
                         (size_t)g_shape_vertex_count * sizeof(shape_vertex)},
        .dst.buffer = g_shape_buffer,
    });
  }
  sg_begin_pass(
      &(sg_pass){.action = g_pass_action, .swapchain = sglue_swapchain()});
  sg_apply_pipeline(g_pip);
  if (g_shape_vertex_count > 0) {
    const sg_bindings bindings = {.vertex_buffers[0] = g_shape_buffer};
    sg_apply_bindings(&bindings);
    sg_draw(0, g_shape_vertex_count, 1);
  }
  // nanovg issues raw GL calls on top of the sokol shapes
  nvgEndFrame(g_vg);
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
