#include "draw.h"

#define SOKOL_IMPL
#define SOKOL_NO_ENTRY
#include <stdint.h>
#include <stdio.h>
#include <sys/utsname.h>

#include "sokol_app.h"
#include "sokol_gfx.h"
#include "sokol_glue.h"
#include "sokol_log.h"

// Mirrored by `corela_gfx::Desc`; keep the field order in sync.
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

// _MOTIF_WM_HINTS tells the window manager which window controls to offer.
// The bit values come from Motif's <Xm/MwmUtil.h>, which is not a dependency.
#define MOTIF_WM_HINTS_ATOM_NAME "_MOTIF_WM_HINTS"
#define MOTIF_HINTS_FUNCTIONS (1UL << 0)
#define MOTIF_FUNC_MOVE (1UL << 2)
#define MOTIF_FUNC_MINIMIZE (1UL << 3)
#define MOTIF_FUNC_CLOSE (1UL << 5)
// X11 stores CARDINAL property items as 32-bit values (passed as longs)
#define X11_PROPERTY_FORMAT_32 32

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
  } motif = {
      .flags = MOTIF_HINTS_FUNCTIONS,
      // no resize or maximize
      .functions = MOTIF_FUNC_MOVE | MOTIF_FUNC_MINIMIZE | MOTIF_FUNC_CLOSE,
  };
  const Atom property = XInternAtom(display, MOTIF_WM_HINTS_ATOM_NAME, False);
  XChangeProperty(display, window, property, property, X11_PROPERTY_FORMAT_32,
                  PropModeReplace, (const unsigned char*)&motif,
                  sizeof(motif) / sizeof(long));
  XFlush(display);
}

static const char* gl_info(GLenum name) {
  const GLubyte* value = glGetString(name);
  return value ? (const char*)value : "Unknown";
}

static void log_system_info(void) {
  struct utsname system;
  if (uname(&system) == 0) {
    fprintf(stderr, "System: %s %s - %s - X11\n", system.sysname,
            system.release, system.machine);
  } else {
    fprintf(stderr, "System: Unknown - X11\n");
  }
  fprintf(stderr, "OpenGL %s - Sokol - Using Device: %s - %s\n",
          gl_info(GL_VERSION), gl_info(GL_VENDOR), gl_info(GL_RENDERER));
  fprintf(stderr, "GLSL %s\n\n", gl_info(GL_SHADING_LANGUAGE_VERSION));
}

static void on_init(void) {
  configure_fixed_window();
  sg_setup(&(sg_desc){
      .environment = sglue_environment(),
      .logger.func = slog_func,
  });
  log_system_info();
  if (g_init) g_init();
}

static void on_event(const sapp_event* e) {
  if (e->type == SAPP_EVENTTYPE_KEY_DOWN && !e->key_repeat && g_key_down)
    g_key_down((int32_t)e->key_code);
}

static void on_cleanup(void) {
  gfx_renderer_cleanup();
  sg_shutdown();
}

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
      // nanovg fills concave paths with stencil-then-cover
      .depth_format = SAPP_PIXELFORMAT_DEPTH_STENCIL,
      .gl = {.major_version = OPENGL_MAJOR_VERSION,
             .minor_version = OPENGL_MINOR_VERSION},
      .logger.func = slog_func,
  });
  return 0;
}

void gfx_quit(void) { sapp_request_quit(); }
