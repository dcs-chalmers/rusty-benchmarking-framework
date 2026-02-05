use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand};
use std::fmt::Display;

/// General arguments for all benchmarks
#[derive(ClapArgs, Debug, Clone)]
pub struct GeneralArgs {
    /// Duration of each benchmark
    #[arg(short, long, default_value_t = 10)]
    pub time_limit: u64,

    /// Attemps to only use one socket. Specific for the developers test environment.
    #[arg(short, long, default_value_t = true, action = ArgAction::SetFalse)]
    pub one_socket: bool,

    /// How many times the chosen benchmark should be run.
    #[arg(short, long, default_value_t = 1)]
    pub iterations: u32,

    /// Set the amount of floating point numbers generated between each operation. Default is 10.
    #[arg(short, long, default_value_t = 10)]
    pub delay: u64,

    /// Set the output path for the result files.
    #[arg(long = "path", default_value_t = String::from("./output"))]
    pub path_output: String,

    /// If set to true, benchmark will output to stdout instead of to files.
    #[arg(long = "write-stdout", default_value_t = false)]
    pub write_to_stdout: bool,

    /// Write benchmark configuration and hardware info to a separate file.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub print_info: bool,

    #[cfg(feature = "memory_tracking")]
    /// The interval of which memory tracking will update [ms].
    #[arg(long, default_value_t = 50)]
    pub memory_tracking_interval: u64,
}

/// Arguments for the FIFO Queue benchmark types
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct FifoQueueArgs {
    /// Count empty pop operations. Off by default.
    #[arg(short, long, default_value_t = false)]
    pub empty_pops: bool,

    /// Set the size of the bounded queues.
    #[arg(short, long, default_value_t = 10000)]
    pub queue_size: u32,

    /// The runner to use for the benchmark
    #[command(subcommand)]
    pub benchmark_runner: FifoQueueBenchmarks,

    /// Prefill the FIFO Queue with default values before running the benchmark.
    #[arg(short, long, default_value_t = 0)]
    pub prefill_amount: u64,

    /// General arguments agnostic to the FIFO Queue
    #[command(flatten)]
    pub general_args: GeneralArgs,
}

/// Benchmark runners for FIFO Queues.
#[derive(Subcommand, Debug)]
pub enum FifoQueueBenchmarks {
    /// A benchmark for measuring throughput using producers and consumers that
    /// are each bound to a thread
    ProdCon(FifoQueueProdConArgs),

    /// A benchmark measuring throughput where a thread will switch between
    /// producing and consuming
    EnqDeq(FifoQueueEnqDeqArgs),

    /// A benchmark measuring throughput where a thread will enqueue an item
    /// and then immediately dequeue it
    EnqDeqPairs(FifoQueueEnqDeqPairsArgs),

    /// Benchmarks how fast the FIFO Queue can complete a breadth-first search
    /// on a graph
    BFS(FifoQueueBFSArgs),
}

#[derive(ClapArgs, Debug)]
pub struct FifoQueueProdConArgs {
    /// Amount of producers to be used
    #[arg(short, long, default_value_t = 20)]
    pub producers: usize,

    /// Amount of consumers to be used
    #[arg(short, long, default_value_t = 20)]
    pub consumers: usize,
}

#[derive(ClapArgs, Debug)]
pub struct FifoQueueEnqDeqArgs {
    /// Set the thread count for the pingpong benchmark.
    #[arg(long = "thread-count", default_value_t = 20)]
    pub thread_count: usize,

    /// Decide the spread of producers/consumers for the pingpong benchmark.
    /// Ex. 0.3 means 30% produce 70% consume.
    #[arg(long = "spread", default_value_t = 0.5)]
    pub spread: f64,
}

#[derive(ClapArgs, Debug)]
pub struct FifoQueueEnqDeqPairsArgs {
    /// Set the thread count for the pingpong benchmark.
    #[arg(long = "thread-count", default_value_t = 20)]
    pub thread_count: usize,
}

#[derive(ClapArgs, Debug)]
pub struct FifoQueueBFSArgs {
    #[arg(short, long, default_value_t = 20)]
    pub thread_count: usize,

    #[arg(short, long)]
    pub graph_file: String,

    #[arg(short, long, default_value_t = false)]
    pub no_verify: bool,
}

/// Arguments for the Priority Queue benchmark types
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct PriorityQueueArgs {
    /// Set the size of the bounded queues.
    #[arg(short, long, default_value_t = 10000)]
    pub queue_size: u32,

    /// The runner to use for the benchmark
    #[command(subcommand)]
    pub benchmark_runner: PriorityQueueBenchmarks,

    /// Prefill the FIFO Queue with default values before running the benchmark.
    #[arg(short, long, default_value_t = 0)]
    pub prefill_amount: u64,

    /// General arguments agnostic to the Priority Queue
    #[command(flatten)]
    pub general_args: GeneralArgs,
}

/// Benchmark runners for priority queues.
#[derive(Subcommand, Debug)]
pub enum PriorityQueueBenchmarks {
    /// A benchmark for measuring throughput using producers and consumers that
    /// are each bound to a thread
    ProdCon(PQProdConArgs),
}

#[derive(ClapArgs, Debug)]
pub struct PQProdConArgs {
    /// Amount of producers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    pub producers: usize,

    /// Amount of consumers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    pub consumers: usize,
}

/// This is used to write the benchmark type to the output.
/// That is why the arguments are discarded.
impl Display for FifoQueueBenchmarks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FifoQueueBenchmarks::ProdCon(_) => write!(f, "ProdCon"),
            FifoQueueBenchmarks::EnqDeq(_) => write!(f, "EnqDeq"),
            FifoQueueBenchmarks::EnqDeqPairs(_) => write!(f, "EnqDeqPairs"),
            // #[cfg(feature = "bfs")]
            FifoQueueBenchmarks::BFS(_) => write!(f, "BFS"),
        }
    }
}

/// This is used to write the benchmark type to the output.
/// That is why the arguments are discarded.
impl Display for PriorityQueueBenchmarks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriorityQueueBenchmarks::ProdCon(_) => write!(f, "ProdCon"),
        }
    }
}

/// This is used in the print_info function.
impl Display for GeneralArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Time limit:             {}", self.time_limit)?;
        writeln!(f, "One socket?:            {}", self.one_socket)?;
        writeln!(f, "Iterations:             {}", self.iterations)?;
        writeln!(f, "Delay:                  {}", self.delay)?;
        writeln!(f, "Output path:            {}", self.path_output)?;
        writeln!(f, "Write to stdout:        {}", self.write_to_stdout)?;
        Ok(())
    }
}

/// Implemented so that tests are easier to write.
impl Default for GeneralArgs {
    fn default() -> Self {
        GeneralArgs {
            time_limit: 1,
            one_socket: true,
            iterations: 1,
            delay: 10,
            path_output: "".to_string(),
            write_to_stdout: true,
            print_info: false,
            #[cfg(feature = "memory_tracking")]
            memory_tracking_interval: 50,
        }
    }
}

/// Implemented for easier testing
impl Default for FifoQueueArgs {
    fn default() -> Self {
        FifoQueueArgs {
            empty_pops: false,
            queue_size: 10000,
            benchmark_runner: FifoQueueBenchmarks::ProdCon(
                FifoQueueProdConArgs {
                    producers: 20,
                    consumers: 20,
                },
            ),
            prefill_amount: 1000,
            general_args: GeneralArgs::default(),
        }
    }
}

/// Implemented for easier testing
impl Default for PriorityQueueArgs {
    fn default() -> Self {
        PriorityQueueArgs {
            queue_size: 10000,
            benchmark_runner: PriorityQueueBenchmarks::ProdCon(
                PQProdConArgs {
                    producers: 20,
                    consumers: 20,
                },
            ),
            prefill_amount: 1000,
            general_args: GeneralArgs::default(),
        }
    }
}
