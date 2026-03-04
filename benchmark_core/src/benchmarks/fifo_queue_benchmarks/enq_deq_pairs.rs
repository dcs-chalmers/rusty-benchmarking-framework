use crate::arguments::{FifoQueueArgs, FifoQueueBenchmarks};
use crate::benchmarks::benchmark_helpers::{self, BenchConfig};
use crate::benchmark_stats::BenchmarkStats;
use crate::traits::{ConcurrentQueue, HandleQueue};
use core_affinity::CoreId;
use log::{debug, error, info, trace};
use rand::Rng;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Barrier,
};
use std::sync::{mpsc, Arc};

/// # Explanation:
#[allow(dead_code)]
pub fn benchmark_enq_deq_pairs<C, T>(
    cqueue: C,
    bench_conf: &BenchConfig,
    fifo_queue_args: &FifoQueueArgs,
    stats: &mut BenchmarkStats,
) -> Result<(), std::io::Error>
where
    C: ConcurrentQueue<T>,
    T: Default,
    for<'a> &'a C: Send,
{
    // Extract specific arguments for this benchmark runner
    let enq_deq_pairs_args = match &fifo_queue_args.benchmark_runner {
        FifoQueueBenchmarks::EnqDeqPairs(a) => a,
        _ => panic!(
            "benchmark_enq_deq_pairs called with another FIFO Queue \
            configured. This is an implementation error."
        ),
    };

    {
        debug!(
            "Prefilling queue with {} items.",
            fifo_queue_args.prefill_amount
        );

        let mut tmp_handle = cqueue.register();
        for _ in 0..fifo_queue_args.prefill_amount {
            let _ = tmp_handle.push(Default::default());
        }
    }

    let thread_count = enq_deq_pairs_args.thread_count;
    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(thread_count + 1);
    let pops = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    info!("Starting pingpong benchmark with {} threads", thread_count);

    // Get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
    let mut core_iter = available_cores.into_iter().cycle();

    // Shared atomic bool for when a thread fails
    let thread_failed = Arc::new(AtomicBool::new(false));

    let _ = std::thread::scope(|s| -> Result<(), std::io::Error> {
        let queue = &cqueue;
        let thread_failed = &thread_failed; // Every thread clones the thread_failed bool
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let &thread_count = &thread_count;
        let is_one_socket = &bench_conf.args.one_socket;
        let tx = &tx;
        for _i in 0..thread_count {
            let mut core: CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    core_affinity::set_for_current(core);
                    let mut handle = queue.register();
                    let mut l_pushes = 0;
                    let mut l_pops = 0;
                    let _thread_failed = thread_failed.clone();
                    barrier.wait();
                    while !done.load(Ordering::Relaxed) {
                        let _ = handle.push(T::default());
                        l_pushes += 1;
                        let _ = handle.pop();
                        l_pops += 1;
                        for _ in 0..bench_conf.args.delay {
                            let _some_num = std::hint::black_box(
                                rand::rng().random::<f64>()
                            );
                        }
                    }
                    pushes.fetch_add(l_pushes, Ordering::Relaxed);
                    pops.fetch_add(l_pops, Ordering::Relaxed);
                    tx.send(l_pops + l_pushes).unwrap();
                    trace!("{}: Pushed: {}, Popped: {}", _i, l_pushes, l_pops);
                }));
                // A thread panicked, aborting the benchmark...
                if let Err(e) = result {
                    error!("Thread {} panicked: {:?}. Aborting benchmark, padding results to zero", _i, e);
                    thread_failed.store(true, Ordering::Relaxed);
                    done.store(true, Ordering::Relaxed);
                }
            });
        }
        barrier.wait();
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done.store(true, Ordering::Relaxed);
        Ok(())
    });
    drop(tx);
    let pops = pops.into_inner();
    let pushes = pushes.into_inner();
    // Fairness
    let ops_per_thread = {
        let mut vals = vec![];
        for received in rx {
            vals.push(received);
        }
        vals
    };

    // compute and store statistics
    let mut throughput = Some((pushes + pops) as f64 / time_limit as f64);
    let mut pushes = Some(pushes);
    let mut pops = Some(pops);
    let mut fairness = Some(benchmark_helpers::calc_fairness(ops_per_thread));
    if thread_failed.load(Ordering::Relaxed) {
        // a failed benchmark does not have some values
        throughput = None;
        pushes = None;
        pops = None;
        fairness = None;
    }
    stats.insert("Throughput", throughput);
    stats.insert("Enqueues", pushes);
    stats.insert("Dequeues", pops);
    stats.insert("Thread Count", Some(thread_count));
    stats.insert("Queuetype", Some(C::get_id()));
    stats.insert("Benchmark", Some(fifo_queue_args.benchmark_runner.to_string()));
    stats.insert("Test ID", Some(bench_conf.benchmark_id.to_owned()));
    stats.insert("Fairness", fairness);
    stats.insert("Queue Size", Some(fifo_queue_args.queue_size));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::arguments::FifoQueueArgs;
    use crate::arguments::FifoQueueBenchmarks;
    use crate::arguments::FifoQueueEnqDeqPairsArgs;

    use super::*;

    use crate::benchmarks::test_helpers::test_queue::TestQueue;

    #[test]
    fn run_enqdeq_pairs_with_struct() {
        let fifo_queue_args = FifoQueueArgs {
            benchmark_runner: FifoQueueBenchmarks::EnqDeqPairs(
                FifoQueueEnqDeqPairsArgs { thread_count: 20 },
            ),
            ..Default::default()
        };
        let bench_conf = benchmark_helpers::BenchConfig {
            args: fifo_queue_args.general_args.clone(),
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string(),
            benchmark_name: fifo_queue_args.benchmark_runner.to_string(),
        };
        let queue: TestQueue<usize> = TestQueue::new(0);
        let mut stats = BenchmarkStats::new("0");
        assert!(benchmark_enq_deq_pairs(queue, &bench_conf, &fifo_queue_args, &mut stats).is_ok());
    }
}
