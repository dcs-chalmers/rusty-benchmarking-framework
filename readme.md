# Queue benchmarking tool
This is a project to benchmark different implementations of queues (currently 
FIFO, LIFO, bounded or unbounded) to measure their output and performance.

## How to use:
```bash
# Using cargo run
cargo run --release --features <Queue type>,<Optional feature> -- <General optional flags> <Benchmark type> <Optional flags for benchmark type>
# Alternatively
cargo build --release --features <Queue type>,<Optional feature>
./target/release/lockfree-benchmark <General optional flags> <Benchmark type> <Optional flags for benchmark type>
```

To run for a basic lock-based queue:
```bash
# Basic queue, benchmark measures throughput
cargo run --features basic_queue --release -- basic
# Basic queue, benchmark measures throughput and memory allocation
cargo run --features basic_queue,memory_tracking --release -- basic
```
This will compile and run the benchmarking tool. It will run the `basic` benchmark on the `basic_queue` implementation and produce a file in the `./output` with information from the benchmark, as well as a file with a name starting with `mem` containing information about total memory allocated during the running.

There are several useful scripts located inside the `scripts` folder, as well
as a README which describes how to use them.
## Benchmark types
You have to choose which type of benchmark you want to run on your queue. They have sub commands specific to themselves. Use the `--help` flag after specifying queue type to print a help text about the sub commands.
* `basic` - Measures throughput and fairness. Threads are either producers or consumers. You can choose the amount of producers and consumers using their respective flags.
* `ping-pong` - Measures throughput and fairness. Threads alternate between producers and consumers randomly. You can choose the spread of producers/consumers using the `--spread` flag. Using the `--thread-count` flag you can decide how many threads you want to use for the test.
## Queue implementations and features
Implemented queues are:
* `array_queue` - A queue from the crate [`crossbeam`](https://crates.io/crates/crossbeam).
* `atomic_queue` - A queue from the crate [`atomic-queue`](https://crates.io/crates/atomic-queue).
* `basic_queue` - A `VecDeque` with a mutex lock.  [Implementation.](https://github.com/WilleBerg/lockfree-benchmark/blob/main/src/queues/basic_queue.rs) 
* `bounded_ringbuffer` -A simple ringbuffer. [Implementation](https://github.com/WilleBerg/lockfree-benchmark/blob/main/src/queues/bounded_ringbuffer.rs)
* `bounded_concurrent_queue` - A bounded queue from the crate [`concurrent-queue`](https://crates.io/crates/concurrent-queue).
* `lf_queue` - An unbounded lock-free queue from the crate [`lf-queue`](https://crates.io/crates/lf-queue)
* `lockfree_queue` - A lock-free unbounded queue from the crate [`lockfree`](https://crates.io/crates/lockfree).
* `lockfree_stack` - A lock-free unbounded stack from the crate [`lockfree`](https://crates.io/crates/lockfree).
* `ms_queue` - An unbounded lock-free queue. [Implementation.](https://github.com/WilleBerg/lockfree-benchmark/blob/main/src/queues/ms_queue.rs)
* `scc2_queue` - An unbounded lock-free queue from the crate [`scc2`](https://crates.io/crates/scc2).
* `scc2_stack` - An unbounded lock-free stack from the crate [`scc2`](https://crates.io/crates/scc2).
* `scc_queue` - An unbounded lock-free queue from the crate [`scc`](https://crates.io/crates/scc).
* `scc_stack` - An unbounded lock-free stack from the crate [`scc`](https://crates.io/crates/scc).
* `seg_queue` - An unbounded queue from the crate [`crossbeam`](https://crates.io/crates/crossbeam).
* `wfqueue` - A bounded lock-free queue from the crate [`wfqueue`](https://crates.io/crates/wfqueue). Patched [here](https://github.com/WilleBerg/wfqueue) by William to be able to be compiled.
* `boost` - A bounded lock-free C++ queue from the [`boost`](https://www.boost.org/) C++ library. Can be benchmarked using bindings. Required `boost` to be installed on your system. **Experimental**.
* `moodycamel` - [A fast lock-free C++ queue](https://github.com/cameron314/concurrentqueue). Can be benchmarked using bindings. **Experimental**.
* `lcrq` - [An unbounded C++ queue](https://github.com/pramalhe/ConcurrencyFreaks/blob/master/CPP/queues/LCRQueue.hpp). **Experimental**.
* `lprq` - [An unbounded C++ queue](https://zenodo.org/records/7337237). **Experimental**.
* `tz_queue_hp` - A lock-free bounded queue based on [this paper](https://dl.acm.org/doi/abs/10.1145/378580.378611). This implementation uses hazard pointers for memory reclamation. [Implementation.](https://github.com/WilleBerg/lockfree-benchmark/blob/main/src/queues/tsigas_zhang_queue_hp.rs)
* `tz_queue` - A lock-free bounded queue based on [this paper](https://dl.acm.org/doi/abs/10.1145/378580.378611). This implementation has no memory reclamation scheme. [Implementation.](https://github.com/WilleBerg/lockfree-benchmark/blob/main/src/queues/tsigas_zhang_queue.rs)
* `bbq` - A Block Based Bounded Queue based on [this paper](https://www.usenix.org/conference/atc22/presentation/wang-jiawei) from the crate[`bbq-rs`](https://crates.io/crates/bbq-rs). This queue implements [`BlockingQueue`](https://crates.io/crates/blockingqueue) and thus, does not work for `ping-pong`.

### Optional extra feature:
* `memory_tracking` - Writes to a file the memory allocated by the program
during the execution. Requires `jemalloc`, so should work on most UNIX systems.
- `silent-release` - Compiles the benchmarking tool without any logging. Need to pass the `--no-default-features`  to work.
- `verbose-release` - Compiles the benchmarking tool with all log levels. Need to pass the `--no-default-features`  to work.
## Flags
To use specific values you can add different flags to the run command:
* General flags:
    * `-t`, `--time-limit` for specific time values.
    * `-o`, `--one-socket` to run on one socket (specific for our test environment).
    * `-i`, `--iterations` to specify how many iterations to run the benchmark.
    * `-e`, `--empty-pops` if you want to include empty dequeue operations.
    * `-q`, `--queue-size` to specify the sizes of bounded queues.
    * `-d`, `--delay` to specify amount of floating points generated between each operation. Default: 10
    * `--write-stdout` - If you want to output to stdout instead of a file.
    * `-h`, `--help` to print help.
    * `-V` `--version` to print the version of the benchmark.
    * `--path` to change where the output of the benchmark is put.
    * `--print-info` - To create a file with hardware info and benchmark info
* `basic` benchmark type sub commands:
    * `-p`, `--producers` for specified amount of producers.
    * `-c`, `--consumers` for specified amount of consumers.
- `ping-pong` benchmark type sub commands:
    * `--spread` - To specify the spread for the `ping-pong` benchmark type.
    * `--thread-count` - To specify the amount of threads in the `ping-pong` benchmark type.

## Add your own queues
To add your own queues to the benchmark, you first create a new file in `src/queues` for the source code. You then have to add the queue to the `src/queues.rs` file as a feature in the following way:
```rust
// Module name has to be the same as the file name of the file you created.
// The name inside quotation marks will be your feature name and has to
// match 1:1 with what you put inside the Cargo.toml file.
// src/queues.rs
// [...]
#[cfg(feature = "new_queue_name")]
pub mod new_queue;
```
Then, add the feature to the `Cargo.toml` file, under `[features]` as well as in `all_queues`. In this case, the feature name is `new_queue_name`.
```toml
# Cargo.toml
# [...]
# In the brackets, add the dependencies of your queue.
new_queue_name = []

all_queues = [
   [...]
   "new_queue_name"
]
```
To be able to use the queue in the benchmarking suite you will have to add it to the `src/lib.rs` file as well. All you need to do there is add a call to the macro `implement_benchmark!()`.
```rust
// lib.rs
[...]
pub fn start_benchmark() -> Result<(), std::io::Error> {
   [...]
   implement_benchmark!("new_queue_name",          // Feature name
        crate::queues::new_queue::NewQueue<i32>,   // Your queue with desired type
        "New queue",                               // A &str just for printing
        &bench_conf);                              // Benchmark config struct, just pass as reference

   Ok(())
}
[...]
```
You should then be able to run benchmarks on your queue by running for example:
```bash
cargo run -F new_queue_name -r -- -t 1 basic
```
### IDE Help
For your preferred IDE to work (give suggestions, etc.), you will need to make sure to activate the feature in rust-analyzer as well. How to do this is IDE specific, but here it is for Neovim (rustaceanvim):
```lua
vim.g.rustaceanvim = {
  server = {
    on_attach = function(client, bufnr)
    end,
    default_settings = {
      ['rust-analyzer'] = {
          cargo = {
            features = {
               -- This will enable all features inside "all_queues" in Cargo.toml
                "all_queues",
            }
          }
      },
    },
  },
}
```
### Order test
In the file `order.rs` there are two functions that test that the queue dequeues
items in the same order that they were enqueued. This function returning `Ok(())`
does not mean that the queue always dequeues in order, however it returning
`Err(())` does mean that the queue sometimes dequeues out of order. The way we
have used these functions is by creating one test per queue that run one of the
two (depending on if the queue uses `Box` or not). Example:
```rust
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: BasicQueue<i32> = BasicQueue {
            bqueue: BQueue::new() 
        };
        if crate::order::benchmark_order_i32(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
    
    #[test]
    fn test_order() {
        let _ = env_logger::builder().is_test(true).try_init();
        let q: LCRQueue = LCRQueue::new(10);
        if crate::order::benchmark_order_box(q, 20, 5, true, 10).is_err() {
            panic!();
        }
    }
```
## Output files
If the `--write-stdout` flag is not set, the benchmark will produce a folder called `./output` and in it will include a .csv file with the headers and results of the entire benchmark test. For example, with the command:
```bash
cargo run --release --features basic_queue -- -t 1 -i 10 basic
```
| Throughput | Enqueues | Dequeues | Consumers | Producers | Thread Count | Queuetype  | Benchmark | Test ID           | Fairness |
|------------|----------|----------|-----------|-----------|---------------|------------|-----------|-------------------|----------|
| 3836116    | 2022116  | 1814000  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.7928   |
| 3680283    | 1906235  | 1774048  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.7334   |
| 3797156    | 2156525  | 1640631  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.6659   |
| 3630639    | 1893518  | 1737121  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.6256   |
| 4054568    | 2193896  | 1860672  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.5884   |
| 3725101    | 1903091  | 1822010  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.7417   |
| 3439946    | 1719978  | 1719968  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.6608   |
| 3397534    | 1904483  | 1493051  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.7792   |
| 3611314    | 1807886  | 1803428  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.8447   |
| 3539239    | 1952269  | 1586970  | 20        | 20        | -1            | BasicQueue | Basic     | b820a6a3f925aa03  | 0.8757   |

Furthermore, if the `--print-info` flag is set, you will get more specific information about your current test, including some hardware specifications. For example:
```txt
Benchmark done:              Basic
With queue:             Basic Queue
Arguments used in test:

Time limit:             1
One socket?:            true
Iterations:             10
Queue size:             10000
Delay:                  10
Output path:            ./output
Benchmark:              Basic(BasicArgs { producers: 20, consumers: 20 })
Write to stdout:        false
prefill amount:         0


Test ran on hardware specs:
System name:            x
System kernel version:  x
System OS version:      x
Total RAM (in GB):      x
```

## Logging
The benchmark tool contains a logger which you can change the level of by changing the environment variable `RUST_LOG`. When compiled in debug mode, there are 5 levels you can choose from (`error` will only print errors, `warn` will print warnings and errors etc.):
1. `error`
2. `warn`
3. `info`
4. `debug`
5. `trace`
```bash
# Example
RUST_LOG=info cargo run --feature basic_queue -- basic
```
When compiling for debug, it will be able to log all these levels. When compiling for release it will by default only have the `warn` and `error` levels. If you want a release version with more logging you can compile with the `verbose-release` feature. If you want a release completely void of logging you can compile it with the feature `silent-release`. However, you will need to pass the `--no-default-features` flag to cargo as well.
```bash
#  Example verbose
RUST_LOG=verbose cargo run --release --features verbose-release,basic_queue --no-default-features -- basic
# Or in two steps
cargo build --release --features verbose-release,basic_queue --no-default-features
RUST_LOG=trace ./target/release/lockfree-benchmark basic
```
