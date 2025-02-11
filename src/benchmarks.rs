use core_affinity::CoreId;
use rand::Rng;
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Barrier};
use crate::{ConcurrentQueue, Args, Handle};
use std::fs::OpenOptions;
use std::io::Write;

/// # Explanation:
/// A macro used to add your queue to the benchmark.
/// * `$feature:&str` - The name of the queue/feature.
/// * `$wrapper` - The queue type. Queue must implement `ConcurrentQueue` trait.
/// * `$desc` - A description to be printed when the queue gets benchmarked.
/// * `$args` - The argument struct created by clap.
/// * `$output_filename` - Name of the file to be written to. TODO: Remove this
#[macro_export]
macro_rules! implement_benchmark {
    ($feature:literal, $wrapper:ty, $desc:expr, $args:expr, $output_filename:expr) => {
        #[cfg(feature = $feature)]
        {
            println!("Running benchmark on: {}", $desc);
            let test_q: $wrapper = <$wrapper>::new($args.queue_size as usize);
            match $args.benchmark.as_str() {
                "basic" => crate::benchmarks::benchmark_throughput(test_q, &$args, &$output_filename)?,
                "pingpong" => crate::benchmarks::benchmark_ping_pong(test_q, &$args, &$output_filename)?,
                _ => crate::benchmarks::benchmark_throughput(test_q, &$args, &$output_filename)?,
            }
        }
    };
}

/// # Explanation:
/// A simple benchmark that measures the throughput of a queue.
/// Has by default a 1ns delay between each operation, but this can be changed
/// through flags passed to the program.
#[allow(dead_code)]
pub fn benchmark_throughput<C>(cqueue: C, config: &Args, filename: &String) -> Result<(), std::io::Error>
where 
    C: ConcurrentQueue<i32> ,
    for<'a> &'a C: Send
{
    let time_limit: u64 = config.time_limit;
    let barrier = Barrier::new(config.consumers + config.producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    println!("Starting throughput benchmark with {} consumer and {} producers", config.consumers, config.producers);
    
    // get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
        let mut core_iter = available_cores.into_iter().cycle();

    let _ = std::thread::scope(|s| -> Result<(), std::io::Error>{
        let queue = &cqueue;
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let consumers = &config.consumers;
        let producers = &config.producers;
        let is_one_socket = &config.one_socket;

        for _ in 0..*producers{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // push
                let mut l_pushes = 0; 
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    handle.push(1);
                    l_pushes += 1;
                    std::thread::sleep(std::time::Duration::from_nanos(config.delay_nanoseconds));
                }
                pushes.fetch_add(l_pushes, Ordering::Relaxed);
            }); 
        }
        for _ in 0..*consumers {
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // pop
                let mut l_pops = 0; 
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    match handle.pop() {
                        Some(_) => l_pops += 1,
                        None => {
                            if config.empty_pops {
                                l_pops += 1;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_nanos(config.delay_nanoseconds));

                }
                pops.fetch_add(l_pops, Ordering::Relaxed);
            }); 
        }
        barrier.wait();
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done.store(true, Ordering::Relaxed);
        Ok(())
    });
    let pops = pops.into_inner();
    let pushes = pushes.into_inner();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&filename)?;
    if config.human_readable {
        writeln!(file, "Throughput: {}\n", (pushes + pops) as f64 / time_limit as f64)?;
        writeln!(file, "Number of pushes: {}\n", pushes)?;
        writeln!(file, "Number of pops: {}\n", pops)?;
    } else {
        writeln!(file, "{},{},{},{},{},{},Basic Throughput",(pushes + pops) as f64 / time_limit as f64, pushes, pops, config.consumers, config.producers, cqueue.get_id())?;
    }

    Ok(())
}

#[allow(dead_code)]
pub fn benchmark_ping_pong<C> (cqueue: C, config: &Args, filename: &String) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<i32> ,
    for<'a> &'a C: Send
{
    let time_limit: u64 = config.time_limit;
    let barrier = Barrier::new(config.consumers + config.producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    println!("Starting pingpong benchmark with {} threads", config.consumers + config.producers);
    
    // get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
        let mut core_iter = available_cores.into_iter().cycle();

    let _ = std::thread::scope(|s| -> Result<(), std::io::Error>{
        let queue = &cqueue;
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let thread_count = config.consumers + config.producers;
        let is_one_socket = &config.one_socket;

        for _ in 0..thread_count{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                let mut l_pushes = 0; 
                let mut l_pops = 0;
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    let is_consumer = rand::rng().random::<bool>();
                    if is_consumer {
                        match handle.pop() {
                            Some(_) => l_pops += 1,
                            None => {
                                if config.empty_pops {
                                    l_pops += 1;
                                }
                            }
                        }
                    } else {
                        handle.push(1);
                        l_pushes += 1;
                    }
                    std::thread::sleep(std::time::Duration::from_nanos(config.delay_nanoseconds));
                }

                pushes.fetch_add(l_pushes, Ordering::Relaxed);
                pops.fetch_add(l_pops, Ordering::Relaxed);
                // println!("{}: Pushed: {}, Popped: {}", i, l_pushes, l_pops)
            }); 
        }
        barrier.wait();
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done.store(true, Ordering::Relaxed);
        Ok(())
    });
    let pops = pops.into_inner();
    let pushes = pushes.into_inner();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&filename)?;
    if config.human_readable {
        writeln!(file, "Throughput: {}\n", (pushes + pops) as f64 / time_limit as f64)?;
        writeln!(file, "Number of pushes: {}\n", pushes)?;
        writeln!(file, "Number of pops: {}\n", pops)?;
    } else {
        writeln!(file, "{},{},{},{},{},{},PingPong",(pushes + pops) as f64 / time_limit as f64, pushes, pops, config.consumers, config.producers, cqueue.get_id())?;
    }
    Ok(())
}
