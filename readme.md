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
* `basic` benchmark type sub commands:
    * `-p`, `--producers` for specified amount of producers.
    * `-c`, `--consumers` for specified amount of consumers.
- `ping-pong` benchmark type sub commands:
    * `--spread` - To specify the spread for the `ping-pong` benchmark type.
    * `--thread-count` - To specify the amount of threads in the `ping-pong` benchmark type.
    * `--print-info` - To create a file with hardware info and benchmark info
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
## Add your own queues
TODO
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
