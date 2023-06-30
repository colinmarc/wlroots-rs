use std::env;
use std::path::PathBuf;

fn main() {
    let libs = system_deps::Config::new().probe().unwrap();

    let mut builder = bindgen::builder()
        .header("wlroots.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg("-DWLR_USE_UNSTABLE") // Most features are still marked unstable in wlroots.
        .allowlist_type("_?wlr_.*");

    // TODO: support statically linking wlroots w/ optional features.

    for path in libs.all_include_paths() {
        builder = builder.clang_arg("-I").clang_arg(path.to_str().unwrap());
    }

    let bindings = builder.generate().unwrap();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
