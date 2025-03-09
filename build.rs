fn main() {
    #[cfg(any(feature = "boost", feature = "moodycamel", feature = "lcrq", feature = "lprq"))]
    {
        use std::env;
        use std::path::PathBuf;
        
        // Tell cargo to rerun this if our wrapper changes
        println!("cargo:rerun-if-changed=src/wrapper.hpp");
        println!("cargo:rerun-if-changed=src/wrapper.cpp");
        
        // Compile the C++ wrapper code
        let mut build = cc::Build::new();
        
        build.cpp(true)
            .file("src/wrapper.cpp")
            .flag("-std=c++20")
            .include("/usr/include");  // Path to headers
        
        // Add definitions for conditional compilation
        #[cfg(feature = "boost")]
        build.define("USE_BOOST_QUEUE", None);
        
        #[cfg(feature = "moodycamel")]
        build.define("USE_MOODYCAMEL_QUEUE", None);

        #[cfg(feature = "lcrq")]
        build.define("USE_LCRQUEUE", None);

        #[cfg(feature = "lprq")]
        build.define("USE_LCRQUEUE", None);

        
        build.compile("queue_wrapper");
        
        // Generate bindings
        let mut bindgen = bindgen::Builder::default()
            .header("src/wrapper.hpp")
            .clang_arg("-I/usr/include")
            .clang_arg("-I/home/jam/lockfree-benchmark/src/cpp-ring-queues-research/include");
        
        // Include bindings for both queue implementations based on features
        #[cfg(feature = "boost")]
        {
            bindgen = bindgen
                .allowlist_function("boost_queue_.*")
                .allowlist_type("BoostLockfreeQueue.*")
                .opaque_type("BoostLockfreeQueueImpl");
        }
        
        #[cfg(feature = "moodycamel")]
        {
            bindgen = bindgen
                .allowlist_function("moody_camel_.*")
                .allowlist_type("MoodyCamelConcurrentQueue.*")
                .opaque_type("MoodyCamelConcurrentQueueImpl");
        }

        #[cfg(feature = "lcrq")]
        {
            bindgen = bindgen
                .allowlist_function("lcrq_.*")
                .allowlist_type("LCRQ.*")
                .opaque_type("LCRQImpl");
        }

        #[cfg(feature = "lprq")]
        {
            bindgen = bindgen
                .allowlist_function("lprq_.*")
                .allowlist_type("LPRQ.*")
                .opaque_type("LPRQImpl");
        }
        
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
}
