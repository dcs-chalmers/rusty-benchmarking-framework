#[cfg(feature = "memory_tracking")]
use jemalloc_ctl::{epoch, stats};
#[cfg(not(target_os = "windows"))]
use jemallocator::Jemalloc;

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use chrono::Local;
#[allow(unused_imports)]
use log::{self, debug, error, info};
use clap::Parser;
use std::collections::hash_map::DefaultHasher;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
use crate::arguments::Benchmarks;
#[allow(unused_imports)]
use crate::traits::{ConcurrentQueue, Handle};

pub mod benchmarks;
pub mod order;
pub mod queues;
pub mod arguments;
pub mod traits;

/// Starts the actual benchmark.
/// 
/// All work unrelated to the chosen benchmark is done here.
pub fn start_benchmark() -> Result<(), std::io::Error> {
    let args = crate::arguments::Args::parse();
    let date_time = Local::now().format("%Y%m%d%H%M%S").to_string();
    // Create benchmark hashed id
    let benchmark_id = {
        let mut hasher = DefaultHasher::new();
        date_time.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    debug!("Benchmark ID: {}", benchmark_id);
    debug!("Arguments: {:?}", args);

    // Create dir if it doesnt already exist.
    if !std::path::Path::new(&args.path_output).exists() {
        std::fs::create_dir(&args.path_output)?;
    }

    let output_filename = format!("{}/{}", args.path_output, date_time);
    let bench_conf = benchmarks::BenchConfig {
        args,
        date_time,
        benchmark_id,
        output_filename,
    };
    let columns = match bench_conf.args.benchmark {
        #[cfg(feature = "bfs")]
        Benchmarks::BFS(_) => {
            "Milliseconds,Queuetype"
        },
        _ => {
            "Throughput,Enqueues,Dequeues,Consumers,Producers,Thread Count,Queuetype,Benchmark,Test ID,Fairness"
        }
    };
    if bench_conf.args.write_to_stdout {
        println!("{columns}")
    } else {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&bench_conf.output_filename)?;
        writeln!(file, "{columns}")?;
    }
    implement_benchmark!(
        "lockfree_queue",
        crate::queues::lockfree_queue::LockfreeQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "basic_queue",
        crate::queues::basic_queue::BasicQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("bounded_concurrent_queue",
        crate::queues::bounded_concurrent_queue::BoundedCQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("array_queue",
        crate::queues::array_queue::AQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "bounded_ringbuffer",
        crate::queues::bounded_ringbuffer::BoundedRingBuffer<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "atomic_queue",
        crate::queues::atomic_queue::AtomicQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "scc_queue",
        crate::queues::scc_queue::SCCQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "scc2_queue",
        crate::queues::scc2_queue::SCC2Queue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "lf_queue",
        crate::queues::lf_queue::LFQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "wfqueue",
        crate::queues::wfqueue::WFQueue<Box<usize>>,
        &bench_conf
    );
    implement_benchmark!(
        "lockfree_stack",
        crate::queues::lockfree_stack::LockfreeStack<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "scc_stack",
        crate::queues::scc_stack::SCCStack<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "scc2_stack",
        crate::queues::scc2_stack::SCC2Stack<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "ms_queue",
        crate::queues::ms_queue::MSQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "boost",
        crate::queues::boost::BoostCppQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "moodycamel",
        crate::queues::moodycamel::MoodyCamelCppQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "lcrq",
        crate::queues::lcrq::LCRQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "lprq",
        crate::queues::lprq::LPRQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("tz_queue",
        crate::queues::tsigas_zhang_queue::TZQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("tz_queue_hp",
        crate::queues::tsigas_zhang_queue_hp::TZQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("bbq",
        crate::queues::bbq::BBQueue<usize>,
        &bench_conf
    );
    implement_benchmark!("seg_queue",
        crate::queues::seg_queue::SQueue<usize>,
        &bench_conf
    );
    implement_benchmark!(
        "faa_array_queue",
        crate::queues::faa_array_queue::FAAArrayQueue<usize>,
        &bench_conf
    );

    Ok(())
}
