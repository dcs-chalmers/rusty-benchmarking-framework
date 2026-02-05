#[allow(unused_imports)]
use crate::arguments::{FifoQueueArgs, FifoQueueBenchmarks, GeneralArgs};
use crate::benchmarks::benchmark_helpers::{self, BenchConfig};
#[allow(unused_imports)]
use crate::traits::{ConcurrentQueue, HandleQueue};
use clap::Parser;
#[allow(unused_imports)]
use log::{self, debug, error, info};
#[allow(unused_imports)]
use std::sync::atomic::AtomicBool;

pub mod bfs;
pub mod enq_deq;
pub mod enq_deq_pairs;
pub mod prod_con;

/// Create the fifo queue, and run the selected benchmark a set of times
pub fn benchmark_fifo_queue<Q>(queue_name: &str) -> Result<(), std::io::Error>
where
    Q: ConcurrentQueue<usize> + Send,
    for<'a> &'a Q: Send,
{
    let (bench_conf, fifo_queue_args) = setup_benchmark()?;

    // The variables need to be references to avoid moving into closures below.
    // TODO(emilbjornlinger): Find a better solution.
    let bench_conf = &bench_conf;
    let fifo_queue_args = &fifo_queue_args;

    // Create a runner lambda for the different benchmarks, mainly needed for eg. BFS to load graph and so on
    let mut runner: Box<
        dyn FnMut(Q, &BenchConfig) -> Result<(), std::io::Error>,
    > = match &fifo_queue_args.benchmark_runner {
        FifoQueueBenchmarks::ProdCon(_) => Box::new(move |q, bench_conf| {
            prod_con::benchmark_prod_con(q, bench_conf, fifo_queue_args)
        }),
        FifoQueueBenchmarks::EnqDeq(_) => Box::new(move |q, bench_conf| {
            enq_deq::benchmark_enq_deq(q, bench_conf, fifo_queue_args)
        }),
        FifoQueueBenchmarks::EnqDeqPairs(_) => {
            Box::new(move |q, bench_conf| {
                enq_deq_pairs::benchmark_enq_deq_pairs(
                    q,
                    bench_conf,
                    fifo_queue_args,
                )
            })
        }
        FifoQueueBenchmarks::BFS(args) => {
            let (graph, seq_ret_vec, start_node) = bfs::pre_bfs_work(
                Q::new(fifo_queue_args.queue_size as usize),
                &args,
            );
            Box::new(move |q, _conf| {
                bfs::benchmark_bfs(
                    q,
                    &graph,
                    &bench_conf,
                    &seq_ret_vec,
                    start_node,
                    fifo_queue_args,
                )
            })
        }
    };

    for _current_iteration in 0..bench_conf.args.iterations {
        // Create the queue.
        let test_q: Q = Q::new(fifo_queue_args.queue_size as usize);

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
        runner(test_q, &bench_conf)?;

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
            &bench_conf,
            fifo_queue_args.benchmark_runner.to_string(),
        )?;
    }

    Ok(())
}

/// Parse arguments and start outputting result of benchmark
pub fn setup_benchmark() -> Result<(BenchConfig, FifoQueueArgs), std::io::Error>
{
    let args = crate::arguments::FifoQueueArgs::parse();
    let bench_config =
        benchmark_helpers::create_bench_config(&args.general_args)?;

    let columns = match args.benchmark_runner {
        FifoQueueBenchmarks::BFS(_) => {
            "Milliseconds,Queuetype,Thread Count,Test ID"
        },
        _ => {
            "Throughput,Enqueues,Dequeues,Consumers,Producers,Thread Count,Queuetype,Benchmark,Test ID,Fairness,Spread,Queue Size"
        }
    };

    benchmark_helpers::output_result_header(
        columns.to_string(),
        &bench_config,
    )?;

    Ok((bench_config, args))
}
