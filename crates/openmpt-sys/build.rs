use std::env;
use std::path::PathBuf;

fn main() {
    let library = pkg_config::probe_library("libopenmpt").expect("libopenmpt not found via pkg-config");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut builder = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell bindgen to generate bindings for openmpt.* and OPENMPT.* items
        .allowlist_item("openmpt.*")
        .allowlist_item("OPENMPT.*")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files change.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Add include paths from pkg-config
    for path in library.include_paths {
        builder = builder.clang_arg(format!("-I{}", path.display()));
    }

    let bindings = builder
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
