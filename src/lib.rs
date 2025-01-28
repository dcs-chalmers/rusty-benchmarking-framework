use crate::queues::lf_queue::LFQueue;
use std::{
    thread,
    sync::atomic::{AtomicUsize, AtomicBool, Ordering},
    sync::Barrier
};
use chrono;

use clap::Parser;

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
}

pub fn start_benchmark() {
    let test_q: LFQueue<i32> =  LFQueue {
        lfq: lockfree::queue::Queue::new(),
    };
    let args = Args::parse();


    let time_limit: u64 = args.time_limit;
    let barrier = Barrier::new(args.consumers + args.producers + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    println!("Starting throughput benchmark with {} consumer and {} producers", args.consumers, args.producers);

    // let thread_count: i32 = 40;
    
    thread::scope(|s| {
        let queue = &test_q;
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;
        let consumers = &args.consumers;
        let producers = &args.producers;

        for _ in 0..*producers{
            s.spawn(move || {
                // println!("Thread: {}, working", i);
                let mut handle = queue.register();
                // push
                let mut l_pushes = 0; 
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    handle.push(1);
                    l_pushes += 1;
                    
                    // println!("Thread: {}, pushed!", i);
                }
                pushes.fetch_add(l_pushes, Ordering::Relaxed);
            }); 
        }
        for _ in 0..*consumers {
            s.spawn(move || {
                // println!("Thread: {}, working", i);
                let mut handle = queue.register();
                // pop
                let mut l_pops = 0; 
                barrier.wait();
                while !done.load(Ordering::Relaxed) {
                    handle.pop();
                    l_pops += 1;
                    
                    // println!("Thread: {}, popped!", i);
                }
                pops.fetch_add(l_pops, Ordering::Relaxed);
            }); 
        }
        // println!("Waiting");
        barrier.wait();
        // println!("Sleeping");
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        // println!("Done");
        done.store(true, Ordering::Relaxed);
        // println!("After done");
    });
    // println!("into inner pops");
    let pops = pops.into_inner();
    // println!("into inner pushes");
    let pushes = pushes.into_inner();
    
    let mut outfile = match std::fs::File::create(format!("./output/{}.txt", chrono::offset::Local::now()
        .to_string())) {
        Ok(of) => of,
        Err(e) => {
            eprintln!("Error when creating outfile: {}", e);
            return;
        }
    };
    let mut string = String::new();
    // Format the statistics into the string
    string.push_str(&format!("Throughput: {}\n", (pushes + pops) as f64 / time_limit as f64));
    string.push_str(&format!("Number of pushes: {}\n", pushes));
    string.push_str(&format!("Number of pops: {}\n", pops));

    // Write the string to the file
    use std::io::Write;
    if let Err(e) = outfile.write_all(string.as_bytes()) {
        eprintln!("Error when writing output to outfile: {}", e)
    }
    println!("{}", string);
}

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
}

pub trait Handle<T> {
    fn push(&mut self, item: T);
    fn pop(&mut self) -> Option<T>;
}


