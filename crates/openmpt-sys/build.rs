use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=LIBOPENMPT_STATIC");
    println!("cargo:rerun-if-env-changed=LIBOPENMPT_LIB_DIR");

    let mut fallback_include = false;

    let library = if env::var("LIBOPENMPT_STATIC").is_ok() {
        let lib_dir = env::var("LIBOPENMPT_LIB_DIR")
            .expect("LIBOPENMPT_LIB_DIR must be set when LIBOPENMPT_STATIC is used");
        println!("cargo:rustc-link-search=native={}", lib_dir);
        println!("cargo:rustc-link-lib=static=openmpt");

                        // libopenmpt is C++, so we need the C++ standard library
                        let target = env::var("TARGET").unwrap();
                        let is_musl = target.contains("musl");
                        
                        if target.contains("apple") || target.contains("freebsd") || target.contains("openbsd") {
                            println!("cargo:rustc-link-lib=dylib=c++");
                        } else if is_musl {
                            // For musl, we usually want to link libstdc++ statically if possible, 
                            // but often we just link against the system's static libstdc++.
                            println!("cargo:rustc-link-lib=static=stdc++");
                        } else {
                            println!("cargo:rustc-link-lib=dylib=stdc++");
                        }
                                // We still need the include paths for bindgen. 
                // Try pkg-config but don't fail if it doesn't find the library (just to get include paths).
                let pc = pkg_config::Config::new();
                let pc_lib = pc.probe("libopenmpt").ok();
                // If pkg-config failed, add the local openmpt directory as a fallback for headers
        if pc_lib.is_none() {
            fallback_include = true;
        }
        pc_lib
    } else {
        Some(pkg_config::probe_library("libopenmpt").expect("libopenmpt not found via pkg-config"))
    };

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

    if fallback_include {
        println!("cargo:rustc-env=CPATH=../../../openmpt");
        builder = builder.clang_arg("-I../../../openmpt");
    }

    // Add include paths if we found them
    if let Some(lib) = library {
        for path in lib.include_paths {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
    } else {
        // Fallback: assume include is in /usr/include or similar,
        // or just let it fail if bindgen can't find headers.
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
