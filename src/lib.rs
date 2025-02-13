#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;
#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{stats, epoch};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use chrono::Local;
use clap::Parser;
use crate::benchmarks::Benchmarks;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs::OpenOptions;
use std::io::Write;

pub mod queues;
pub mod benchmarks;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Duration of each benchmark
    #[arg(short, long, default_value_t = 10)]
    time_limit: u64,
    /// Amount of producers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    producers: usize,
    /// Amount of consumers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    consumers: usize,
    /// Attemps to only use on socket. Specific for the developers test environment.
    #[arg(short, long, default_value_t = true)]
    one_socket: bool,
    /// How many times the chosen benchmark should be run.
    #[arg(short, long, default_value_t = 1)]
    iterations: u32,
    /// Count empty pop operations. Off by default.
    #[arg(short, long, default_value_t = false)]
    empty_pops: bool,
    /// Make the output of the benchmark human readable.
    #[arg(long, default_value_t = false)]
    human_readable: bool,
    /// Set the size of the bounded queues.
    #[arg(short, long, default_value_t = 10000)]
    queue_size: u32,
    /// Set the delay between operations. Default is 1ns.
    #[arg(short, long, default_value_t = 1)]
    delay_nanoseconds: u64,
    /// Set the output path for the result files.
    #[arg(long = "path", default_value_t = String::from("./output"))]
    path_output: String,
    /// Choose which benchmark to run.
    #[arg(value_enum)]
    benchmark: Benchmarks,
    /// Decide the spread of producers/consumers for the pingpong benchmark.
    /// Ex. 0.3 means 30% produce 70% consume.
    #[arg(long = "spread", default_value_t = 0.5)]
    spread: f64,
    /// If set to true, benchmark will output to stdout instead of to files.
    #[arg(long ="write-stdout", default_value_t = false)]
    write_to_stdout: bool,
    /// Set the thread count for the pingpong benchmark.
    #[arg(long = "thread-count", default_value_t = 20)]
    thread_count: usize,
}

pub fn start_benchmark() -> Result<(), std::io::Error> {
    let args = Args::parse();
    let date_time = Local::now().format("%Y%m%d%H%M%S").to_string();
    let benchmark_id = {
        let mut hasher = DefaultHasher::new();
        date_time.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };
    let output_filename = String::from(format!("{}/{}", args.path_output, date_time));
    let bench_conf = benchmarks::BenchConfig {
        args,
        date_time,
        benchmark_id,
        output_filename,
    };
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&bench_conf.output_filename)?;
    writeln!(file, "Throughput,Enqueues,Dequeues,Consumers,Producers,Queuetype,Benchmark,Test ID")?;

    for _ in 0..bench_conf.args.iterations {
        implement_benchmark!("lockfree_queue",
            crate::queues::lf_queue::LFQueue<i32>,
            "lockfree::queue:Queue",
            &bench_conf);
        implement_benchmark!("basic_queue",
            crate::queues::basic_queue::BasicQueue<i32>,
            "Basic Queue",
            &bench_conf);
        implement_benchmark!("concurrent_queue",
            crate::queues::concurrent_queue::CQueue<i32>,
            "concurrent_queue::ConcurrentQueue",
            &bench_conf);
        implement_benchmark!("array_queue",
            crate::queues::array_queue::AQueue<i32>,
            "crossbeam::queue::ArrayQueue",
            &bench_conf);
        implement_benchmark!("bounded_ringbuffer",
            crate::queues::bounded_ringbuffer::BoundedRingBuffer<i32>,
            "Bounded ringbuffer",
            &bench_conf);
    }
    Ok(())
}

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
    fn get_id(&self) -> String;
    fn new(size: usize) -> Self;
}

pub trait Handle<T> {
    fn push(&mut self, item: T);
    fn pop(&mut self) -> Option<T>;
}
