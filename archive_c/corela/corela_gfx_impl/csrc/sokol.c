#include "draw.h"

#define SOKOL_IMPL
#define SOKOL_NO_ENTRY
#include <sys/utsname.h>

#include "sokol_app.h"
#include "sokol_gfx.h"
#include "sokol_glue.h"
#include "sokol_log.h"

// Mirrored by `corela_gfx::Desc`; keep the field order in sync.
typedef struct {
  const gchar* title;
  gint32 width;
  gint32 height;
  void (*init)(void);
  void (*frame)(void);
  void (*cleanup)(void);
  void (*key_down)(gint32 key_code);
} gfx_desc;

static void (*gfx_init)(void);
static void (*gfx_key_down)(gint32);

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
  const Window window = (Window)(guintptr)sapp_x11_get_window();
  XSizeHints hints = {0};
  glong supplied = 0;
  XGetWMNormalHints(display, window, &hints, &supplied);
  hints.flags |= PMinSize | PMaxSize;
  hints.min_width = hints.max_width = sapp_width();
  hints.min_height = hints.max_height = sapp_height();
  XSetWMNormalHints(display, window, &hints);

  const struct {
    gulong flags, functions, decorations;
    glong input_mode;
    gulong status;
  } motif = {
      .flags = MOTIF_HINTS_FUNCTIONS,
      // no resize or maximize
      .functions = MOTIF_FUNC_MOVE | MOTIF_FUNC_MINIMIZE | MOTIF_FUNC_CLOSE,
  };
  const Atom property = XInternAtom(display, MOTIF_WM_HINTS_ATOM_NAME, False);
  XChangeProperty(display, window, property, property, X11_PROPERTY_FORMAT_32,
                  PropModeReplace, (const guchar*)&motif,
                  sizeof(motif) / sizeof(glong));
  XFlush(display);
}

static const gchar* gl_info(GLenum name) {
  const GLubyte* value = glGetString(name);
  return value ? (const gchar*)value : "Unknown";
}

static void log_system_info(void) {
  struct utsname system;
  if (uname(&system) == 0) {
    g_printerr("System: %s %s - %s - X11\n", system.sysname, system.release,
               system.machine);
  } else {
    g_printerr("System: Unknown - X11\n");
  }
  g_printerr("OpenGL %s - Sokol - Using Device: %s - %s\n", gl_info(GL_VERSION),
             gl_info(GL_VENDOR), gl_info(GL_RENDERER));
  g_printerr("GLSL %s\n\n", gl_info(GL_SHADING_LANGUAGE_VERSION));
}

// sokol_app and sokol_gfx allocate through GLib's allocator
static gpointer glib_alloc(gsize size, G_GNUC_UNUSED gpointer user_data) {
  return g_malloc(size);
}

static void glib_free(gpointer ptr, G_GNUC_UNUSED gpointer user_data) {
  g_free(ptr);
}

static void on_init(void) {
  configure_fixed_window();
  sg_setup(&(sg_desc){
      .environment = sglue_environment(),
      .allocator = {.alloc_fn = glib_alloc, .free_fn = glib_free},
      .logger.func = slog_func,
  });
  log_system_info();
  if (gfx_init) gfx_init();
}

static void on_event(const sapp_event* e) {
  if (e->type == SAPP_EVENTTYPE_KEY_DOWN && !e->key_repeat && gfx_key_down)
    gfx_key_down((gint32)e->key_code);
}

static void on_cleanup(void) {
  gfx_renderer_cleanup();
  sg_shutdown();
}

static void (*gfx_user_cleanup)(void);
static void on_cleanup_wrapped(void) {
  if (gfx_user_cleanup) gfx_user_cleanup();
  on_cleanup();
}

gint32 gfx_run(const gfx_desc* d) {
  gfx_init = d->init;
  gfx_key_down = d->key_down;
  gfx_user_cleanup = d->cleanup;
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
      .allocator = {.alloc_fn = glib_alloc, .free_fn = glib_free},
      .logger.func = slog_func,
  });
  return 0;
}

void gfx_quit(void) { sapp_request_quit(); }
