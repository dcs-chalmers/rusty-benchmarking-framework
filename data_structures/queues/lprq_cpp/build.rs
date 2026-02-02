fn main() {
    use std::env;
    use std::path::PathBuf;

    let queue_location = "cpp_src";

    let mut build = cc::Build::new();
    build.cpp(true).flag("-std=c++20").include("/usr/include");


    let mut bindgen = bindgen::Builder::default()
        .clang_arg("-I/usr/include");

    // Configure for LPRQ
    {
        println!("cargo:rerun-if-changed={}/lprq_wrapper.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/lprq_wrapper.cpp", queue_location);
        println!("cargo:rerun-if-changed={}/LCRQueue.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LPRQueue.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/RQCell.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/CacheRemap.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LinkedRingQueue.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/x86AtomicOps.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/HazardPointers.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/Metrics.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/Stats.hpp", queue_location);

        build.file(format!("{}/lprq_wrapper.cpp", queue_location));
            // .define("USE_LPRQUEUE", None)
            // .include(format!("{}/../", queue_location));

        bindgen = bindgen
            .header(format!("{}/lprq_wrapper.hpp", queue_location))
            .allowlist_function("lprq_.*")
            .allowlist_type("LPRQ.*")
            .opaque_type("LPRQImpl");
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
