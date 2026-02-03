use core_affinity::CoreId;
use log::{debug, error, info, trace};
use rand::Rng;
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Barrier};
use crate::traits::{ConcurrentQueue, HandleQueue};
use crate::benchmarks::benchmark_helpers;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{mpsc, Arc};

/// # Explanation:
#[allow(dead_code)]
pub fn benchmark_enq_deq_pairs<C, T> (cqueue: C, bench_conf: &benchmark_helpers::BenchConfig) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<T>,
T: Default,
    for<'a> &'a C: Send
{
    let args = match &bench_conf.args.benchmark {
        crate::arguments::QueueBenchmarks::EnqDeqPairs(a) => a,
        _ => panic!(),
    };
    {
        debug!("Prefilling queue with {} items.", bench_conf.args.prefill_amount);
        let mut tmp_handle = cqueue.register();
        for _ in 0..bench_conf.args.prefill_amount {
            let _ = tmp_handle.push(Default::default());
        }
    }
    let thread_count = args.thread_count;
    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(thread_count + 1);
    let pops  = AtomicUsize::new(0);
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


    let _ = std::thread::scope(|s| -> Result<(), std::io::Error>{
        let queue = &cqueue;
        let thread_failed = &thread_failed; // Every thread clones the thread_failed bool
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let &thread_count = &thread_count;
        let is_one_socket = &bench_conf.args.one_socket;
        let tx = &tx;
        for _i in 0..thread_count{
            let mut core : CoreId = core_iter.next().unwrap();
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
                            let _some_num = rand::rng().random::<f64>();
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
    let fairness = benchmark_helpers::calc_fairness(ops_per_thread);

    // If a thread crashed, pad the results with zero-values
    let formatted = if thread_failed.load(Ordering::Relaxed) {
        format!("0,0,0,-1,-1,{},{},{},{},0,{},{}",
            thread_count,
            cqueue.get_id(),
            bench_conf.args.benchmark,
            bench_conf.benchmark_id,
            -1,
            bench_conf.args.queue_size
            )
    }
    else {
        format!("{},{},{},{},{},{},{},{},{},{},{},{}",
        (pushes + pops) as f64 / time_limit as f64,
        pushes,
        pops,
        -1,
        -1,
        thread_count,
        cqueue.get_id(),
        bench_conf.args.benchmark,
        bench_conf.benchmark_id,
        fairness,
        -1,
        bench_conf.args.queue_size)
    };
    // Write to file or stdout depending on flag
    if !bench_conf.args.write_to_stdout {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        writeln!(file, "{}", formatted)?;
    } else {
        println!("{}", formatted);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::arguments::Args;
    use crate::arguments::EnqDeqPairsArgs;
    use crate::arguments::QueueBenchmarks;
    use crate::benchmarks::queue_benchmarks::enq_deq_pairs::benchmark_enq_deq_pairs;

    use super::*;

    use crate::benchmarks::test_helpers::test_queue::TestQueue;

    #[test]
    fn run_enqdeq_pairs_with_struct() {
        let args = Args {
            benchmark: QueueBenchmarks::EnqDeqPairs(EnqDeqPairsArgs { thread_count: 10 }),
            ..Default::default()
        };
        let bench_conf = benchmark_helpers::BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let queue = TestQueue::<usize>::new(0);
        if benchmark_enq_deq_pairs(queue, &bench_conf).is_err() {
            panic!();
        }
    }
}
