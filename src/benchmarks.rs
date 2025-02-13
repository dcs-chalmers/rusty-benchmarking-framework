// TODO: Write tests for this module.
use core_affinity::CoreId;
use rand::Rng;
use std::{fmt::Display, sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Barrier}};
use crate::{ConcurrentQueue, Args, Handle};
use std::fs::OpenOptions;
use std::io::Write;

/// Possible benchmark types.
#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Benchmarks {
    /// Basic throughput test. Decide amount of producers and consumers using flags.
    Basic,
    /// A test where each thread performs both consume and produce based on a random floating point
    /// value. Spread is decided using the `--spread` flag.
    PingPong
}

impl Display for Benchmarks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Benchmarks::Basic => write!(f, "Basic"),
            Benchmarks::PingPong => write!(f, "PingPong"),
        }
    }
}

/// Benchmark config struct
/// Needs to be fully filled for benchmarks to be able to run.
pub struct BenchConfig {
    pub args: Args,
    pub date_time: String,
    pub benchmark_id: String,
    pub output_filename: String,
}


/// # Explanation:
/// A macro used to add your queue to the benchmark.
/// * `$feature:&str` - The name of the queue/feature.
/// * `$wrapper` - The queue type. Queue must implement `ConcurrentQueue` trait.
/// * `$desc` - A description to be printed when the queue gets benchmarked.
/// * `$bench_conf` - The benchmark config struct.
#[macro_export]
macro_rules! implement_benchmark {
    ($feature:literal, $wrapper:ty, $desc:expr, $bench_conf:expr) => {
        #[cfg(feature = $feature)]
        {
            println!("Running benchmark on: {}", $desc);
            let test_q: $wrapper = <$wrapper>::new($bench_conf.args.queue_size as usize);

//////////////////////////////////// MEMORY TRACKING ///////////////////////////
            #[cfg(feature = "memory_tracking")]
            let _done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            #[cfg(feature = "memory_tracking")]
            let mem_thread_handle: std::thread::JoinHandle<_>;
            #[cfg(feature = "memory_tracking")]
            {
                use std::sync::atomic::Ordering;
                // TODO: Check if core stuff is possible here as well.
                // let mut core : CoreId = core_iter.next().unwrap();
                // if is_one_socket is true, make all thread ids even 
                // (this was used for our testing enviroment to get one socket)
                // if *is_one_socket {
                //     core = core_iter.next().unwrap();
                // }
                let _done = std::sync::Arc::clone(&_done);
                let benchmark_id = $bench_conf.benchmark_id.clone();
                let queue_type = test_q.get_id();
                let bench_type = $bench_conf.args.benchmark; 
                let to_stdout = $bench_conf.args.write_to_stdout;

                let mut memfile = if !to_stdout {
                    let output_filename = String::from(format!("{}/mem{}", $bench_conf.args.path_output, $bench_conf.date_time));
                    let mut file = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&output_filename)?;
                    writeln!(file, "Memory Allocated,Queuetype,Benchmark,Test ID")?;
                    Some(file)
                } else {
                    println!("Memory Allocated,Queuetype,Benchmark,Test ID");
                    None
                };

                mem_thread_handle = std::thread::spawn(move|| -> Result<(), std::io::Error>{
                    while !_done.load(Ordering::Relaxed) {
                        // Update stats
                        if let Err(e) = epoch::advance() {
                            eprintln!("Error occured while advancing epoch: {}", e);
                        }
                        // Get allocated bytes
                        let allocated = stats::allocated::read().unwrap();

                        let output = format!("{},{},{},{}", allocated, queue_type, bench_type, &benchmark_id);
                        
                        match &mut memfile {
                            Some(file) => writeln!(file, "{}", output)?,
                            None => println!("{}", output),
                        }

                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Ok(())
                });
            }
////////////////////////////////////// MEMORY END //////////////////////////////
            
            match $bench_conf.args.benchmark {
                Benchmarks::Basic     => crate::benchmarks::benchmark_throughput(test_q, $bench_conf)?,
                Benchmarks::PingPong  => crate::benchmarks::benchmark_ping_pong(test_q, $bench_conf)?,
            }

//////////////////////////////////// MEMORY TRACKING ///////////////////////////
            #[cfg(feature = "memory_tracking")]
            {
                use std::sync::atomic::Ordering;
                _done.store(true, Ordering::Relaxed);
                if let Err(e) = mem_thread_handle.join().unwrap() {
                    eprintln!("Error joining memory tracking thread: {}", e);
                }
            }  
////////////////////////////////////// MEMORY END //////////////////////////////
        }
    };
}

/// # Explanation:
/// A simple benchmark that measures the throughput of a queue.
/// Has by default a 1ns delay between each operation, but this can be changed
/// through flags passed to the program.
#[allow(dead_code)]
pub fn benchmark_throughput<C>(cqueue: C, bench_conf: &BenchConfig) -> Result<(), std::io::Error>
where 
    C: ConcurrentQueue<i32> ,
    for<'a> &'a C: Send
{
    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(bench_conf.args.consumers + bench_conf.args.producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    println!("Starting throughput benchmark with {} consumer and {} producers", bench_conf.args.consumers, bench_conf.args.producers);
    
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
        let consumers = &bench_conf.args.consumers;
        let producers = &bench_conf.args.producers;
        let is_one_socket = &bench_conf.args.one_socket;

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
                    std::thread::sleep(std::time::Duration::from_nanos(bench_conf.args.delay_nanoseconds));
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
                            if bench_conf.args.empty_pops {
                                l_pops += 1;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_nanos(bench_conf.args.delay_nanoseconds));

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
    if !bench_conf.args.write_to_stdout {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        if bench_conf.args.human_readable {
            writeln!(file, "Throughput: {}\n", (pushes + pops) as f64 / time_limit as f64)?;
            writeln!(file, "Number of pushes: {}\n", pushes)?;
            writeln!(file, "Number of pops: {}\n", pops)?;
        } else {
            writeln!(file, "{},{},{},{},{},{},{},{}",(pushes + pops) as f64 / time_limit as f64, pushes, pops, bench_conf.args.consumers, bench_conf.args.producers, cqueue.get_id(), bench_conf.args.benchmark, bench_conf.benchmark_id)?;
        }
    } else {
        println!("{},{},{},{},{},{},{},{}",(pushes + pops) as f64 / time_limit as f64, pushes, pops, bench_conf.args.consumers, bench_conf.args.producers, cqueue.get_id(), bench_conf.args.benchmark, bench_conf.benchmark_id);
    }

    Ok(())
}

#[allow(dead_code)]
pub fn benchmark_ping_pong<C> (cqueue: C, bench_conf: &BenchConfig) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<i32> ,
    for<'a> &'a C: Send
{
    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(bench_conf.args.consumers + bench_conf.args.producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    println!("Starting pingpong benchmark with {} threads", bench_conf.args.consumers + bench_conf.args.producers);
    
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
        let thread_count = bench_conf.args.thread_count; 
        let is_one_socket = &bench_conf.args.one_socket;

        for _i in 0..thread_count{
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
                    let random_float = rand::rng().random::<f64>();
                    if random_float > bench_conf.args.spread {
                        match handle.pop() {
                            Some(_) => l_pops += 1,
                            None => {
                                if bench_conf.args.empty_pops {
                                    l_pops += 1;
                                }
                            }
                        }
                    } else {
                        handle.push(1);
                        l_pushes += 1;
                    }
                    std::thread::sleep(std::time::Duration::from_nanos(bench_conf.args.delay_nanoseconds));
                }

                pushes.fetch_add(l_pushes, Ordering::Relaxed);
                pops.fetch_add(l_pops, Ordering::Relaxed);
                if bench_conf.args.human_readable {
                    println!("{}: Pushed: {}, Popped: {}", _i, l_pushes, l_pops)
                }
            }); 
        }
        barrier.wait();
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done.store(true, Ordering::Relaxed);
        Ok(())
    });
    let pops = pops.into_inner();
    let pushes = pushes.into_inner();
    if !bench_conf.args.write_to_stdout {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        if bench_conf.args.human_readable {
            writeln!(file, "Throughput: {}\n", (pushes + pops) as f64 / time_limit as f64)?;
            writeln!(file, "Number of pushes: {}\n", pushes)?;
            writeln!(file, "Number of pops: {}\n", pops)?;
        } else {
            writeln!(file, "{},{},{},{},{},{},{},{}",(pushes + pops) as f64 / time_limit as f64, pushes, pops, bench_conf.args.consumers, bench_conf.args.producers, cqueue.get_id(), bench_conf.args.benchmark, bench_conf.benchmark_id)?;
        }
    } else {
        println!("{},{},{},{},{},{},{},{}",(pushes + pops) as f64 / time_limit as f64, pushes, pops, bench_conf.args.consumers, bench_conf.args.producers, cqueue.get_id(), bench_conf.args.benchmark, bench_conf.benchmark_id);
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use crate::queues::basic_queue::{BQueue, BasicQueue};

    use super::*;
    
    #[test]
    fn run_basic() {
        let args = Args {
            time_limit: 1,
            producers: 5,
            consumers: 5,
            one_socket: true,
            iterations: 1,
            empty_pops: false,
            human_readable: false,
            queue_size: 10000,
            delay_nanoseconds: 1,
            path_output: "".to_string(),
            benchmark: Benchmarks::Basic,
            spread: 0.5,
            write_to_stdout: true,
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new()
        };
        if let Err(_) = benchmark_throughput(basic_queue, &bench_conf) {
            panic!();
        }
    }
    #[test]
    fn run_pingpong() {
        let args = Args {
            time_limit: 1,
            producers: 5,
            consumers: 5,
            one_socket: true,
            iterations: 1,
            empty_pops: false,
            human_readable: false,
            queue_size: 10000,
            delay_nanoseconds: 1,
            path_output: "".to_string(),
            benchmark: Benchmarks::Basic,
            spread: 0.5,
            write_to_stdout: true,
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new()
        };
        if let Err(_) = benchmark_ping_pong(basic_queue, &bench_conf) {
            panic!();
        }
    }
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_macro() -> Result<(), std::io::Error> {
        use jemalloc_ctl::{stats, epoch};

        let args = Args {
            time_limit: 1,
            producers: 5,
            consumers: 5,
            one_socket: true,
            iterations: 1,
            empty_pops: false,
            human_readable: false,
            queue_size: 10000,
            delay_nanoseconds: 1,
            path_output: "".to_string(),
            benchmark: Benchmarks::Basic,
            spread: 0.5,
            write_to_stdout: true,
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        implement_benchmark!("basic_queue",
            BasicQueue<i32>,
            "Testing macro",
            &bench_conf);
        Ok(())
    }
}
