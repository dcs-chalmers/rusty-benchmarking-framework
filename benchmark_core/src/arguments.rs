use clap::{ArgAction, Args as ClapArgs, Parser, Subcommand};
use std::fmt::Display;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
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
    /// Choose which benchmark to run.
    #[command(subcommand)]
    pub benchmark: BenchmarkTypes,
    /// If set to true, benchmark will output to stdout instead of to files.
    #[arg(long = "write-stdout", default_value_t = false)]
    pub write_to_stdout: bool,
    /// Prefill the data structure with values before running the benchmark.
    #[arg(short, long, default_value_t = 0)]
    pub prefill_amount: u64,
    /// Write benchmark configuration and hardware info to a separate file.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub print_info: bool,
    #[cfg(feature = "memory_tracking")]
    /// The interval of which memory tracking will update [ms].
    #[arg(long, default_value_t = 50)]
    pub memory_tracking_interval: u64,
}

/// Possible benchmark types.
#[derive(Subcommand, Debug)]
pub enum BenchmarkTypes {
    Queue(QueueArgs),
    PriorityQueue(PriorityQueueArgs),
}

/// Arguments for the Queue benchmark type
#[derive(ClapArgs, Debug, Clone)]
pub struct QueueArgs {
    /// Count empty pop operations. Off by default.
    #[arg(short, long, default_value_t = false)]
    pub empty_pops: bool,
    /// Set the size of the bounded queues.
    #[arg(short, long, default_value_t = 10000)]
    pub queue_size: u32,
    /// The runner to use for the benchmark
    #[command(subcommand)]
    pub benchmark_runner: QueueBenchmarks,
}

/// Benchmark runners for queues.
#[derive(Subcommand, Debug, Clone)]
pub enum QueueBenchmarks {
    /// ProdCon throughput test. Decide amount of producers and consumers using flags.
    ProdCon(QueueProdConArgs),
    /// A test where each thread performs both consume and produce based on a random floating point
    /// value. Spread is decided using the `--spread` flag.
    EnqDeq(QueueEnqDeqArgs),
    EnqDeqPairs(QueueEnqDeqPairsArgs),
    BFS(QueueBFSArgs),
}

#[derive(ClapArgs, Debug, Clone)]
pub struct QueueProdConArgs {
    /// Amount of producers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    pub producers: usize,
    /// Amount of consumers to be used for basic throughput test.
    #[arg(short, long, default_value_t = 20)]
    pub consumers: usize,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct QueueEnqDeqArgs {
    /// Set the thread count for the pingpong benchmark.
    #[arg(long = "thread-count", default_value_t = 20)]
    pub thread_count: usize,
    /// Decide the spread of producers/consumers for the pingpong benchmark.
    /// Ex. 0.3 means 30% produce 70% consume.
    #[arg(long = "spread", default_value_t = 0.5)]
    pub spread: f64,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct QueueEnqDeqPairsArgs {
    /// Set the thread count for the pingpong benchmark.
    #[arg(long = "thread-count", default_value_t = 20)]
    pub thread_count: usize,
}

#[derive(ClapArgs, Debug, Clone)]
pub struct QueueBFSArgs {
    #[arg(short, long, default_value_t = 20)]
    pub thread_count: usize,
    #[arg(short, long)]
    pub graph_file: String,
    #[arg(short, long, default_value_t = false)]
    pub no_verify: bool,
}

/// Arguments for the PriorityQueue benchmark type
#[derive(ClapArgs, Debug, Clone)]
pub struct PriorityQueueArgs {
    /// Set the size of the bounded queues.
    #[arg(short, long, default_value_t = 10000)]
    pub queue_size: u32,
    /// The runner to use for the benchmark
    #[command(subcommand)]
    pub benchmark_runner: PriorityQueueBenchmarks,
}

/// Benchmark runners for priority queues.
#[derive(Subcommand, Debug, Clone)]
pub enum PriorityQueueBenchmarks {
    /// Producer Consumer throughput test. Decide amount of producers and consumers using flags.
    ProdCon(PQProdConArgs),
}

#[derive(ClapArgs, Debug, Clone)]
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
impl Display for QueueBenchmarks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueBenchmarks::ProdCon(_) => write!(f, "ProdCon"),
            QueueBenchmarks::EnqDeq(_) => write!(f, "EnqDeq"),
            QueueBenchmarks::EnqDeqPairs(_) => write!(f, "EnqDeqPairs"),
            // #[cfg(feature = "bfs")]
            QueueBenchmarks::BFS(_) => write!(f, "BFS"),
        }
    }
}
/// This is used in the print_info function.
impl Display for Args {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Time limit:             {}", self.time_limit)?;
        writeln!(f, "One socket?:            {}", self.one_socket)?;
        writeln!(f, "Iterations:             {}", self.iterations)?;
        writeln!(f, "Delay:                  {}", self.delay)?;
        writeln!(f, "Output path:            {}", self.path_output)?;
        writeln!(f, "Benchmark:              {:?}", self.benchmark)?;
        writeln!(f, "Write to stdout:        {}", self.write_to_stdout)?;
        writeln!(f, "prefill amount:         {}", self.prefill_amount)?;
        Ok(())
    }
}

impl Display for BenchmarkTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Benchmark type:         {:?}", self)?;
        Ok(())
    }
}

/// Implemented so that tests are easier to write.
impl Default for Args {
    fn default() -> Self {
        Args {
            prefill_amount: 0,
            time_limit: 1,
            one_socket: true,
            iterations: 1,
            delay: 10,
            path_output: "".to_string(),
            benchmark: BenchmarkTypes::Queue(QueueArgs {
                empty_pops: false,
                queue_size: 10000,
                benchmark_runner: QueueBenchmarks::ProdCon(QueueProdConArgs {
                    producers: 5,
                    consumers: 5,
                }),
            }),
            write_to_stdout: true,
            print_info: false,
            #[cfg(feature = "memory_tracking")]
            memory_tracking_interval: 50,
        }
    }
}
