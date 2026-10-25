#define SOKOL_IMPL
#define SOKOL_NO_ENTRY
#include <math.h>
#include <stdint.h>

#include "font_data.h"
#include "sokol_app.h"
#include "sokol_gfx.h"
#include "sokol_glue.h"
#include "sokol_log.h"

typedef struct {
  const char* title;
  int32_t width;
  int32_t height;
  void (*init)(void);
  void (*frame)(void);
  void (*cleanup)(void);
  void (*key_down)(int32_t key_code);
} gfx_desc;

static void (*g_init)(void);
static void (*g_key_down)(int32_t);

static void configure_fixed_window(void) {
  Display* display = (Display*)sapp_x11_get_display();
  const Window window = (Window)(uintptr_t)sapp_x11_get_window();
  XSizeHints hints = {0};
  long supplied = 0;
  XGetWMNormalHints(display, window, &hints, &supplied);
  hints.flags |= PMinSize | PMaxSize;
  hints.min_width = hints.max_width = sapp_width();
  hints.min_height = hints.max_height = sapp_height();
  XSetWMNormalHints(display, window, &hints);

  const struct {
    unsigned long flags, functions, decorations;
    long input_mode;
    unsigned long status;
  } motif = {.flags = 1, .functions = (1UL << 2) | (1UL << 3) | (1UL << 5)};
  const Atom property = XInternAtom(display, "_MOTIF_WM_HINTS", False);
  XChangeProperty(display, window, property, property, 32, PropModeReplace,
                  (const unsigned char*)&motif, 5);
  XFlush(display);
}

static void on_init(void) {
  configure_fixed_window();
  sg_setup(&(sg_desc){
      .environment = sglue_environment(),
      .logger.func = slog_func,
  });
  if (g_init) g_init();
}

static void on_event(const sapp_event* e) {
  if (e->type == SAPP_EVENTTYPE_KEY_DOWN && !e->key_repeat && g_key_down)
    g_key_down((int32_t)e->key_code);
}

static void on_cleanup(void) { sg_shutdown(); }

static void (*g_user_cleanup)(void);
static void on_cleanup_wrapped(void) {
  if (g_user_cleanup) g_user_cleanup();
  on_cleanup();
}

int32_t gfx_run(const gfx_desc* d) {
  g_init = d->init;
  g_key_down = d->key_down;
  g_user_cleanup = d->cleanup;
  sapp_run(&(sapp_desc){
      .init_cb = on_init,
      .frame_cb = d->frame,
      .cleanup_cb = on_cleanup_wrapped,
      .event_cb = on_event,
      .width = d->width,
      .height = d->height,
      .window_title = d->title,
      .gl = {.major_version = 3, .minor_version = 3},
      .logger.func = slog_func,
  });
  return 0;
}

void gfx_quit(void) { sapp_request_quit(); }

static const char* const VS_SRC =
    "#version 330\n"
    "layout(location = 0) in vec2 a_pos;\n"
    "layout(location = 1) in vec3 a_color;\n"
    "out vec3 v_color;\n"
    "void main() {\n"
    "  v_color = a_color;\n"
    "  gl_Position = vec4(a_pos, 0.0, 1.0);\n"
    "}\n";

static const char* const FS_SRC =
    "#version 330\n"
    "in vec3 v_color;\n"
    "out vec4 frag_color;\n"
    "void main() {\n"
    "  frag_color = vec4(v_color, 1.0);\n"
    "}\n";

static sg_pipeline g_pip;
static sg_buffer g_shape_buffer;
static float g_shape_vertices[262144][5];
static int g_shape_vertex_count;
static sg_pass_action g_pass_action;

void gfx_renderer_init(float clear_r, float clear_g, float clear_b) {
  g_shape_buffer = sg_make_buffer(&(sg_buffer_desc){
      .size = 262144 * 5 * sizeof(float),
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
              [0].format = SG_VERTEXFORMAT_FLOAT2,
              [1].format = SG_VERTEXFORMAT_FLOAT3,
          },
      .label = "shape-pipeline",
  });
  g_pass_action = (sg_pass_action){
      .colors[0] = {.load_action = SG_LOADACTION_CLEAR,
                    .clear_value = {clear_r, clear_g, clear_b, 1.0f}},
  };
}

void gfx_frame_begin(void) { g_shape_vertex_count = 0; }

static void draw_polygon(const float points[][2], int count, uint32_t color) {
  const float dpi = sapp_dpi_scale();
  const float window_width = (float)sapp_width() / dpi;
  const float window_height = (float)sapp_height() / dpi;
  if (window_width <= 0 || window_height <= 0) return;
  const int vertex_count = (count - 2) * 3;
  if (g_shape_vertex_count + vertex_count > 262144) {
    SOKOL_ASSERT(false && "shape vertex capacity exceeded");
    return;
  }
  const float r = ((color >> 16) & 255) / 255.0f;
  const float g = ((color >> 8) & 255) / 255.0f;
  const float b = (color & 255) / 255.0f;
  for (int i = 1; i < count - 1; i++) {
    const int indices[3] = {0, i, i + 1};
    for (int j = 0; j < 3; j++) {
      const float* point = points[indices[j]];
      float* vertex = g_shape_vertices[g_shape_vertex_count++];
      vertex[0] = 2.0f * point[0] / window_width - 1.0f;
      vertex[1] = 1.0f - 2.0f * point[1] / window_height;
      vertex[2] = r;
      vertex[3] = g;
      vertex[4] = b;
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
  const float half_height = radius * 0.8660254037844386f;  // sqrt(3) / 2
  const float points[6][2] = {
      {x - radius, y},
      {x - radius * 0.5f, y + half_height},
      {x + radius * 0.5f, y + half_height},
      {x + radius, y},
      {x + radius * 0.5f, y - half_height},
      {x - radius * 0.5f, y - half_height},
  };
  draw_polygon(points, 6, color);
}

void gfx_draw_label(int32_t index, float x, float y, uint32_t background,
                    bool selected) {
  if (index < 0 || index >= (int)(sizeof(LABELS) / sizeof(LABELS[0]))) return;
  const label_info label = LABELS[index];
  const uint32_t foreground = 0x232634;
  const float opacity = selected ? 1.0f : 0.16f;
  for (int i = label.start; i < label.start + label.count; i++) {
    const label_span span = LABEL_SPANS[i];
    const float alpha = opacity * span.coverage / 255.0f;
    uint32_t color = 0;
    for (int shift = 0; shift <= 16; shift += 8) {
      const float bg = (background >> shift) & 255;
      const float fg = (foreground >> shift) & 255;
      color |= (uint32_t)(bg + (fg - bg) * alpha + 0.5f) << shift;
    }
    gfx_draw_rectangle(roundf(x - label.width * 0.5f) + span.x,
                       roundf(y - label.height * 0.5f) + span.y, span.width,
                       1.0f, color);
  }
}

void gfx_draw_circle(float x, float y, float radius, uint32_t color) {
  float points[64][2];
  for (int i = 0; i < 64; i++) {
    const float angle = -6.28318530718f * i / 64.0f;
    points[i][0] = x + cosf(angle) * radius;
    points[i][1] = y + sinf(angle) * radius;
  }
  draw_polygon(points, 64, color);
}

void gfx_draw_double_chevron_right(float x, float y, float dx, float dy,
                                   uint32_t color) {
  const float length = sqrtf(dx * dx + dy * dy);
  if (length == 0.0f) {
    return;
  }
  const float cosine = dx / length;
  const float sine = dy / length;
  for (int i = 0; i < 2; i++) {
    const float left = -16.0f + i * 8.5f;
    const float top = -16.0f;
    float upper[4][2] = {
        {left + 7.5146f, top + 6.1005f},
        {left + 6.1006f, top + 7.5146f},
        {left + 14.5859f, top + 15.9999f},
        {left + 17.414f, top + 15.9999f},
    };
    float lower[4][2] = {
        {left + 17.414f, top + 15.9999f},
        {left + 14.5859f, top + 15.9999f},
        {left + 6.1005f, top + 24.4853f},
        {left + 7.5146f, top + 25.8994f},
    };
    for (int j = 0; j < 4; j++) {
      const float ux = upper[j][0], uy = upper[j][1];
      const float lx = lower[j][0], ly = lower[j][1];
      upper[j][0] = x + ux * cosine - uy * sine;
      upper[j][1] = y + ux * sine + uy * cosine;
      lower[j][0] = x + lx * cosine - ly * sine;
      lower[j][1] = y + lx * sine + ly * cosine;
    }
    draw_polygon(upper, 4, color);
    draw_polygon(lower, 4, color);
  }
}

void gfx_frame_end(void) {
  if (g_shape_vertex_count > 0) {
    sg_write_buffer_transient(&(sg_write_buffer_desc){
        .src.data = {.ptr = g_shape_vertices,
                     .size = (size_t)g_shape_vertex_count * 5 * sizeof(float)},
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
  sg_end_pass();
  sg_commit();
}
