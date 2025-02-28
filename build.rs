// fn main() {
//     #[cfg(feature = "boost")]
//     {
//         use std::env;
//         use std::path::PathBuf;
//
//         // Tell cargo to rerun this if our wrapper changes
//         println!("cargo:rerun-if-changed=src/wrapper.hpp");
//         println!("cargo:rerun-if-changed=src/wrapper.cpp");
//
//         // Compile the C++ wrapper code
//         cc::Build::new()
//             .cpp(true)
//             .file("src/wrapper.cpp")
//             .flag("-std=c++14")  // Adjust based on your Boost version
//             .include("/usr/include")  // Path to Boost headers
//             .compile("boost_queue_wrapper");
//
//         // Generate bindings only for the C interface
//         let bindings = bindgen::Builder::default()
//             .header("src/wrapper.hpp")
//             // Tell bindgen about include paths
//             .clang_arg("-I/usr/include")
//             // Only generate bindings for functions and types we explicitly need
//             .allowlist_function("boost_queue_.*")
//             .allowlist_type("BoostLockfreeQueue.*")
//             // Avoid trying to generate bindings for all of Boost
//             .opaque_type("BoostLockfreeQueueImpl")
//             .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
//             .generate()
//             .expect("Unable to generate bindings");
//
//         // Write the bindings
//         let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
//         bindings
//             .write_to_file(out_path.join("bindings.rs"))
//             .expect("Couldn't write bindings!");
//     }
// }
fn main() {
    #[cfg(any(feature = "boost", feature = "moodycamel"))]
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
            .flag("-std=c++14")
            .include("/usr/include");  // Path to headers
        
        // Add definitions for conditional compilation
        #[cfg(feature = "boost")]
        build.define("USE_BOOST_QUEUE", None);
        
        #[cfg(feature = "moodycamel")]
        build.define("USE_MOODYCAMEL_QUEUE", None);
        
        build.compile("queue_wrapper");
        
        // Generate bindings
        let mut bindgen = bindgen::Builder::default()
            .header("src/wrapper.hpp")
            .clang_arg("-I/usr/include");
        
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
