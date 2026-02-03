#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{epoch, stats};
use log::{debug, error, trace};
use sysinfo::System;

/// Benchmark config struct
/// Needs to be fully filled for benchmarks to be able to run.
pub struct BenchConfig {
    pub args: Args,
    pub date_time: String,
    pub benchmark_id: String,
    pub output_filename: String,
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
