fn main() {
    use std::env;
    use std::path::PathBuf;

    let queue_location = "cpp_src";

    let mut build = cc::Build::new();
    build.cpp(true).flag("-std=c++20").include("/usr/include");


    let mut bindgen = bindgen::Builder::default()
        .clang_arg("-I/usr/include");

    // Configure for FAAAAQ
    {
        println!("cargo:rerun-if-changed={}/faaa_queue_wrapper.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/faaa_queue_wrapper.cpp", queue_location);
        println!("cargo:rerun-if-changed={}/FAAArrayQueue.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/../HazardPointers.hpp", queue_location);

        build.file(format!("{}/faaa_queue_wrapper.cpp", queue_location))
            .define("USE_FAAAQUEUE", None);

        bindgen = bindgen
            .header(format!("{}/faaa_queue_wrapper.hpp", queue_location))
            .allowlist_function("faaaq_.*")
            .allowlist_type("FAAAQ.*")
            .opaque_type("FAAAQImpl");
    }

    // Compile the C++ code
    build.compile("queue_wrapper");

    // Generate bindings
    let bindings = bindgen
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
