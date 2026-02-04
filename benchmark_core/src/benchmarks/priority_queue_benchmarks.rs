#[allow(unused_imports)]
use crate::arguments::{
    Args, BenchmarkTypes, PriorityQueueArgs, PriorityQueueBenchmarks,
};
use crate::benchmarks::benchmark_helpers;
#[allow(unused_imports)]
use crate::traits::{ConcurrentPriorityQueue, HandlePriorityQueue};
use chrono::Local;
use clap::Parser;
#[allow(unused_imports)]
use log::{self, debug, error, info};
use std::collections::hash_map::DefaultHasher;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
#[allow(unused_imports)]
use std::sync::atomic::AtomicBool;

pub mod prod_con;

/// Create the queue, and run the selected benchmark a set of times
pub fn benchmark_priority_queue<Q, T>(
    queue_name: &str,
) -> Result<(), std::io::Error>
where
    Q: ConcurrentPriorityQueue<usize, T> + Send,
    T: Default,
    for<'a> &'a Q: Send,
{
    // Setup output and parse arguments
    let SetupResult {
        bench_conf,
        pq_conf,
        ..
    } = setup_benchmark()?;
    let bench_conf = &bench_conf;

    // Create a runner lambda for the different benchmarks, mainly needed for eg. BFS to load graph and so on
    let mut runner: Box<
        dyn FnMut(
            Q,
            &benchmark_helpers::BenchConfig,
        ) -> Result<(), std::io::Error>,
    > = match &pq_conf.benchmark_runner {
        PriorityQueueBenchmarks::ProdCon(_) => {
            Box::new(move |q, bench_conf| {
                prod_con::benchmark_prod_con(q, bench_conf)
            })
        }
    };

    for _current_iteration in 0..bench_conf.args.iterations {
        // Create the queue.
        let test_q: Q = Q::new(pq_conf.queue_size as usize);

        // Start memory tracking (if enabled)
        #[cfg(feature = "memory_tracking")]
        let (done, mem_thread_handle) = {
            let done =
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handle = benchmark_helpers::create_mem_tracking_thread(
                bench_conf,
                _current_iteration,
                &test_q,
                &done,
            )?;
            (done, handle)
        };

        // Execute the benchmark
        runner(test_q, bench_conf)?;

        // Join the thread again
        debug!("Queue should have been dropped now.");

        // Stop memory tracking (if enabled)
        #[cfg(feature = "memory_tracking")]
        {
            use std::sync::atomic::Ordering;
            debug!("Joining memory thread.");
            done.store(true, Ordering::Relaxed);
            if let Err(e) = mem_thread_handle.join().unwrap() {
                log::error!("Couldn't join memory tracking thread: {}", e);
            }
        }
    }

    if bench_conf.args.print_info {
        benchmark_helpers::print_info(queue_name.to_string(), bench_conf)?;
    }

    Ok(())
}

pub struct SetupResult {
    pub bench_conf: benchmark_helpers::BenchConfig,
    pub pq_conf: PriorityQueueArgs,
    pub columns: String, // Optional, but alreadyy written (TODO: have a stream of some sort here)
}

/// Set up the actual benchmark.
///
/// All work unrelated to the chosen benchmark is done here.
pub fn setup_benchmark() -> Result<SetupResult, std::io::Error> {
    let args = crate::arguments::Args::parse();

    let pq_args = match &args.benchmark {
        BenchmarkTypes::PriorityQueue(queue_args) => queue_args.clone(),
        _ => panic!(
            "Trying to run a queue benchmark with another benchmark type"
        ),
    };

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
    let bench_conf = benchmark_helpers::BenchConfig {
        args,
        date_time,
        benchmark_id,
        output_filename,
    };

    let columns = "Throughput,Enqueues,Dequeues,Consumers,Producers,Thread Count,Queuetype,Benchmark,Test ID,Fairness,Spread,Queue Size";

    if bench_conf.args.write_to_stdout {
        println!("{columns}")
    } else {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        writeln!(file, "{columns}")?;
    }

    Ok(SetupResult {
        bench_conf,
        pq_conf: pq_args,
        columns: columns.to_string(),
    })
}
