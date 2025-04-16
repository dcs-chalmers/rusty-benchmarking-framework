use log::{debug, error, trace};
#[allow(unused_imports)]
use crate::arguments::{Args, Benchmarks};
use std::fs::OpenOptions;
use std::io::Write;
use sysinfo::System;

pub mod throughput;
pub mod ping_pong;
#[cfg(feature = "bfs")]
pub mod bfs;

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
/// * `$feature:&str`   - The name of the queue/feature.
/// * `$wrapper`        - The queue type. Queue must implement `ConcurrentQueue` trait.
/// * `$bench_conf`     - The benchmark config struct.
#[macro_export]
macro_rules! implement_benchmark {
    ($feature:literal, $wrapper:ty, $bench_conf:expr) => {
        #[cfg(feature = $feature)]
        {
            #[cfg(feature = "bfs")]
            let (graph, seq_ret_vec, start_node) = 
                $crate::benchmarks::bfs::pre_bfs_work(
                    <$wrapper>::new($bench_conf.args.queue_size as usize),
                    $bench_conf,
                );
            for _current_iteration in 0..$bench_conf.args.iterations {
                // Create the queue.
                let test_q: $wrapper = <$wrapper>::new($bench_conf.args.queue_size as usize);
                {
                    debug!("Prefilling queue with {} items.", $bench_conf.args.prefill_amount);
                    let mut tmp_handle = test_q.register();
                    for _ in 0..$bench_conf.args.prefill_amount {
                        tmp_handle.push(Default::default()).expect("Queue size too small");
                    } 
                }
//////////////////////////////////////// MEMORY TRACKING ///////////////////////////
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
                    let bench_type = format!("{}", $bench_conf.args.benchmark);
                    let to_stdout = $bench_conf.args.write_to_stdout;
                    let queue_type = test_q.get_id();

                    // Create file if printing to stdout is disabled
                    let top_line = "Memory Allocated,Queuetype,Benchmark,Test ID,Iteration";
                    let mut memfile = if !to_stdout {
                        let output_filename = format!("{}/mem{}", $bench_conf.args.path_output, $bench_conf.date_time);
                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(&output_filename)?;
                        if _current_iteration == 0 {
                            writeln!(file, "{}", top_line)?;
                        }
                        Some(file)
                    } else {
                        if _current_iteration == 0 {
                            println!("{}", top_line);
                        }
                        None
                    };
                    
                    // Spawn thread to check total memory allocated every 50ms
                    let interval = $bench_conf.args.memory_tracking_interval;
                    debug!("Spawning memory thread.");
                    mem_thread_handle = std::thread::spawn(move|| -> Result<(), std::io::Error>{
                        while !_done.load(Ordering::Relaxed) {
                            // Update stats
                            if let Err(e) = epoch::advance() {
                                eprintln!("Error occured while advancing epoch: {}", e);
                            }
                            // Get allocated bytes
                            let allocated = stats::allocated::read().unwrap();

                            let output = format!("{},{},{},{},{}", allocated, queue_type, bench_type, &benchmark_id, _current_iteration);
                            
                            match &mut memfile {
                                Some(file) => writeln!(file, "{}", output)?,
                                None => println!("{}", output),
                            }

                            std::thread::sleep(std::time::Duration::from_millis(interval));
                        }
                        Ok(())
                    });
                }
////////////////////////////////////////// MEMORY END //////////////////////////////
                // Select which benchmark to use
                match $bench_conf.args.benchmark {
                    Benchmarks::Basic(_)     => $crate::benchmarks::throughput::benchmark_throughput(test_q, $bench_conf)?,
                    Benchmarks::PingPong(_)  => $crate::benchmarks::ping_pong::benchmark_ping_pong(test_q, $bench_conf)?,
                    #[cfg(feature = "bfs")]
                    Benchmarks::BFS(_)       => {
                        $crate::benchmarks::bfs::benchmark_bfs(
                            test_q,
                            &graph,
                            $bench_conf,
                            &seq_ret_vec,
                            start_node
                            )?
                    },
                }

//////////////////////////////////////// MEMORY TRACKING ///////////////////////////
                // Join the thread again
                debug!("Queue should have been dropped now.");
                #[cfg(feature = "memory_tracking")]
                {
                    use std::sync::atomic::Ordering;
                    debug!("Joining memory thread.");
                    _done.store(true, Ordering::Relaxed);
                    if let Err(e) = mem_thread_handle.join().unwrap() {
                        log::error!("Couldnt join memory tracking thread: {}", e);
                    }
                }  
////////////////////////////////////// MEMORY END //////////////////////////////
            }
            if $bench_conf.args.print_info {
                $crate::benchmarks::print_info($feature.to_string(), $bench_conf)?;
            }
        }
    };
    
}

/// Calculates the fairness based on paper: 
/// [A Study of the Behavior of Synchronization Methods in Commonly Used Languages and Systems](https://ieeexplore.ieee.org/document/6569906).
pub fn calc_fairness(ops_per_thread: Vec<usize>) -> f64 {
    debug!("Calculating fairness");
    let sum: usize = ops_per_thread.iter().sum();

    let length: f64 = ops_per_thread.len() as f64;
    debug!("The vector {:?}", ops_per_thread);
    debug!("Sum: {}, Length: {}",sum, length);

    // The thread that does the least amount of ops
    let minop: f64 = match ops_per_thread.iter().min() {
        Some(&val) => val as f64,
        None => {
            error!("No record of operations: {:?}", ops_per_thread);
            panic!();
        }
    };
    trace!("Minop fairness: {}", minop);

    // The thread that does the most amount of ops
    let maxop: f64 = match ops_per_thread.iter().max() {
        Some(&val) => val as f64,
        None => {
            error!("No record of operations: {:?}", ops_per_thread);
            panic!();
        }
    };
    trace!("Maxop fairness: {}", maxop);
    
    let fairness: f64 = f64::min((length * minop) /  sum as f64, sum as f64 / (length * maxop));
    
    debug!("Calculated fairness: {}", fairness);
    fairness
}



/// Function to print the specifications of the hardware used and the benchmnark configs that ran
pub fn print_info(queue: String, bench_conf: &BenchConfig) -> Result<(), std::io::Error>{
    // Create file if printing to stdout is disabled
    if bench_conf.args.write_to_stdout {
        return Ok(());
    }
    let memfile = {
        let output_filename = format!("{}/info{}.txt", bench_conf.args.path_output, bench_conf.benchmark_id);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&output_filename)?;
        Some(file)
    };
    let num: u64 = 1000;
    let sys = System::new_all();
    if let Some(mut file) = memfile {
        writeln!(file, "Benchmark done:              {}", bench_conf.args.benchmark)?;
        writeln!(file, "With queue:             {}", queue)?;

        writeln!(file, "Arguments used in test:")?;
        writeln!(file, "\n{}\n", bench_conf.args)?;

        writeln!(file, "Test ran on hardware specs:")?;
        writeln!(file, "System name:            {}", System::name().unwrap())?;
        writeln!(file, "System kernel version:  {}", System::kernel_version().unwrap())?;
        writeln!(file, "System OS version:      {}", System::os_version().unwrap())?;
        writeln!(file, "Total RAM (in GB):      {:?}", sys.total_memory()/(num.pow(3)))?;

    }
    else {
        eprintln!("Error producing info file")
    }
    Ok(())
}


#[cfg(feature = "tests_benchmark")]
#[cfg(test)]
mod tests {
    use crate::queues::basic_queue::{BQueue, BasicQueue};
    use crate::arguments::PingPongArgs;
    use crate::benchmarks::{
        throughput::benchmark_throughput,
        ping_pong::benchmark_ping_pong
    };
    use crate::traits::{ConcurrentQueue, Handle};

    use super::*;
    
    #[test]
    fn run_basic() {
        let args = Args::default();
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new()
        };
        if benchmark_throughput(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
    #[test]
    fn run_pingpong() {
        let args = Args {
            benchmark: Benchmarks::PingPong(PingPongArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
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
        if benchmark_ping_pong(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_macro() -> Result<(), std::io::Error> {
        #[allow(unused_imports)]
        use jemalloc_ctl::{stats, epoch};

        let args = Args::default();
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        implement_benchmark!("basic_queue",
            BasicQueue<i32>,
            &bench_conf);
        Ok(())
    }
    #[test]
    fn run_basic_with_string() {
        let args = Args::default();
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<String> = BasicQueue {
            bqueue: BQueue::new()
        };
        if benchmark_throughput(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
    #[test]
    fn run_pingpong_with_string() {
        let args = Args {
            benchmark: Benchmarks::PingPong(PingPongArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<String> = BasicQueue {
            bqueue: BQueue::new()
        };
        if benchmark_ping_pong(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
    #[test]
    fn run_basic_with_struct() {
        let args = Args::default();
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<Args> = BasicQueue {
            bqueue: BQueue::new()
        };
        if benchmark_throughput(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
    #[test]
    fn run_pingpong_with_struct() {
        let args = Args {
            benchmark: Benchmarks::PingPong(PingPongArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let basic_queue: BasicQueue<Args> = BasicQueue {
            bqueue: BQueue::new()
        };
        if benchmark_ping_pong(basic_queue, &bench_conf).is_err() {
            panic!();
        }
    }
}
