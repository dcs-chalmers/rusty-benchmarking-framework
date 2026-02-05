use log::*;
use benchmark_core::benchmarks::fifo_queue_benchmarks::benchmark_fifo_queue;


fn main() {
    // initialize env_logger if not in silent release mode
    #[cfg(not(all(not(debug_assertions), feature = "silent-release")))]
    {
        env_logger::init();
        debug!("envlogger init");
    }
    log::info!("Starting benchmark");

    match benchmark_fifo_queue::<basic_queue::BasicQueue<usize>>("basic_queue") {
        Ok(_) => println!("Benchmark done."),
        Err(e) => {
            eprintln!("Benchmark received error: {}", e);
            println!("Benchmark exiting due to error.");
        }
    }
}
