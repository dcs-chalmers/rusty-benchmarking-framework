use std::sync::{atomic::{AtomicBool, Ordering}, Barrier};
use rand::Rng;
use core_affinity::CoreId;
use log::{error, info, trace};

use crate::ConcurrentQueue;
use crate::Handle;


#[allow(clippy::result_unit_err)]
pub fn benchmark_order_box<C>(cqueue: C, thread_count: usize, time_limit: u64, one_socket: bool, delay: usize) -> Result<(), ()>
where 
    C: ConcurrentQueue<Box<i32>>,
    for<'a> &'a C: Send
{
    use std::time::Duration;
    use std::sync::Mutex;

    let barrier = Barrier::new(thread_count + 1);
    let done_pushing = AtomicBool::new(false);
    let order = Mutex::new((1..=10_000_000).collect());
    let mut order2: Vec<i32> = (1..=10_000_000).collect();
    let done_popping = AtomicBool::new(false);
    let was_ordered = AtomicBool::new(true);
    info!("Starting order benchmark with {} threads", thread_count);
    
    // get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
        let mut core_iter = available_cores.into_iter().cycle();

    std::thread::scope(|s| -> Result<(),()>{
        let queue = &cqueue;
        let done_pushing = &done_pushing;
        let barrier = &barrier;
        let &thread_count = &thread_count; 
        let is_one_socket = one_socket;
        let lock: &Mutex<Vec<i32>> = &order;
        let order2 = &mut order2;
        let done_popping = &done_popping;
        let was_ordered = &was_ordered;
        for _i in 0..thread_count{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                barrier.wait();
                while !done_pushing.load(Ordering::Relaxed) {
                    for _ in 0..delay {
                       let _some_num = rand::rng().random::<f64>();
                    }
                    {
                        let mut q = lock.lock().unwrap();
                        let elem = match q.pop() {
                            Some(e) => e,
                            None => {
                                done_pushing.store(true, Ordering::Relaxed);
                                break;
                            },
                        };
                        let elem_c = elem;
                        if handle.push(Box::new(elem)).is_err() {
                            trace!("failed to push {elem_c}");
                            q.push(elem_c);
                            continue;
                        }
                        trace!("Pushed {elem}");
                    }
                }
            }); 
            
        }
        // TODO: Make it quit after it finds that it is unordered
        s.spawn(move || {
            let mut handle = queue.register();
            barrier.wait();
            while !done_pushing.load(Ordering::Relaxed) {
                if let Some(val) = handle.pop() {
                    let value = order2.pop().unwrap();
                    trace!("{} = {}",value, val);
                    std::thread::sleep(Duration::from_millis(1));
                    if value != *val {
                        error!("Not ordered, failed at value {}, should have had value {val}", value);
                        was_ordered.store(false, Ordering::Relaxed);
                        break;
                    } 
                }
            }
        });
        done_popping.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done_pushing.store(true, Ordering::Relaxed);

        println!("Order test over.");
        if was_ordered.load(Ordering::Relaxed) {
            println!("Queue seems ordered");
            Ok(())
        } else {
            println!("Queue was unordered");
            Err(())
        }
    })
}

#[allow(clippy::result_unit_err)]
pub fn benchmark_order_i32<C>(cqueue: C, thread_count: usize, time_limit: u64, one_socket: bool, delay: usize) -> Result<(), ()>
where 
    C: ConcurrentQueue<i32>,
    for<'a> &'a C: Send
{
    use std::time::Duration;
    use std::sync::Mutex;

    let barrier = Barrier::new(thread_count + 1);
    let done_pushing = AtomicBool::new(false);
    let order = Mutex::new((1..=10_000_000).collect());
    let mut order2: Vec<i32> = (1..=10_000_000).collect();
    let done_popping = AtomicBool::new(false);
    let was_ordered = AtomicBool::new(true);
    info!("Starting order benchmark with {} threads", thread_count);
    
    // get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
        let mut core_iter = available_cores.into_iter().cycle();

    std::thread::scope(|s| -> Result<(), ()>{
        let queue = &cqueue;
        let done_pushing = &done_pushing;
        let barrier = &barrier;
        let &thread_count = &thread_count; 
        let is_one_socket = one_socket;
        let lock: &Mutex<Vec<i32>> = &order;
        let order2 = &mut order2;
        let done_popping = &done_popping;
        let was_ordered = &was_ordered;
        for _i in 0..thread_count{
            let mut core : CoreId = core_iter.next().unwrap();
            // if is_one_socket is true, make all thread ids even 
            // (this was used for our testing enviroment to get one socket)
            if is_one_socket {
                core = core_iter.next().unwrap();
            }
            // println!("{:?}", core);
            s.spawn(move || {
                core_affinity::set_for_current(core);
                let mut handle = queue.register();
                barrier.wait();
                while !done_pushing.load(Ordering::Relaxed) {
                    for _ in 0..delay {
                       let _some_num = rand::rng().random::<f64>();
                    }
                    {
                        let mut q = lock.lock().unwrap();
                        let elem = match q.pop() {
                            Some(e) => e,
                            None => {
                                done_pushing.store(true, Ordering::Relaxed);
                                break;
                            },
                        };
                        let elem_c = elem;
                        if handle.push(elem).is_err() {
                            trace!("failed to push {elem_c}");
                            q.push(elem_c);
                            continue;
                        }
                        trace!("Pushed {elem}");
                    }
                }
            }); 
            
        }
        // TODO: Make it quit after it finds that it is unordered
        s.spawn(move || {
            let mut handle = queue.register();
            barrier.wait();
            while !done_pushing.load(Ordering::Relaxed) {
                if let Some(val) = handle.pop() {
                    let value = order2.pop().unwrap();
                    // trace!("{} = {}",value, val);
                    std::thread::sleep(Duration::from_millis(1));
                    if value != val {
                        error!("Not ordered, failed at value {}, should have had value {val}", value);
                        was_ordered.store(false, Ordering::Relaxed);
                        break;
                    } 
                }
            }
        });
        done_popping.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_secs(time_limit));
        done_pushing.store(true, Ordering::Relaxed);

        println!("Order test over.");
        if was_ordered.load(Ordering::Relaxed) {
            println!("Queue seems ordered");
            Ok(())
        } else {
            println!("Queue was unordered");
            Err(())
        }
    })
}
