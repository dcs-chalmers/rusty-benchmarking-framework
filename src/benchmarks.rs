use core_affinity::CoreId;
use log::{debug, error, info, trace};
use rand::Rng;
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Barrier};
use crate::{Args, Benchmarks, ConcurrentQueue, Handle};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc;
use sysinfo::System;

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
            for current_iteration in 0..$bench_conf.args.iterations {
                info!("Running benchmark on: {}", $desc);
                let test_q: $wrapper = <$wrapper>::new($bench_conf.args.queue_size as usize);
                {
                    let mut tmp_handle = test_q.register();
                    for _ in 0..$bench_conf.args.prefill_amount {
                        tmp_handle.push(Default::default()).expect("Queue size too small");
                    } 
                }
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
                    let bench_type = format!("{}", $bench_conf.args.benchmark);
                    let to_stdout = $bench_conf.args.write_to_stdout;
                    
                    // Create file if printing to stdout is disabled
                    let top_line = "Memory Allocated,Queuetype,Benchmark,Test ID,Iteration";
                    let mut memfile = if !to_stdout {
                        let output_filename = format!("{}/mem{}", $bench_conf.args.path_output, $bench_conf.date_time);
                        let mut file = OpenOptions::new()
                            .append(true)
                            .create(true)
                            .open(&output_filename)?;
                        if current_iteration == 0 {
                            writeln!(file, "{}", top_line)?;
                        }
                        Some(file)
                    } else {
                        if current_iteration == 0 {
                            println!("{}", top_line);
                        }
                        None
                    };
                    
                    // Spawn thread to check total memory allocated every 50ms
                    mem_thread_handle = std::thread::spawn(move|| -> Result<(), std::io::Error>{
                        while !_done.load(Ordering::Relaxed) {
                            // Update stats
                            if let Err(e) = epoch::advance() {
                                eprintln!("Error occured while advancing epoch: {}", e);
                            }
                            // Get allocated bytes
                            let allocated = stats::allocated::read().unwrap();

                            let output = format!("{},{},{},{},{}", allocated, queue_type, bench_type, &benchmark_id, current_iteration);
                            
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
                // Select which benchmark to use
                match $bench_conf.args.benchmark {
                    Benchmarks::Basic(_)     => $crate::benchmarks::benchmark_throughput(test_q, $bench_conf)?,
                    Benchmarks::PingPong(_)  => $crate::benchmarks::benchmark_ping_pong(test_q, $bench_conf)?,
                }

    //////////////////////////////////// MEMORY TRACKING ///////////////////////////
                // Join the thread again
                debug!("Queue should have been dropped now, joining memory thread.");
                #[cfg(feature = "memory_tracking")]
                {
                    use std::sync::atomic::Ordering;
                    
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    _done.store(true, Ordering::Relaxed);
                    if let Err(e) = mem_thread_handle.join().unwrap() {
                        log::error!("Couldnt join memory tracking thread: {}", e);
                    }
                }  
////////////////////////////////////// MEMORY END //////////////////////////////
            }
            $crate::benchmarks::print_info($desc.to_string(), $bench_conf)?;
        }
    };
    
}

/// # Explanation:
/// A simple benchmark that measures the throughput of a queue.
/// Has by default a 1ns delay between each operation, but this can be changed
/// through flags passed to the program.
#[allow(dead_code)]
pub fn benchmark_throughput<C, T>(cqueue: C, bench_conf: &BenchConfig) -> Result<(), std::io::Error>
where 
    C: ConcurrentQueue<T>,
    T: Default,
    for<'a> &'a C: Send
{
    let producers = bench_conf
        .get_producers()
        .expect("Should not be able to get here if Benchmark != Basic");
    let consumers = bench_conf
        .get_consumers()
        .expect("Should not be able to get here if Benchmark != Basic");

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

        for i in 0..producers{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if *is_one_socket {
                core = core_iter.next().unwrap();
            }
            trace!("Thread: {} Core: {:?}", i, core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // push
                let mut l_pushes = 0; 
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    // NOTE: Maybe we should care about this result?
                    let _ = handle.push(T::default());
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
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                // pop
                let mut l_pops = 0; 
                let mut empty_pops = 0;
                barrier.wait();
                // TODO: add empty pops probably to fairness calculations
                while !done.load(Ordering::Relaxed) {
                    match handle.pop() {
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
    let fairness = calc_fairness(ops_per_thread);
    let formatted = format!("{},{},{},{},{},{},{},{},{},{}",
        (pushes + pops) as f64 / time_limit as f64,
        pushes,
        pops,
        consumers,
        producers,
        -1,
        cqueue.get_id(),
        bench_conf.args.benchmark,
        bench_conf.benchmark_id,
        fairness);
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


/// Calculates the fairness based on paper: [A Study of the Behavior of Synchronization Methods in Commonly Used Languages and Systems](https://ieeexplore.ieee.org/document/6569906).
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

#[allow(dead_code)]
pub fn benchmark_ping_pong<C, T> (cqueue: C, bench_conf: &BenchConfig) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<T>,
T: Default,
    for<'a> &'a C: Send
{
    let thread_count = bench_conf
        .get_thread_count()
        .expect("Should not get here if Benchmark != PingPong");
    let time_limit: u64 = bench_conf.args.time_limit;
    let barrier = Barrier::new(thread_count + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    info!("Starting pingpong benchmark with {} threads", thread_count);
    


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
        let &thread_count = &thread_count; 
        let &spread = &bench_conf
            .get_spread()
            .expect("Should not get here if Benchmark != PingPong");
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
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                let mut l_pushes = 0; 
                let mut l_pops = 0;
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    let random_float = rand::rng().random::<f64>();
                    if random_float > spread {
                        match handle.pop() {
                            Some(_) => l_pops += 1,
                            None => {
                                if bench_conf.args.empty_pops {
                                    l_pops += 1;
                                }
                            }
                        }
                    } else {
                        // NOTE: Should we care about this Result?
                        let _ = handle.push(T::default());
                        l_pushes += 1;
                    }
                    for _ in 0..bench_conf.args.delay {
                        let _some_num = rand::rng().random::<f64>();
                    }
                }

                pushes.fetch_add(l_pushes, Ordering::Relaxed);
                pops.fetch_add(l_pops, Ordering::Relaxed);
                tx.send(l_pops + l_pushes).unwrap();
                trace!("{}: Pushed: {}, Popped: {}", _i, l_pushes, l_pops)
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
    let fairness = calc_fairness(ops_per_thread);
    let formatted = format!("{},{},{},{},{},{},{},{},{},{}",
        (pushes + pops) as f64 / time_limit as f64,
        pushes,
        pops,
        -1,
        -1,
        thread_count,
        cqueue.get_id(),
        bench_conf.args.benchmark,
        bench_conf.benchmark_id,
        fairness);
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
        writeln!(file, "Test done:              {}", bench_conf.args.benchmark)?;
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

impl BenchConfig {
    pub fn get_thread_count(&self) -> Option<usize> {
        match &self.args.benchmark {
            Benchmarks::PingPong(s)=> Some(s.thread_count),
            _ => None,
        }  
    }
    fn get_spread(&self) -> Option<f64> {
        if let Benchmarks::PingPong(s) = &self.args.benchmark {
            return Some(s.spread);
        }
        None
    }
    fn get_consumers(&self) -> Option<usize> {
        if let Benchmarks::Basic(s) = &self.args.benchmark {
            return Some(s.consumers);
        }
        None
    }
    fn get_producers(&self) -> Option<usize> {
        if let Benchmarks::Basic(s) = &self.args.benchmark {
            return Some(s.producers);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::queues::basic_queue::{BQueue, BasicQueue};

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
            benchmark: Benchmarks::PingPong(crate::PingPongArgs { thread_count: 10, spread: 0.5 }),
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
            "Testing macro",
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
            benchmark: Benchmarks::PingPong(crate::PingPongArgs { thread_count: 10, spread: 0.5 }),
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
            benchmark: Benchmarks::PingPong(crate::PingPongArgs { thread_count: 10, spread: 0.5 }),
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
