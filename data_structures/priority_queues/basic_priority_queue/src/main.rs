use log::*;
// use benchmark_core::benchmarks::priority_queue_benchmarks::benchmark_priority_queue;


fn main() {
    // initialize env_logger if not in silent release mode
    #[cfg(not(all(not(debug_assertions), feature = "silent-release")))]
    {
        env_logger::init();
        debug!("envlogger init");
    }
    log::info!("Starting benchmark");

    // match benchmark_priority_queue::<basic_priority_queue::BasicPriorityQueue<usize>>("basic_priority_queue") {
    //     Ok(_) => println!("Benchmark done."),
    //     Err(e) => {
    //         eprintln!("Benchmark received error: {}", e);
    //         println!("Benchmark exiting due to error.");
    //     }
    // }

    println!("Hello from Basic Priority Queue Main");
}
