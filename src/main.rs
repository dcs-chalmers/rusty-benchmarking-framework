fn main() {
    match lockfree_benchmark::start_benchmark() {
        Ok(_) => println!("Benchmark done."),
        Err(e) => {
            eprintln!("Benchmark received error: {}", e);
            println!("Benchmark exiting due to error.");
        }
    }
}

