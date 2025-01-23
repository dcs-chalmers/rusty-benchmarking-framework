use crate::queues::lf_queue::LFQueue;
use std::{
    thread,
    sync::atomic::{AtomicUsize, AtomicBool, Ordering},
    sync::Barrier
};
pub mod queues;

pub fn start_benchmark() {
    let test_q: LFQueue<i32> =  LFQueue {
        lfq: lockfree::queue::Queue::new(),
    };

    let time_limit: u64 = 10;
    let barrier = Barrier::new(20 + 20 + 1);
    let pops  = AtomicUsize::new(0);
    let pushes = AtomicUsize::new(0);
    let done = AtomicBool::new(false);


    let thread_count: i32 = 40;
    
    thread::scope(|s| {
        let queue = &test_q;
        let pushes = &pushes;
        let pops = &pops;
        let done = &done;
        let barrier = &barrier;

        for i in 0..thread_count{
            s.spawn(move || {
                println!("Thread: {}, working", i);
                let handle = queue.register();
                if i % 2 == 0 {
                    //push
                    let mut l_pushes = 0; 
                    barrier.wait();
                    while !done.load(Ordering::Relaxed) {
                        handle.push(i);
                        l_pushes += 1;
                        
                        // println!("Thread: {}, pushed!", i);
                    }
                    pushes.fetch_add(l_pushes, Ordering::Relaxed);
                }
                else {
                    //pop
                    let mut l_pops = 0; 
                    barrier.wait();
                    while !done.load(Ordering::Relaxed) {
                        handle.pop();
                        l_pops += 1;
                        
                        // println!("Thread: {}, popped!", i);
                    }
                    pops.fetch_add(l_pops, Ordering::Relaxed);
                }
            }); 
        }
        println!("Waiting");
        barrier.wait();
        println!("Sleeping");
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        println!("Done");
        done.store(true, Ordering::Relaxed);
        println!("After done");
    });
    println!("into inner pops");
    let pops = pops.into_inner();
    println!("into inner pushes");
    let pushes = pushes.into_inner();

    println!(
        "Throghput: {}",
        (pushes + pops) as f64 / time_limit as f64
    );
    println!("Number of pushes: {}", pushes);
    println!("Number of pops: {}", pops);
}

pub trait ConcurrentQueue<T> {
    fn register(&self) -> impl Handle<T>;
}

pub trait Handle<T> {
    fn push(&self, item: T);
    fn pop(&self) -> Option<T>;
}


