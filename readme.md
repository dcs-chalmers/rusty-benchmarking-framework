# Lock-free benchmarking tool
This is a project to test different implementations of lock-free based data structures to measure their output and performance.

## How to use:
To run the benchmark, 
* download the the repo
* Write in a terminal `cargo run` for standard values
* To use specific values you can add different flags to the run command:
    * `-t`, `--time-limit` for specific time values.
    * `-p`, `--producers` for specified amount of producers.
    * `-c`, `--consumers` for specified amount of consumers.
    * `-o`, `--one-socket` to run on one socket (specific for our test environment).
    * `-i`, `--iterations` to specify how many iterations to run the benchmark.
    * `-e`, `--empty-pops` if you want to include empty dequeue operations.
    * `-h`, `--human-readable` if you want the output to be human readable.
    * `--help` to print help.
    * `-V` `--version` to print the version of the benchmark.

So to run using cargo:
`cargo run -- -t *specified value* -p *specified amount* -c *specified amount*`

# TODO
* Fix different implementations for queues
* Add to config to be able to choose which queue to test
