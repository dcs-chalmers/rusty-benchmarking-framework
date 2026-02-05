use core_affinity::CoreId;
use log::{debug, error, info, trace};
use rand::Rng;
use crate::arguments::{PriorityQueueArgs, PriorityQueueBenchmarks};
use crate::traits::{ConcurrentPriorityQueue, HandlePriorityQueue};
use crate::benchmarks::benchmark_helpers::{self, BenchConfig};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Barrier};
use std::sync::{mpsc, Arc};

// TODO(emilbjornlinger): Change benchmark to something reasonable

/// # Explanation:
/// A simple benchmark that measures the throughput of a queue.
/// Has by default a 10 floating points generated delay between each operation, but this can be changed
/// through flags passed to the program.
/// Benchmark specific flags:
/// * -p        Set specified amount of producers
/// * -c        Set specified amount of consumers
#[allow(dead_code)]
pub fn benchmark_prod_con<C, P, T>(cqueue: C, bench_conf: &BenchConfig, pq_args: &PriorityQueueArgs) -> Result<(), std::io::Error>
where 
    C: ConcurrentPriorityQueue<P, T>,
    P: Ord + From<usize>,
    T: Default,
    for<'a> &'a C: Send
{
    // Extract specific arguments for this benchmark runner
    let prod_con_args = match &pq_args.benchmark_runner {
        PriorityQueueBenchmarks::ProdCon(a) => a,
    };

    {
        debug!("Prefilling priority queue with {} items.", pq_args.prefill_amount);
        let mut tmp_handle = cqueue.register();
        for i in 0..pq_args.prefill_amount {
            let prio: usize = i.try_into().unwrap();
            let _ = tmp_handle.insert(P::from(prio), Default::default());
        } 
    }
    let producers = prod_con_args.producers;
    let consumers = prod_con_args.consumers;

    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(consumers + producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    info!("Starting throughput benchmark with {} consumer and {} producers", consumers, producers);
    
    // get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
        let mut core_iter = available_cores.into_iter().cycle();

    // Shared atomic bool for when a thread fails
    let thread_failed = Arc::new(AtomicBool::new(false));

    let _ = std::thread::scope(|s| -> Result<(), std::io::Error>{
        let queue = &cqueue;
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let tx = &tx;
        let &consumers = &consumers;
        let &producers = &producers;
        let is_one_socket = &bench_conf.args.one_socket;
        let thread_failed = &thread_failed;

        for i in 0..producers{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            trace!("Thread: {} Core: {:?}", i, core);
            s.spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // push
                let mut l_pushes= 0; 
                let _thread_failed = thread_failed.clone(); // Every thread clones the thread_failed bool
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    // NOTE: Maybe we should care about this result?
                    let _ = handle.insert(P::from(l_pushes), T::default());
                    l_pushes += 1;
                    // Add some delay to simulate real workload
                    for _ in 0..bench_conf.args.delay {
                        let _some_num = rand::rng().random::<f64>();
                    }
                }
                pushes.fetch_add(l_pushes, Ordering::Relaxed);
                // Thread sends its total operations down the channel for fairness calculations
                if let Err(e) = tx.send(l_pushes) {
                    error!("Error sending operations down the channel: {}", e);
                };
            }));
            // A thread panicked, aborting the benchmark...
            if let Err (e) = result {
                error!("Thread {} panicked in pushing: {:?}. Aborting benchmark, padding results to zero", i, e);
                    thread_failed.store(true, Ordering::Relaxed);
                    done.store(true, Ordering::Relaxed);
            }
            }); 
        }
        for i in 0..consumers {
            let mut core : CoreId = core_iter.next().expect("Core iter error");
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            trace!("Thread: {} Core: {:?}", i, core);
            
            s.spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // pop
                let mut l_pops = 0; 
                let mut empty_pops = 0;
                let _thread_failed = thread_failed.clone(); // Every thread clones the thread_failed bool
                barrier.wait();
                // TODO: add empty pops probably to fairness calculations
                while !done.load(Ordering::Relaxed) {
                    match handle.delete_min() {
                        Some(_) => l_pops += 1,
                        None => {
                            // if bench_conf.args.empty_pops {
                            //     l_pops += 1;
                            // }
                            empty_pops += 1;
                        }
                    }
                    for _ in 0..bench_conf.args.delay {
                        let _some_num = rand::rng().random::<f64>();
                    }
                }
                pops.fetch_add(l_pops, Ordering::Relaxed);
                // Thread sends its total operations down the channel for fairness calculations
                if let Err(e) = tx.send(l_pops + empty_pops) {
                    error!("Error sending operations down the channel: {}", e);
                };
            }));
            // A thread panicked, aborting the benchmark...
            if let Err(e) = result {
                error!("Thread {} panicked while popping: {:?}. Aborting benchmark, padding results to zero", i, e);
                thread_failed.store(true, Ordering::Relaxed);
                done.store(true, Ordering::Relaxed);
            }
            }); 
        }
        debug!("Waiting for barrier");
        barrier.wait();
        debug!("Done waiting for barrier. Going to sleep.");
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done.store(true, Ordering::Relaxed);
        Ok(())
    });
    drop(tx);
    debug!("TX Dropped");
    let pops = pops.into_inner();
    let pushes = pushes.into_inner();

    // Fairness
    // Get total operations per thread
    let ops_per_thread = {
        let mut vals = vec![];
        for received in rx {
            vals.push(received);
        };
        vals
    };
    // If a thread crashed, pad the results with zero-values 
    let formatted = if thread_failed.load(Ordering::Relaxed) {
        format!("0,0,0,{},{},-1,{},{},{},0,-1,{}", producers, consumers, cqueue.get_id(), pq_args.benchmark_runner, bench_conf.benchmark_id, pq_args.queue_size)
    }
    else {
        let fairness = benchmark_helpers::calc_fairness(ops_per_thread);
        format!("{},{},{},{},{},{},{},{},{},{},{},{}",
            (pushes + pops) as f64 / time_limit as f64,
            pushes,
            pops,
            consumers,
            producers,
            -1,
            cqueue.get_id(),
            pq_args.benchmark_runner,
            bench_conf.benchmark_id,
            fairness,
            -1,
            pq_args.queue_size)
    };
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
    use super::*;

    use crate::arguments::PriorityQueueArgs;

    use crate::benchmarks::test_helpers::test_priority_queue::TestPriorityQueue;

    #[test]
    fn run_basic_prod_con() {
        let pq_args = PriorityQueueArgs::default();
        let bench_conf = BenchConfig {
            args: pq_args.general_args.clone(),
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let queue: TestPriorityQueue<usize, i32> = TestPriorityQueue::new(0);
        if benchmark_prod_con(queue, &bench_conf, &pq_args).is_err() {
            panic!();
        }
    }

    #[test]
    fn run_basic_with_string() {
        let pq_args = PriorityQueueArgs::default();
        let bench_conf = BenchConfig {
            args: pq_args.general_args.clone(),
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let queue: TestPriorityQueue<usize, String> = TestPriorityQueue::new(0);
        if benchmark_prod_con(queue, &bench_conf, &pq_args).is_err() {
            panic!();
        }
    }

    #[test]
    fn run_basic_with_struct() {
        let pq_args = PriorityQueueArgs::default();
        let bench_conf = BenchConfig {
            args: pq_args.general_args.clone(),
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let queue: TestPriorityQueue<usize, PriorityQueueArgs> = TestPriorityQueue::new(0);
        if benchmark_prod_con(queue, &bench_conf, &pq_args).is_err() {
            panic!();
        }
    }
}
