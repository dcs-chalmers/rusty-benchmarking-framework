#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{epoch, stats};
use log::{debug, error, trace};
#[allow(unused_imports)]
use crate::arguments::{Args, Benchmarks};
#[allow(unused_imports)]
use crate::traits::ConcurrentQueue;
use std::fs::OpenOptions;
use std::io::Write;
#[allow(unused_imports)]
use std::sync::atomic::AtomicBool;
use sysinfo::System;

pub mod prod_con;
pub mod enq_deq;
pub mod enq_deq_pairs;
pub mod bfs;

/// Benchmark config struct
/// Needs to be fully filled for benchmarks to be able to run.
pub struct BenchConfig {
    pub args: Args,
    pub date_time: String,
    pub benchmark_id: String,
    pub output_filename: String,
}

/// Create the queue, and run the selected benchmark a set of times
pub fn benchmark_queue<Q>(bench_conf: &BenchConfig, queue_name: &str) ->  Result<(), std::io::Error>
where
    Q: ConcurrentQueue<usize> + Send,
    for<'a> &'a Q: Send
{
    // Create a runner lambda for the different benchmarks, mainly needed for eg. BFS to load graph and so on
    let mut runner: Box<dyn FnMut(Q, &BenchConfig) -> Result<(), std::io::Error>> = match &bench_conf.args.benchmark {
        Benchmarks::ProdCon(_)     => Box::new(move |q, bench_conf| prod_con::benchmark_prod_con(q, bench_conf)),
        Benchmarks::EnqDeq(_)      => Box::new(move |q, bench_conf| enq_deq::benchmark_enq_deq(q, bench_conf)),
        Benchmarks::EnqDeqPairs(_) => Box::new(move |q, bench_conf| enq_deq_pairs::benchmark_enq_deq_pairs(q, bench_conf)),
        Benchmarks::BFS(args) => {
            let (graph, seq_ret_vec, start_node) =
                bfs::pre_bfs_work(
                    Q::new(bench_conf.args.queue_size as usize),
                    args,
                );
            Box::new(move |q, _conf| bfs::benchmark_bfs(
                q,
                &graph,
                bench_conf,
                &seq_ret_vec,
                start_node
                )
            )
        },
    };

    for _current_iteration in 0..bench_conf.args.iterations {
        // Create the queue.
        let test_q: Q = Q::new(bench_conf.args.queue_size as usize);

        // Start memory tracking (if enabled)
        #[cfg(feature = "memory_tracking")]
        let (done, mem_thread_handle) = {
            let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handle = create_mem_tracking_thread(
                bench_conf,
                _current_iteration,
                &test_q,
                &done
            )?;
            (done, handle)
        };

        // Execute the benchmark
        runner(test_q, bench_conf)?;

        // Join the thread again
        debug!("Queue should have been dropped now.");

        // Stop memory tracking (if enabled)
        #[cfg(feature = "memory_tracking")]
        {
            use std::sync::atomic::Ordering;
            debug!("Joining memory thread.");
            done.store(true, Ordering::Relaxed);
            if let Err(e) = mem_thread_handle.join().unwrap() {
                log::error!("Couldn't join memory tracking thread: {}", e);
            }
        }
    }

    if bench_conf.args.print_info {
        print_info(queue_name.to_string(), bench_conf)?;
    }

    Ok(())
}


#[cfg(feature = "memory_tracking")]
pub fn create_mem_tracking_thread<Q,T>(
    bench_conf: &BenchConfig,
    _current_iteration: u32,
    test_q: &Q,
    _done: &std::sync::Arc<AtomicBool>)
-> Result<std::thread::JoinHandle<Result<(), std::io::Error>>, std::io::Error>
where
    Q: ConcurrentQueue<T>
{

    use std::sync::atomic::Ordering;
    // TODO: Check if core stuff is possible here as well.
    // let mut core : CoreId = core_iter.next().unwrap();
    // if is_one_socket is true, make all thread ids even
    // (this was used for our testing enviroment to get one socket)
    // if *is_one_socket {
    //     core = core_iter.next().unwrap();
    // }
    let _done = std::sync::Arc::clone(_done);
    let benchmark_id = bench_conf.benchmark_id.clone();
    let bench_type = format!("{}", bench_conf.args.benchmark);
    let to_stdout = bench_conf.args.write_to_stdout;
    let queue_type = test_q.get_id();

    // Create file if printing to stdout is disabled
    let top_line = "Memory Allocated,Queuetype,Benchmark,Test ID,Iteration";
    let mut memfile = if !to_stdout {
        let output_filename = format!("{}/mem{}", bench_conf.args.path_output, bench_conf.date_time);
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
    let interval = bench_conf.args.memory_tracking_interval;
    debug!("Spawning memory thread.");
    Ok(std::thread::spawn(move|| -> Result<(), std::io::Error>{
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
    }))

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


#[cfg(test)]
mod tests {
    use crate::arguments::EnqDeqArgs;
    use crate::benchmarks::enq_deq_pairs::benchmark_enq_deq_pairs;
    use crate::benchmarks::{
        prod_con::benchmark_prod_con,
        enq_deq::benchmark_enq_deq
    };

    use super::*;

    /// A very simple ConcurrentQueue implementation for testing
    mod test_queue {
        use crate::traits::{ConcurrentQueue, Handle};
        use std::{collections::VecDeque, sync::Mutex};

        pub struct TestQueue<T> {
            queue: Mutex<VecDeque<T>>,
        }

        pub struct TestQueueHandle<'a, T> {
            queue: &'a TestQueue<T>,
        }

        impl<T> Handle<T> for TestQueueHandle<'_, T> {
            fn push(&mut self, item: T) -> Result<(), T>{
                self.queue.queue.lock().unwrap().push_back(item);
                Ok(())
            }

            fn pop(&mut self) -> Option<T> {
                self.queue.queue.lock().unwrap().pop_front()
            }
        }

        impl<T> ConcurrentQueue<T> for TestQueue<T> {
            fn register(&self) -> impl Handle<T> {
                TestQueueHandle {
                    queue: self,
                }
            }

            fn get_id(&self) -> String {
                "test_queue".to_string()
            }

            fn new(_size: usize) -> Self {
                TestQueue {
                    queue: Mutex::new(VecDeque::new())
                }
            }
        }
    }
    use test_queue::TestQueue;

    #[test]
    fn run_basic() {
        let args = Args::default();
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test1".to_string(),
            output_filename: "".to_string()
        };
        let queue: TestQueue<i32> = TestQueue::new(0);
        if benchmark_prod_con(queue, &bench_conf).is_err() {
            panic!();
        }
    }

    #[test]
    fn run_pingpong() {
        let args = Args {
            benchmark: Benchmarks::EnqDeq(EnqDeqArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let queue = TestQueue::<usize>::new(0);
        if benchmark_enq_deq(queue, &bench_conf).is_err() {
            panic!();
        }
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
        let queue = TestQueue::<String>::new(0);
        if benchmark_prod_con(queue, &bench_conf).is_err() {
            panic!();
        }
    }

    #[test]
    fn run_pingpong_with_bool() {
        let args = Args {
            benchmark: Benchmarks::EnqDeq(EnqDeqArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let queue = TestQueue::<bool>::new(0);
        if benchmark_enq_deq(queue, &bench_conf).is_err() {
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
        let queue = TestQueue::<Args>::new(0);
        if benchmark_prod_con(queue, &bench_conf).is_err() {
            panic!();
        }
    }

    #[test]
    fn run_enqdeq_pairs_with_struct() {
        let args = Args {
            benchmark: Benchmarks::EnqDeq(EnqDeqArgs { thread_count: 10, spread: 0.5 }),
            ..Default::default()
        };
        let bench_conf = BenchConfig {
            args,
            date_time: "".to_string(),
            benchmark_id: "test2".to_string(),
            output_filename: "".to_string()
        };
        let queue = TestQueue::<Args>::new(0);
        if benchmark_enq_deq_pairs(queue, &bench_conf).is_err() {
            panic!();
        }
    }
}
