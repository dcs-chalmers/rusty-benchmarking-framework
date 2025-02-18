# Lock-free benchmarking tool
This is a project to test different implementations of lock-free based data structures to measure their output and performance.

## How to use:
To run the benchmark, first clone the repository.
Then compile for the queue you want to test. Do this by using the `--features`
flag in cargo. To run for a basic lock-based queue:
```bash
# Create the output folder first
mkdir output
# Basic queue, benchmark measures throughput
cargo run --features basic_queue --release
# Basic queue, benchmark measures throughput and memory allocation
cargo run --features basic_queue,memory_tracking --release
```
## Queue implementations and features
Implemented queues are:
* `basic_queue` - A `VecDeque` with a mutex lock.
* `bounded_ringbuffer` -A simple own implemented ringbuffer using a `Vec`.
* `lockfree_queue` - A lock-free queue from the crate `lockfree`.
* `concurrent_queue` - A queue from the crate `concurrent-queue`.
* `array_queue` - A queue from the crate `crossbeam`.
### Optional extra feature:
* `memory_tracking` - Writes to a file the memory allocated by the program
during the execution. Requires `jemalloc`.

## Flags
* To use specific values you can add different flags to the run command:
    * `-t`, `--time-limit` for specific time values.
    * `-p`, `--producers` for specified amount of producers.
    * `-c`, `--consumers` for specified amount of consumers.
    * `-o`, `--one-socket` to run on one socket (specific for our test environment).
    * `-i`, `--iterations` to specify how many iterations to run the benchmark.
    * `-e`, `--empty-pops` if you want to include empty dequeue operations.
    * `-h`, `--help` to print help.
    * `-V` `--version` to print the version of the benchmark.
    * `-d` `--delay-nanoseconds` to change the delay between operations.
    * `--path` to change where the output of the benchmark is put.


# TODO
* Fix different implementations for queues
* Add to config to be able to choose which queue to test
