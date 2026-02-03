# Queue benchmarking framework
This is a project to benchmark different implementations of queues (currently FIFO, LIFO, bounded or unbounded) to measure their output and performance.

# Contents
- [How to use](#how-to-use)
- [Queue implementations](#queue-implementations)
- [Benchmarks](#benchmarks)
  - [Flags](#flags)
  - [Optional features](#optional-features)
- [Add your own queues](#add-your-own-queues)
  - [Order test](#order-test)
  - [Adding C/C++ queues](#adding-cc-queues)
- [Output files](#output-files)
  - [BFS](#bfs)
- [Logging](#logging)

## How to use:
```bash
# Using cargo run
cargo run --release -p <Queue type> --features <Optional features>-- <General optional flags> <Benchmark type> <Optional flags for benchmark type>
# Alternatively
cargo build --release -p <Queue type> --features <Optional features>
./target/release/<Queue type> <General optional flags> <Benchmark type> <Optional flags for benchmark type>
```

To run for a basic lock-based queue:
```bash
# Basic queue, benchmark measures throughput
cargo run -p basic_queue --release -- prod-con
# Basic queue, benchmark measures throughput and memory allocation
cargo run --p basic_queue --features benchmark_core/memory_tracking --release -- prod-con
```
This will compile and run the benchmarking framework. It will run the `prod-con` benchmark on the `basic_queue` implementation and produce a csv file in the `./output` with results from the benchmark, as well as a file with a name starting with `mem` containing information about total memory allocated during the running.

There are several useful scripts located inside the `scripts` folder, as well as a README which describes how to use them.

## Queue implementations
Implemented Rust queues are:
* `array_queue` - A queue from the crate [`crossbeam`](https://crates.io/crates/crossbeam).
* `atomic_queue` - A queue from the crate [`atomic-queue`](https://crates.io/crates/atomic-queue).
* `basic_queue` - A `VecDeque` wrapped in a mutex.  [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/basic_queue/)
* `bounded_ringbuffer` - A simple custom ringbuffer wrapped in a mutex. [Implementation](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/bounded_ringbuffer/)
* `bounded_concurrent_queue` - A bounded queue from the crate [`concurrent-queue`](https://crates.io/crates/concurrent-queue).
* `unbounded_concurrent_queue` - An unbounded queue from the crate [`concurrent-queue`](https://crates.io/crates/concurrent-queue).
* `lf_queue` - An unbounded lock-free queue from the crate [`lf-queue`](https://crates.io/crates/lf-queue)
* `lockfree_queue` - A (supposedly) lock-free unbounded queue from the crate [`lockfree`](https://crates.io/crates/lockfree).
* `ms_queue` - An unbounded lock-free queue based on the [Michael & Scott queue](https://dl.acm.org/doi/10.1145/248052.248106). [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/ms_queue/)
* `scc_queue` - An unbounded lock-free queue from the crate [`scc`](https://crates.io/crates/scc).
* `scc2_queue` - An unbounded lock-free queue from the crate [`scc2`](https://crates.io/crates/scc2).
* `seg_queue` - An unbounded queue from the crate [`crossbeam`](https://crates.io/crates/crossbeam).
* `wf_queue` - A bounded lock-free queue from the crate [`wfqueue`](https://crates.io/crates/wfqueue). Patched [here](https://github.com/WilleBerg/wfqueue) to be able to be compiled.
* `lcrq` - Our Rust implementation of the scalable lock-free unbounded [LCRQ](https://dl.acm.org/doi/10.1145/2517327.2442527). [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/lcrq/) **Requires x86-64**.
* `lprq` - Our Rust implementation of the scalable lock-free unbounded [LPRQ](https://dl.acm.org/doi/abs/10.1145/3572848.3577485) (a portable extension of the LCRQ). [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/lprq/)
* `faaa_queue` - Our Rust implementation of the scalable lock-free unbounded [FAAArrayQueue](https://concurrencyfreaks.blogspot.com/2016/11/faaarrayqueue-mpmc-lock-free-queue-part.html). [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/faaa_queue/)
* `tz_queue_hp` - A lock-free bounded queue based on [this paper](https://dl.acm.org/doi/abs/10.1145/378580.378611). This implementation uses hazard pointers for memory reclamation. [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/tz_queue_hp/)
* `tz_queue_leak` - A lock-free bounded queue based on [this paper](https://dl.acm.org/doi/abs/10.1145/378580.378611). This implementation has no memory reclamation scheme. [Implementation.](https://github.com/dcs-chalmers/rusty-benchmarking-framework/blob/main/queues/tz_queue_leak/)
* `bbq` - A Block Based Bounded Queue based on [this paper](https://www.usenix.org/conference/atc22/presentation/wang-jiawei) from the crate [`bbq-rs`](https://crates.io/crates/bbq-rs). This queue implements a blocking mechanism and thus does not work for `enq-deq`. We fixed this issue and patched it [here](https://github.com/WilleBerg/bbq). The patched version is the one implemented in the framework. **Requires nightly**.

Additionally, there are some C++ queues, which are included with C bindings to pre-existing implementations:
* `boost_queue_cpp` - A bounded lock-free C++ queue from the [`boost`](https://www.boost.org/) C++ library. Requires `boost` to be installed on your system.
* `moodycamel_cpp` - A fast but non-linearizable (only maintains per-thread order) C++ queue ([GitHub](https://github.com/cameron314/concurrentqueue)).
* `lcrq_cpp` - A C++ implementation of the [LCRQ](https://dl.acm.org/doi/10.1145/2517327.2442527) ([Github](https://github.com/pramalhe/ConcurrencyFreaks/blob/master/CPP/queues/LCRQueue.hpp)). **Requires x86-64**.
* `lprq_cpp` - The [LPRQ](https://dl.acm.org/doi/abs/10.1145/3572848.3577485) C++ implementation ([Zenodo](https://zenodo.org/records/7337237)).
* `faaa_queue_cpp` - The [FAAArrayQueue](https://concurrencyfreaks.blogspot.com/2016/11/faaarrayqueue-mpmc-lock-free-queue-part.html) C++ implementation.

There are also the following Rust stacks:
* `lockfree_stack` - A lock-free unbounded stack from the crate [`lockfree`](https://crates.io/crates/lockfree).
* `scc_stack` - An unbounded lock-free stack from the crate [`scc`](https://crates.io/crates/scc).
* `scc2_stack` - An unbounded lock-free stack from the crate [`scc2`](https://crates.io/crates/scc2).


## Benchmarks
You have to choose which type of benchmark you want to run for your queue. They have sub-commands specific to themselves. Use the `--help` flag to print a help text about the sub-commands.
* `prod-con` - Measures throughput and fairness. Threads are either producers or consumers. You can choose the amount of producers and consumers using their respective flags.
* `enq-deq` - Measures throughput and fairness. Threads alternate between enqueueing and dequeueing randomly. You can choose the spread of enqueuers/dequeuers using the `--spread` flag. Using the `--thread-count` flag you can decide how many threads you want to use for the benchmark.
* `bfs` - Measures execution time. Performs a parallell breadth-first search on a graph of your choosing. After the execution, the benchmark will also do a sequential search to verify the parallel solution. The verification can be turned off by passing the `--no-verify` flag. Choose graph file by passing the `--graph-file` flag and specifying the path. The benchmark supports `.mtx` files. You can run several iterations of BFS by passing the `-i` flag, just as in the other benchmarks. The graph file will only be loaded once, and the sequential solution will also only be generated once.
* `enq-deq-pairs` - Measures throughput and fairness. Threads first enqueue an item, then immediately dequeues an item. Use `--thread-count` to change the amount of threads.

### Flags
To use specific values you can add different flags to the run command:
* General flags:
    * `-t`, `--time-limit` for specific time values.
    * `-o`, `--one-socket` to run on one socket (specific for our test environment).
    * `-i`, `--iterations` to specify how many iterations to run the benchmark.
    * `-e`, `--empty-pops` if you want to include empty dequeue operations.
    * `-q`, `--queue-size` to specify the sizes of bounded queues.
    * `-d`, `--delay` to specify amount of floating points generated between each operation. [Default: 10]
    * `--write-stdout` - If you want to output to stdout instead of a file.
    * `-h`, `--help` to print help.
    * `-V` `--version` to print the version of the benchmark.
    * `--path` to change where the output of the benchmark is put.
    * `--print-info` - To create a file with hardware info and benchmark info
* `prod-con` benchmark type sub commands:
    * `-p`, `--producers` for specified amount of producers.
    * `-c`, `--consumers` for specified amount of consumers.
* `enq-deq` benchmark type sub commands:
    * `--spread` - To specify the spread for the `enq-deq` benchmark type.
    * `--thread-count` - To specify the amount of threads in the `enq-deq` benchmark type.
* `enq-deq-pairs` benchmark type sub commands:
    * `--thread-count` - To specify the amount of threads in the `enq-deq-pairs` benchmark type.

### Optional features
* `benchmark_core/memory_tracking` - Writes to a file the memory allocated by the program during the execution. Requires `jemalloc`, so should work on most UNIX systems.
* `silent-release` - Compiles the benchmarking tool without any logging. Need to pass the `--no-default-features`  to work.
* `verbose-release` - Compiles the benchmarking tool with all log levels. Need to pass the `--no-default-features`  to work.

## Add your own queues
To add your own queues to the benchmarking suite, it should be added as a workspace member, preferably in `queues/`. This requires adding it as a member in `Cargo.toml`, and then adding its package folder similarly to the packages in `queues` (it should have the main `main.tex`, with the exception of the selected queue).

Your queue will have to implement the traits found in `src/traits.rs` so that the framework can run it. The traits look like this:
```rust
/// The required queue trait.
pub trait ConcurrentQueue<T> {
    /// Returns a thread-local handle for the queue.
    fn register(&self) -> impl HandleQueue<T>;

    /// Returns the name of the queue.
    fn get_id(&self) -> String;

    /// Used to create a new queue.
    /// `size` is discarded for unbounded queues.
    fn new(size: usize) -> Self;
}

/// The required queue handle trait.
pub trait HandleQueue<T> {
    /// Pushes an item to the queue.
    /// If it fails, returns the item pushed.
    fn push(&mut self, item: T) -> Result<(), T>;

    /// Pops an item from the queue.
    fn pop(&mut self) -> Option<T>;
}
```

You should then be able to run benchmarks on your queue by running for example:
```bash
cargo run -p new_queue_name -r -- -t 1 enq-deq
```

### Order test
In the file `order.rs`, there are two functions that test that the queue dequeues items in the same order that they were enqueued. This function returning `Ok(())` does not mean that the queue always dequeues in order, however it returning `Err(())` does mean that the queue sometimes dequeues out of order. The way we have used these functions is by creating one test per queue that runs one of the two (depending on whether the queue uses `Box` or not). Example:
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

### Adding C/C++ queues
The framework is capable of running benchmarks on C/C++ queues as well. Adding them is not as straightforward as Rust queues, and there probably exists a better way to do it than what is done here. We use [bindgen](https://github.com/rust-lang/rust-bindgen) to generate C bindings for the C/C++ queues. Below, we show how to add a C++ queue. We also recommend looking at the code in the pre-existing C++ queues.

As with all queues, first create a package folder in `queues/`. Then add all the C++ headers in a nested folder (e.g. `cpp_src/`). Then, create two wrapper files `queue_wrapper.cpp` and `queue_wrapper.hpp`. In `queue_wrapper.hpp`, add the functions you want. Typically `create`, `destroy`, `push` and `pop`.
```Cpp
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct YourQueueImpl* YourQueue;

YourQueue create();
void your_queue_destroy()
int your_queue_push(YourQueue q, void* item);
int your_queue_pop(YourQueue q, void** item);

#ifdef __cplusplus
}
#endif
```

Then, in `queue_wrapper.cpp` create the functions, including the required C++ headers:
```CPP
// Include everything you need.
#include <queue_wrapper.hpp>

// The actual implementation
struct YourQueueImpl {
    your::actual::queue queue;

    explicit YourQueueImpl()
        : queue() {}
};

YourQueue your_queue_create() {
    return new YourQueueImpl(capacity);
}

void your_queue_destroy(YourQueue queue) {
    delete queue;
}

int your_queue_push(YourQueue queue, void* item) {
    return queue->queue.push(item) ? 1 : 0;
}

int your_queue_pop(YourQueue queue, void** item) {
    return queue->queue.pop(*item) ? 1 : 0;
}
```
Add build dependencies to your project for the compilation and bindings:

```TOML
[build-dependencies]
bindgen = { version = "0.71.1" }
cc = { version = "1.0" }
```


Finally, create a `build.rs` in the new package to build the C++ code. We suggest copying the file [queues/lcrq_cpp/build.rs](queues/lcrq_cpp/build.rs) as a starting point, adapting the following block with specific configuration for the LCRQ:

```rust
    // Configure for LCRQ
    {
        let queue_location = "cpp_src";
        println!("cargo:rerun-if-changed={}/lcrq_wrapper.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/lcrq_wrapper.cpp", queue_location);
        println!("cargo:rerun-if-changed={}/LCRQueue.hpp", queue_location);
        println!("cargo:rerun-if-changed={}/HazardPointers.hpp", queue_location);

        build.file(format!("{}/lcrq_wrapper.cpp", queue_location))
            .define("USE_LCRQUEUE", None);

        bindgen = bindgen
            .header(format!("{}/lcrq_wrapper.hpp", queue_location))
            .allowlist_function("lcrq_.*")
            .allowlist_type("LCRQ.*")
            .opaque_type("LCRQImpl");
    }
```
This code tells cargo to recompile the C++ code if we change any of its files, and how to build and create bindings for the queue. When adapting for a new queue, you need to specify the files it is dependent on in the top part, and then change `lcrq_wrapper` to `your_queue_wrapper` and `LCRQ` to `YOUR_QUEUE`.

Now all that is left is to create the Rust queue files. Create `lib.rs` and `main.rs` files in the new package. The `main.rs` just uses the queue in the `lib.rs`, as normal rust implementations. The `lib.rs` file just connects to the C bindings, for example as follows:
```rust
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use benchmark_core::traits::{ConcurrentQueue, HandleQueue};

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// A safe Rust wrapper around the C bindings
pub struct YourCppQueue<T> {
    raw: YourQueue,
    phantom_data: std::marker::PhantomData<T>,
}

unsafe impl<T> Send for YourQueue<T> {}
unsafe impl<T> Sync for YourpQueue<T> {}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
impl<T> YourCppQueue<T> {

    pub fn push(&self, item: *mut std::ffi::c_void) -> bool {
        unsafe {
            your_queue_push(self.raw, item) == 1
        }
    }

    pub fn pop(&self) -> Option<*mut std::ffi::c_void> {
        let mut item: *mut std::ffi::c_void = std::ptr::null_mut();
        let success = unsafe { your_queue_pop(self.raw, &mut item) == 1 };
        if success {
            Some(item)
        } else {
            None
        }
    }

}

impl<T> Drop for YourCppQueue<T> {
    fn drop(&mut self) {
        unsafe { your_queue_destroy(self.raw) };
    }
}

struct YourQueueHandle<'a,T> {
    pub q: &'a YourCppQueue<T>
}

impl<T> HandleQueue<T> for YourCppQueueHandle<'_,T> {
    fn push(&mut self, item: T) -> Result<(), T> {
        let ptr: *mut std::ffi::c_void = Box::<T>::into_raw(Box::new(item)) as *mut std::ffi::c_void;
        match self.q.push(ptr) {
            true => Ok(()),
            false => {
                // Really unsure if this is possible
                let reclaimed: Box<T> = unsafe { Box::from_raw(ptr as *mut T) };
                Err(*reclaimed)
            },
        }
    }

    fn pop(&mut self) -> Option<T> {
        let res = self.q.pop()?;
        let val = unsafe { Box::from_raw(res as *const T as *mut T) };
        Some(*val)
    }
}

impl<T> ConcurrentQueue<T> for YourCppQueue<T> {
    fn register(&self) -> impl HandleQueue<T> {
        YourCppQueueHandle {
            q: self,
        }
    }

    fn get_id(&self) -> String {
        String::from("your_queue")
    }

    fn new(capacity: usize) -> Self {
        let raw = unsafe { your_queue_create(capacity as u32) };
        YourCppQueue { raw, phantom_data: std::marker::PhantomData}
    }
}

```

## Output files
If the `--write-stdout` flag is not set, the framework will produce a folder called `./output` and in it will include a .csv file with the headers and results of the entire benchmark. For example, with the command:
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
### BFS
The output file for the BFS benchmark is a little bit different from the other benchmarks. It looks like the following:
| Milliseconds | Queuetype  | Thread Count | Test ID           |
|--------------|------------|--------------|-------------------|
| 3836116      | BasicQueue | 20           | b820a6af3925aa03  |
| 3680283      | BasicQueue | 20           | b820a6af3925aa03  |
| 3797156      | BasicQueue | 20           | b820a6af3925aa03  |
| 3630639      | BasicQueue | 20           | b820a6af3925aa03  |
| 4054568      | BasicQueue | 20           | b820a6af3925aa03  |
| 3725101      | BasicQueue | 20           | b820a6af3925aa03  |
| 3439946      | BasicQueue | 20           | b820a6af3925aa03  |
| 3397534      | BasicQueue | 20           | b820a6af3925aa03  |
| 3611314      | BasicQueue | 20           | b820a6af3925aa03  |
| 3539239      | BasicQueue | 20           | b820a6af3925aa03  |

## Logging
The framework contains a logger, which you can change the level of by changing the environment variable `RUST_LOG`. When compiled in debug mode, there are 5 levels you can choose from (`error` will only print errors, `warn` will print warnings and errors etc.):
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
RUST_LOG=trace ./target/release/${PACKAGE} basic
```
