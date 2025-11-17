#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use chrono::Local;
#[allow(unused_imports)]
use log::{self, debug, error, info};
use clap::Parser;
use std::collections::hash_map::DefaultHasher;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
#[allow(unused_imports)]
use crate::traits::{ConcurrentQueue, Handle};

pub mod benchmarks;
pub mod order;
pub mod arguments;
pub mod traits;


pub struct SetupResult {
    pub bench_conf: benchmarks::BenchConfig,
    pub columns: String, // Optional, but alreadyy written (TODO: have a stream of some sort here)
}

pub fn benchmark_target_queue<Q>(queue_name: &str) -> Result<(), std::io::Error>
where
    Q: ConcurrentQueue<usize> + Send,
    for<'a> &'a Q: Send
{
    let SetupResult { bench_conf, .. } = setup_benchmark()?;
    benchmarks::benchmark_queue::<Q>(&bench_conf, queue_name)
}


/// Set up the actual benchmark.
///
/// All work unrelated to the chosen benchmark is done here.
pub fn setup_benchmark() -> Result<SetupResult, std::io::Error> {
    let args = crate::arguments::Args::parse();
    let date_time = Local::now().format("%Y%m%d%H%M%S").to_string();
    // Create benchmark hashed id
    let benchmark_id = {
        let mut hasher = DefaultHasher::new();
        date_time.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    debug!("Benchmark ID: {}", benchmark_id);
    debug!("Arguments: {:?}", args);

    // Create dir if it doesnt already exist.
    if !std::path::Path::new(&args.path_output).exists() {
        std::fs::create_dir(&args.path_output)?;
    }

    let output_filename = format!("{}/{}", args.path_output, date_time);
    let bench_conf = benchmarks::BenchConfig {
        args,
        date_time,
        benchmark_id,
        output_filename,
    };

    let columns = match bench_conf.args.benchmark {
        arguments::Benchmarks::BFS(_) => {
            "Milliseconds,Queuetype,Thread Count,Test ID"
        },
        _ => {
            "Throughput,Enqueues,Dequeues,Consumers,Producers,Thread Count,Queuetype,Benchmark,Test ID,Fairness,Spread,Queue Size"
        }
    };

    if bench_conf.args.write_to_stdout {
        println!("{columns}")
    } else {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        writeln!(file, "{columns}")?;
    }

    Ok(SetupResult{bench_conf, columns: columns.to_string()})
}
