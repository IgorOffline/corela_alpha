fn main() {
    println!("cargo:rerun-if-changed=csrc");

    cc::Build::new()
        .file("csrc/sokol.c")
        .include("csrc")
        .define("SOKOL_GLCORE", None)
        .warnings(true)
        .compile("lightshine_sokol");

    for lib in ["X11", "Xi", "Xcursor", "GL", "dl", "pthread", "m", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
