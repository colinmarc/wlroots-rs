extern crate meson_next as meson;
use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let libs = system_deps::Config::new().probe().unwrap();
    let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).to_owned();

    // Build wlroots using meson.
    let wlroots_build_path = out_dir.join("build");
    let wlroots_build_path_str = wlroots_build_path.to_str().unwrap();

    println!("cargo:rustc-link-search=native={}", wlroots_build_path_str);
    println!("cargo:rustc-link-lib=static=wlroots");

    let conf = meson::config::Config::new().options(HashMap::from([
        ("default_library", "static"),
        // Disable optional features for now.
        // TODO: enable these under crate features.
        ("auto_features", "disabled"),
        ("xwayland", "disabled"),
    ]));
    meson::build("wlroots", wlroots_build_path_str, conf);

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wlroots");

    let mut builder = bindgen::builder()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Most features are still marked unstable in wlroots.
        .clang_arg("-DWLR_USE_UNSTABLE")
        .header("wlroots.h")
        .clang_arg("-Iwlroots/include")
        .clang_arg(format!("-I{}/include", wlroots_build_path_str))
        .allowlist_function("_?wlr_.*")
        .allowlist_type("_?wlr_.*");

    for path in libs.all_include_paths() {
        builder = builder.clang_arg("-I").clang_arg(path.to_str().unwrap());
    }

    let bindings = builder.generate().unwrap();

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
