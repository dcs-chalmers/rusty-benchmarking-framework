fn main() {
    #[cfg(any(
            feature = "boost_cpp", 
            feature = "moodycamel_cpp", 
            feature = "lcrq_cpp", 
            feature = "lprq_cpp", 
            feature = "faaa_queue_cpp"
            ))]
    {
        use std::env;
        use std::path::PathBuf;
        
        let queue_location = "src/cpp_queues";
        
        let mut build = cc::Build::new();
        build.cpp(true).flag("-std=c++20").include("/usr/include");
        
        
        let mut bindgen = bindgen::Builder::default()
            .clang_arg("-I/usr/include");
        
        // Configure for LPRQ
        #[cfg(feature = "lprq_cpp")]
        {
            let my_queue_location = format!("{queue_location}/lprq");
            println!("cargo:rerun-if-changed={}/lprq_wrapper.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/lprq_wrapper.cpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LPRQueue.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/LinkedRingQueue.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/cpp-ring-queues-research/include/HazardPointers.hpp", my_queue_location);
            
            
            build.file(format!("{}/lprq_wrapper.cpp", my_queue_location))
                .define("USE_LPRQUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/lprq_wrapper.hpp", my_queue_location))
                .clang_arg(format!("-I{}/cpp-ring-queues-research/include", my_queue_location))
                .allowlist_function("lprq_.*")
                .allowlist_type("LPRQ.*")
                .opaque_type("LPRQImpl");
        }
        
        // Configure for LCRQ
        #[cfg(feature = "lcrq_cpp")]
        {
            let my_queue_location = format!("{queue_location}/lcrq");
            println!("cargo:rerun-if-changed={}/lcrq_wrapper.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/lcrq_wrapper.cpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/LCRQueue.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/../HazardPointers.hpp", my_queue_location);
            
            build.file(format!("{}/lcrq_wrapper.cpp", my_queue_location))
                .define("USE_LCRQUEUE", None)
                .include(format!("{}/../", my_queue_location));
                
            bindgen = bindgen
                .header(format!("{}/lcrq_wrapper.hpp", my_queue_location))
                .allowlist_function("lcrq_.*")
                .allowlist_type("LCRQ.*")
                .opaque_type("LCRQImpl");
        }
        
        // Configure for Boost
        #[cfg(feature = "boost_cpp")]
        {
            let my_queue_location = format!("{queue_location}/boost");

            println!("cargo:rerun-if-changed={}/boost_wrapper.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/boost_wrapper.cpp", my_queue_location);
            
            build.file(format!("{}/boost_wrapper.cpp", my_queue_location))
                .define("USE_BOOST_QUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/boost_wrapper.hpp", my_queue_location))
                .allowlist_function("boost_queue_.*")
                .allowlist_type("BoostLockfreeQueue.*")
                .opaque_type("BoostLockfreeQueueImpl");
        }
        
        // Configure for MoodyCamel
        #[cfg(feature = "moodycamel_cpp")]
        {
            let my_queue_location = format!("{queue_location}/moodycamel");

            println!("cargo:rerun-if-changed={}/moodycamel_wrapper.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/moodycamel_wrapper.cpp", my_queue_location);
            
            build.file(format!("{}/moodycamel_wrapper.cpp", my_queue_location))
                .define("USE_MOODYCAMEL_QUEUE", None);
                
            bindgen = bindgen
                .header(format!("{}/moodycamel_wrapper.hpp", my_queue_location))
                .allowlist_function("moody_camel_.*")
                .allowlist_type("MoodyCamelConcurrentQueue.*")
                .opaque_type("MoodyCamelConcurrentQueueImpl");
        }
        // Configure for FAAAQueue 
        #[cfg(feature = "faaa_queue_cpp")]
        {
            let my_queue_location = format!("{queue_location}/faaaqueue");

            println!("cargo:rerun-if-changed={}/faaa_queue_wrapper.hpp", my_queue_location);
            println!("cargo:rerun-if-changed={}/faaa_queue_wrapper.cpp", my_queue_location);
            
            build.file(format!("{}/faaa_queue_wrapper.cpp", my_queue_location))
                .define("USE_MOODYCAMEL_QUEUE", None)
                .include(format!("{}/../", my_queue_location))
                .include(format!("{}/ConcurrencyFreaks/CPP/queues/array", my_queue_location));
                
            bindgen = bindgen
                .header(format!("{}/faaa_queue_wrapper.hpp", my_queue_location))
                .clang_arg(format!("-I{}/ConcurrencyFreaks/CPP/queues", my_queue_location))
                .clang_arg(format!("-I{}/ConcurrencyFreaks/CPP/queues/array", my_queue_location))
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
}
