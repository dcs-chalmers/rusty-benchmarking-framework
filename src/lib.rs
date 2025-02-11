#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;
#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{stats, epoch};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use std::sync::{atomic::AtomicBool, Arc};
use chrono::Local;
use clap::Parser;
use std::fs::OpenOptions;
use std::io::Write;
pub mod queues;
pub mod benchmarks;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    // Duration of test
    #[arg(short, long, default_value_t = 10)]
    time_limit: u64,
    // Amount of producers to be used for basic throughput test.
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

pub fn start_benchmark() -> Result<(), std::io::Error> {
    let args = Args::parse();
    let _done = Arc::new(AtomicBool::new(false));
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
        use std::sync::atomic::Ordering;
        _done.store(true, Ordering::Relaxed);
        if let Err(e) = mem_thread_handle.join().unwrap() {
            eprintln!("Error joining memory tracking thread: {}", e);
        }
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
