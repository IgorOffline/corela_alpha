/// Must match `links` in Cargo.toml.
const NATIVE_LIB_NAME: &str = "corela_sokol";
const C_SOURCE_DIR: &str = "csrc";
const SOKOL_DIR: &str = "sokol";
const NANOVG_DIR: &str = "nanovg";
const C_STANDARD: &str = "c11";
/// POSIX.1-2008
const POSIX_C_SOURCE_VERSION: &str = "200809L";
/// Install prefix of the GLib built from ~/dev/glib-2.90.1.
/// The runtime search path in .cargo/config.toml must point at the same lib dir.
const GLIB_PREFIX: &str = "/opt/glib-2.90.1";
const GLIB_LIB_NAME: &str = "glib-2.0";
/// Pins the GLib API surface the C sources may use.
const GLIB_VERSION: &str = "GLIB_VERSION_2_90";
/// Tag for g_critical() and friends.
const G_LOG_DOMAIN: &str = "\"corela\"";
/// X11 windowing, OpenGL, and the C runtime pieces sokol_app needs on Linux.
const SYSTEM_LIBS: [&str; 8] = ["X11", "Xi", "Xcursor", "GL", "dl", "pthread", "m", "c"];

fn main() {
    for dir in [C_SOURCE_DIR, SOKOL_DIR, NANOVG_DIR] {
        println!("cargo:rerun-if-changed={dir}");
    }

    cc::Build::new()
        .std(C_STANDARD)
        .define("_POSIX_C_SOURCE", POSIX_C_SOURCE_VERSION)
        .file(format!("{C_SOURCE_DIR}/sokol.c"))
        .file(format!("{C_SOURCE_DIR}/draw.c"))
        .file(format!("{NANOVG_DIR}/nanovg.c"))
        .include(SOKOL_DIR)
        .include(NANOVG_DIR)
        .include(format!("{GLIB_PREFIX}/include/glib-2.0"))
        .include(format!("{GLIB_PREFIX}/lib/glib-2.0/include"))
        .define("G_LOG_DOMAIN", G_LOG_DOMAIN)
        .define("GLIB_VERSION_MIN_REQUIRED", GLIB_VERSION)
        .define("GLIB_VERSION_MAX_ALLOWED", GLIB_VERSION)
        .define("NVG_NO_STB", None)
        .define("SOKOL_GLCORE", None)
        .warnings(true)
        .compile(NATIVE_LIB_NAME);

    println!("cargo:rustc-link-search=native={GLIB_PREFIX}/lib");
    println!("cargo:rustc-link-lib={GLIB_LIB_NAME}");
    for lib in SYSTEM_LIBS {
        println!("cargo:rustc-link-lib={lib}");
    }
}
