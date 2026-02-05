#[allow(unused_imports)]
use crate::arguments::{
    GeneralArgs, PriorityQueueArgs, PriorityQueueBenchmarks,
};
use crate::benchmarks::benchmark_helpers::{self, BenchConfig};
#[allow(unused_imports)]
use crate::traits::{ConcurrentPriorityQueue, HandlePriorityQueue};
use clap::Parser;
#[allow(unused_imports)]
use log::{self, debug, error, info};
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
    let (bench_conf, pq_args) = setup_benchmark()?;

    // The variables need to be references to avoid moving into closures below.
    // TODO(emilbjornlinger): Find a better solution.
    let bench_conf = &bench_conf;
    let pq_args = &pq_args;

    // Create a runner lambda for the different benchmarks, mainly needed for eg. BFS to load graph and so on
    let mut runner: Box<
        dyn FnMut(
            Q,
            &benchmark_helpers::BenchConfig,
        ) -> Result<(), std::io::Error>,
    > = match &pq_args.benchmark_runner {
        PriorityQueueBenchmarks::ProdCon(_) => {
            Box::new(move |q, bench_conf| {
                prod_con::benchmark_prod_con(q, bench_conf, pq_args)
            })
        }
    };

    for _current_iteration in 0..bench_conf.args.iterations {
        // Create the queue.
        let test_q: Q = Q::new(pq_args.queue_size as usize);

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
        benchmark_helpers::print_info(
            queue_name.to_string(),
            bench_conf,
            pq_args.benchmark_runner.to_string(),
        )?;
    }

    Ok(())
}

/// Parse arguments and start outputting result of benchmark
pub fn setup_benchmark(
) -> Result<(BenchConfig, PriorityQueueArgs), std::io::Error> {
    let args = crate::arguments::PriorityQueueArgs::parse();
    let bench_config =
        benchmark_helpers::create_bench_config(&args.general_args)?;

    let columns = "Throughput,Enqueues,Dequeues,Consumers,Producers,\
        Thread Count,Queuetype,Benchmark,Test ID,Fairness,Spread,Queue Size";

    benchmark_helpers::output_result_header(
        columns.to_string(),
        &bench_config,
    )?;

    Ok((bench_config, args))
}
