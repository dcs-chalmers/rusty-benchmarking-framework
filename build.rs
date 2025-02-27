fn main() {
    #[cfg(feature = "boost")]
    {
        use std::env;
        use std::path::PathBuf;

        // Tell cargo to rerun this if our wrapper changes
        println!("cargo:rerun-if-changed=src/wrapper.hpp");
        println!("cargo:rerun-if-changed=src/wrapper.cpp");

        // Compile the C++ wrapper code
        cc::Build::new()
            .cpp(true)
            .file("src/wrapper.cpp")
            .flag("-std=c++14")  // Adjust based on your Boost version
            .include("/usr/include")  // Path to Boost headers
            .compile("boost_queue_wrapper");

        // Generate bindings only for the C interface
        let bindings = bindgen::Builder::default()
            .header("src/wrapper.hpp")
            // Tell bindgen about include paths
            .clang_arg("-I/usr/include")
            // Only generate bindings for functions and types we explicitly need
            .allowlist_function("boost_queue_.*")
            .allowlist_type("BoostLockfreeQueue.*")
            // Avoid trying to generate bindings for all of Boost
            .opaque_type("BoostLockfreeQueueImpl")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        // Write the bindings
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}
