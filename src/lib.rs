#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;
#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{stats, epoch};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, Barrier};
use chrono::Local;
use core_affinity::CoreId;
use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
use rand::Rng;

pub mod queues;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // Duration of test
    #[arg(short, long, default_value_t = 10)]
    time_limit: u64,
    #[arg(short, long, default_value_t = 20)]
    producers: usize,
    #[arg(short, long, default_value_t = 20)]
    consumers: usize,
    #[arg(short, long, default_value_t = true)]
    one_socket: bool,
    #[arg(short, long, default_value_t = 1)]
    iterations: u32,
    #[arg(short, long, default_value_t = false)]
    empty_pops: bool,
    #[arg(long, default_value_t = false)]
    human_readable: bool,
    #[arg(short, long, default_value_t = 10000)]
    queue_size: u32,
    #[arg(short, long, default_value_t = 1)]
    delay_nanoseconds: u64,
    #[arg(long = "path", default_value_t = String::from("./output"))]
    path_output: String,
    #[arg(short,long,default_value_t = String::from("basic"))]
    benchmark: String

}

macro_rules! implement_benchmark {
    ($feature:literal, $wrapper:ty, $desc:expr, $args:expr, $output_filename:expr) => {
        #[cfg(feature = $feature)]
        {
            println!("Running benchmark on: {}", $desc);
            let test_q: $wrapper = <$wrapper>::new($args.queue_size as usize);
            match $args.benchmark.as_str() {
                "basic" => benchmark_throughput(test_q, &$args, &$output_filename)?,
                "pingpong" => benchmark_ping_pong(test_q, &$args, &$output_filename)?,
                _ => benchmark_throughput(test_q, &$args, &$output_filename)?,
            }
        }
    };
}

pub fn start_benchmark() -> Result<(), std::io::Error> {
    let args = Args::parse();
    let _done = Arc::new(AtomicBool::new(false));
    #[cfg(feature = "memory_tracking")]
    let mem_thread_handle: std::thread::JoinHandle<_>;
    #[cfg(feature = "memory_tracking")]
    {
        // TODO: Check if core stuff is possible here as well.
        // let mut core : CoreId = core_iter.next().unwrap();
        // if is_one_socket is true, make all thread ids even 
        // (this was used for our testing enviroment to get one socket)
        // if *is_one_socket {
        //     core = core_iter.next().unwrap();
        // }
        let output_filename = String::from(format!("{}/mem{}", args.path_output, Local::now().format("%Y%m%d%H%M%S").to_string()));
        let mut memfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&output_filename)?;
        let _done = Arc::clone(&_done);
        mem_thread_handle = std::thread::spawn(move|| -> Result<(), std::io::Error>{
            
            while !_done.load(Ordering::Relaxed) {
                // Update stats
                if let Err(e) = epoch::advance() {
                    eprintln!("Error occured while advancing epoch: {}", e);
                }
                // Get allocated bytes
                let allocated = stats::allocated::read().unwrap();
                writeln!(memfile, "{}", allocated)?;

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(())
        });
    }
    let output_filename = String::from(format!("{}/{}", args.path_output, Local::now().format("%Y%m%d%H%M%S").to_string()));
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&output_filename)?;
    writeln!(file, "Throughput,Enqueues,Dequeues,Consumers,Producers,Queuetype,Benchmark")?;
    for i in 0..args.iterations {
        if args.human_readable {
            writeln!(file, "Results from iteration {}:", i)?;
        }
        implement_benchmark!("lockfree_queue",
            crate::queues::lf_queue::LFQueue<i32>,
            "lockfree::queue:Queue",
            args,
            output_filename);
        implement_benchmark!("basic_queue",
            crate::queues::basic_queue::BasicQueue<i32>,
            "Basic Queue",
            args,
            output_filename);
        implement_benchmark!("concurrent_queue",
            crate::queues::concurrent_queue::CQueue<i32>,
            "concurrent_queue::ConcurrentQueue",
            args,
            output_filename);
        implement_benchmark!("array_queue",
            crate::queues::array_queue::AQueue<i32>,
            "crossbeam::queue::ArrayQueue",
            args,
            output_filename);
        implement_benchmark!("bounded_ringbuffer",
            crate::queues::bounded_ringbuffer::BoundedRingBuffer<i32>,
            "Bounded ringbuffer",
            args,
            output_filename);
    }
    #[cfg(feature = "memory_tracking")]
    {
        _done.store(true, Ordering::Relaxed);
        if let Err(e) = mem_thread_handle.join().unwrap() {
            eprintln!("Error joining memory tracking thread: {}", e);
        }
    }  
    Ok(())
}

#[allow(dead_code)]
fn benchmark_throughput<C>(cqueue: C, config: &Args, filename: &String) -> Result<(), std::io::Error>
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
fn benchmark_ping_pong<C> (cqueue: C, config: &Args, filename: &String) -> Result<(), std::io::Error>
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

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
    fn get_id(&self) -> String;
    fn new(size: usize) -> Self;
}

pub trait Handle<T> {
    fn push(&mut self, item: T);
    fn pop(&mut self) -> Option<T>;
}
