fn main() {
    #[cfg(any(feature = "boost", feature = "moodycamel", feature = "lcrq", feature = "lprq"))]
    {
        use std::env;
        use std::path::PathBuf;
        
        let queue_location = "src/cpp_queues";
        
        let mut build = cc::Build::new();
        build.cpp(true).flag("-std=c++20").include("/usr/include");
        
        
        let mut bindgen = bindgen::Builder::default()
            .clang_arg("-I/usr/include");
        
        // Configure for LPRQ
        #[cfg(feature = "lprq")]
        {
            println!("cargo:rerun-if-changed={}/lprq_wrapper.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/lprq_wrapper.cpp", queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LPRQueue.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LinkedRingQueue.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/HazardPointers.hpp", queue_location);
            
            
            build.file(format!("{}/lprq_wrapper.cpp", queue_location))
                .define("USE_LPRQUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/lprq_wrapper.hpp", queue_location))
                .clang_arg(format!("-I{}/cpp-ring-queues-research/include", queue_location))
                .allowlist_function("lprq_.*")
                .allowlist_type("LPRQ.*")
                .opaque_type("LPRQImpl");
        }
        
        // Configure for LCRQ
        #[cfg(feature = "lcrq")]
        {
            println!("cargo:rerun-if-changed={}/lcrq_wrapper.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/lcrq_wrapper.cpp", queue_location);
            println!("cargo:rerun-if-changed={}/LCRQueue.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/HazardPointers.hpp", queue_location);
            
            build.file(format!("{}/lcrq_wrapper.cpp", queue_location))
                .define("USE_LCRQUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/lcrq_wrapper.hpp", queue_location))
                .allowlist_function("lcrq_.*")
                .allowlist_type("LCRQ.*")
                .opaque_type("LCRQImpl");
        }
        
        // Configure for Boost
        #[cfg(feature = "boost")]
        {
            println!("cargo:rerun-if-changed={}/boost_wrapper.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/boost_wrapper.cpp", queue_location);
            
            build.file(format!("{}/boost_wrapper.cpp", queue_location))
                .define("USE_BOOST_QUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/boost_wrapper.hpp", queue_location))
                .allowlist_function("boost_queue_.*")
                .allowlist_type("BoostLockfreeQueue.*")
                .opaque_type("BoostLockfreeQueueImpl");
        }
        
        // Configure for MoodyCamel
        #[cfg(feature = "moodycamel")]
        {
            println!("cargo:rerun-if-changed={}/moodycamel_wrapper.hpp", queue_location);
            println!("cargo:rerun-if-changed={}/moodycamel_wrapper.cpp", queue_location);
            
            build.file(format!("{}/moodycamel_wrapper.cpp", queue_location))
                .define("USE_MOODYCAMEL_QUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/moodycamel_wrapper.hpp", queue_location))
                .allowlist_function("moody_camel_.*")
                .allowlist_type("MoodyCamelConcurrentQueue.*")
                .opaque_type("MoodyCamelConcurrentQueueImpl");
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
}
