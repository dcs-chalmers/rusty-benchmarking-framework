# How to use scripts
There are several useful scripts in this folder. `plot.py` is to plot data from the benchmark.
The shell scripts are for running then benchmark.
## Plotting
You need `pandas` and `matplotlib`.
To use `plot.py`, simply run it in the following way:
```bash
python ./scripts/plot.py <path-to-data> <bench-type>
```
For example:
```bash
python ./scripts/plot.py ./scripts/example_data/thread_count thread_count
```
## Shell scripts
The shell scripts can be used to do many benchmarks after each other.
To run them (all of them are structured the same way), do:
```bash
./scripts/<script> <features-comma-separated> <start> <end> <increment> <path-for-output>
```
For example:
```bash
# This will benchmark all three queues with 2-256 producers 1 consumer
# and put the output in a folder "my_output".
./scripts/mpsc.sh array_queue,basic_queue,wfqueue 2 256 2 my_output
```

