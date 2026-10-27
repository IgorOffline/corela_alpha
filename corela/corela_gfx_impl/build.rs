fn main() {
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=sokol");

    cc::Build::new()
        .file("csrc/sokol.c")
        .include("sokol")
        .define("SOKOL_GLCORE", None)
        .warnings(true)
        .compile("corela_sokol");

    for lib in ["X11", "Xi", "Xcursor", "GL", "dl", "pthread", "m", "c"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}
