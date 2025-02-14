fn main() {
    env_logger::init();
    log::info!("Starting benchmark");
    match lockfree_benchmark::start_benchmark() {
        Ok(_) => println!("Benchmark done."),
        Err(e) => {
            eprintln!("Benchmark received error: {}", e);
            println!("Benchmark exiting due to error.");
        }
    }
}

