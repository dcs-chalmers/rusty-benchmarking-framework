use log::*;
use benchmark_core::benchmarks::queue_benchmarks::benchmark_queue;


fn main() {
    // initialize env_logger if not in silent release mode
    #[cfg(not(all(not(debug_assertions), feature = "silent-release")))]
    {
        env_logger::init();
        debug!("envlogger init");
    }
    log::info!("Starting benchmark");

    match benchmark_queue::<scc_stack::SCCStack<usize>>("scc_stack") {
        Ok(_) => println!("Benchmark done."),
        Err(e) => {
            eprintln!("Benchmark received error: {}", e);
            println!("Benchmark exiting due to error.");
        }
    }
}
