use log::*;

fn main() {
    // initialize env_logger if not in silent release mode
    #[cfg(not(all(not(debug_assertions), feature = "silent-release")))]
    {
        env_logger::init();
        debug!("envlogger init");
    }
    log::info!("Starting benchmark");

    match lockfree_benchmark::start_benchmark() {
        Ok(_) => println!("Benchmark done."),
        Err(e) => {
            eprintln!("Benchmark received error: {}", e);
            println!("Benchmark exiting due to error.");
        }
    }
}

