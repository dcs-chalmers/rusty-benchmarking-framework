# Lock-free benchmarking tool
This is a project to test different implementations of lock-free based data structures to measure their output and performance.

## How to use:
To run the benchmark, 
* download the the repo
* Write in a terminal `cargo run` for standard values
* To use specific values you can add different flags to the run command:
    * `-t` for specific time values
    * `-p` for specified amount of producers
    * `-c` for specified amount of consumers

So for all flags active, write:
`cargo run -- -t *specified value* -p *specified amount* -c *specified amount*`

# TODO
* Fix different implementations for queues
* Add to config to be able to choose which queue to test