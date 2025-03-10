LPRQ Concurrent Queue Algorithm Evaluation
==========================================

**Publicly available at: [https://doi.org/10.5281/zenodo.7337237](https://doi.org/10.5281/zenodo.7337237)**

This guide describes how to run the experiments presented in the *"The State-of-the-Art LCRQ Concurrent Queue Algorithm Does NOT Require CAS2"* paper.

# Getting Started Guide
To reproduce the experiments, you will need an x86_64/amd64 machine capable of running Docker.

1. Install Docker, follow [the official instructions](https://docs.docker.com/engine/install/).

2. Build a Docker image:
   
    ```sh
    # run in the artifact root
    docker build -t queue-experiments .
    ```
    
    This will install all the requirements and build the experiments. Compilation may take several minutes.
    
3. Start the Docker container:
   
    ```sh
    docker run -it -v `pwd`/results:/experiments/results queue-experiments
    ```
    
    The `./results` folder in the working directory on the host machine is now linked to `/experiments/results` in the container. Please use it to transfer result files from the container to the host machine.
    
4. Run experiments in the container:
   
    ```sh
    ./run.sh
    ```
    
    The benchmark should take up to 40 minutes, depending on how many hardware threads the machine provides.
    
    > In case you do not want to wait so long, you can reduce the number of iterations for each run; it will slightly decline the measurement quality:
    >    ```sh
    >    # tune number of iterations for each run
    >    ./run.sh -i 5 # default is 10
    >    ```

5. The benchmarking has finished! See the generated plots in the `./results` directory on the host machine. The raw CSV files are also available there.


# Step-by-Step Instructions
Now we discuss how to vary benchmark parameters, enable and disable different internal mechanisms, and collect additional metrics.

### Benchmark parameters

- Change the number of iterations for each run; the more iterations, the lower deviation:

    ```sh
    ./run.sh -i 3 # default is 10
    ```

- Change the maximal number of threads:

    ```sh
    MAX_THREADS=256 ./run.sh
    ```

    > By default, the benchmark varies the number of threads by up to the number of CPUs.

- Change the amount of additional work performed after each enqueue/dequeue invocation:

    ```sh
    ./run.sh -w 4 # default is 8
    ```

    > We model a realistic workload by performing additional computational work between queue operations. The `-w` option controls the average number of uncontended loop cycles of such work; each loop cycle computes a pseudorandom number.

- Change the ring (segment) size:

    ```sh
    ./run.sh -r 4096 # default is 1024
    ```

    This affects the ring buffer (segment) size of all array-based queues: PRQ, CRQ, SCQ, and FAAArrayQueue.

- All the queue algorithms but cc-queue use hazard pointers as the memory reclamation scheme. You can disable hazard pointers with `DISABLE_HP=1` to validate that memory reclamation affects neither performance nor the resulting trends:

    ```sh
    DISABLE_HP=1 ./run.sh
    ```

### Additional metrics
It is possible to collect some additional metrics that support the claims in the paper. 
In particular, it is possible to analyze how often a new ring buffer is created.


To collect additional metrics, add `-m` parameter:

```sh
./run.sh -m
```

The following metrics are collected in the CSV file with the raw results:

- `appendNode` &mdash; number of segments appended to the queue per iteration
- `wasteNode` &mdash; number of nodes which were allocated but not appended to the list, because another thread did it
- `wasteToAppendNodeRatio` &mdash; `wasteNode/appendNode`
- `transfers` &mdash; number of transferred element per iteration
- `transfersPerNode` &mdash; average number of transfers per segment, equal to `transfers/appendNode`

### Custom plots

By default, `run.sh` produces charts similar to those in the paper. But you can also produce charts from custom data with the `postprocess/draw.py` script. For example, the following command will create `charts.pdf` file, containing two figures with the data from `cpp-res-enq-deq.csv` and `cpp-res-p1c1b.csv` files.

```sh
python3 postprocess/draw.py results/cpp-res-enq-deq.csv results/cpp-res-p1c1b.csv
```

If you collect additional metrics with `-m` flag, you can also draw charts of these metrics.

```sh
python3 -m transfersPerNode \
        postprocess/draw.py results/cpp-res-enq-deq.csv results/cpp-res-p1c1b.csv
```

> `postprocess/draw.py` accepts arbitrary number of CSV files as arguments. If no arguments are given, the script will use all CSVs in the current directory.

You can also compare the same queue in different setups using `postprocess/draw_compare.py` script. For example, to compare queues with and without memory reclamation run:

```sh
./run.sh
cp -r results results-with-hp
DISABLE_HP=1 ./run.sh
cp -r results results-without-hp
python3 postprocess/draw_compare.py \
        results-with-hp/cpp-res-enq-deq.csv results-without-hp/cpp-res-enq-deq.csv
```

The generated `comparison-charts.pdf` file will show lines for both queue implementions, with and without memory reclamation.

# Repository overview

- **Queue implementations**
  
    ```
    include/
        CCQueue.hpp           # [1]
        FAAArrayQueue.hpp     # [6]
        LCRQueue.hpp          # [4]
        LPRQueue.hpp          # [2] Listing 4
        LSCQueue.hpp          # [5]
        MichaelScottQueue.hpp # [3]
        ModLCRQueue.hpp       # [2] Appendix A, Listing 5
        FakeLCRQueue.hpp      # like LCRQ, but does not reuse segments
    third-party/
        ccqueue/              # original implementation of cc-queue [1]
        scqueue/              # original implementation of SCQ [5]
    ```
    
- **Benchmark code**
  
    ```
    include/
        BenchmarkQ.hpp      # source code of both benchmarks
    src/
        pairs-benchmark.cpp # enqueue-dequeue pairs benchmark entry point
        pc-benchmark.cpp    # producer-consumer benchmark entry point
    ```
    
- **Tests**
  
    ```
    src/test.cpp
    ```
    
- **Postprocessing**
  
    ```
    postprocess/
        draw.py         # draw custom charts
        draw_compare.py # draw comparison charts
        draw_main.py    # draw charts like in the paper
    ```

- **Helper scripts**
  
    ```
    run.sh                  # convenient way to run all benchmarks and draw charts
    benchmarks.sh           # benchmark setup like in the paper
    ring-size-benchmarks.sh # test how ring size affects throughput
    ```

We omit the description of CMake and Docker configuration files, as well as supplementary files containing utilities for queues and benchmarks.


# Manual Build

You can build the experiments outside of Docker image.

**Dependencies:**
- clang 13+ (with libstdc++ 11.1+ or libc++ 12+) or gcc 11+
- cmake 3.13+
- jemalloc
- pkgconfig

Gtest and CLI11 will be fetched automatically.

```sh
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
make -j8
```

Cmake command supports the following options:
- `-DUSE_LIBCPP=ON` &mdash; use `libc++` instead of `libstdc++`
- `-DCAUTIOUS_DEQUEUE=ON` &mdash; enable preliminary emptiness check in `Dequeue()` for LCRQ, LPRQ, LSCQ and FAAArrayQueue 
- `-DDISABLE_HP=ON` &mdash; disable memory reclamation for LCRQ, LPRQ, LSCQ, FAAArrayQueue and Michael-Scott Queue

After a successful compilation the build directory will contain three executables:
- `tests` &mdash; the unit tests for all queues
- `bench-prod-cons` &mdash; the producer-consumer benchmark
- `bench-enq-deq` &mdash; the enqueue-dequeue pairs benchmark

To see available options run:
```sh
./bench-prod-cons --help
./bench-enq-deq --help
```

# References

[1] Panagiota Fatourou and Nikolaos D. Kallimanis. 2012. Revisiting the Combining Synchronization Technique. SIGPLAN Not. 47, 8 (feb 2012), 257–266. https://doi.org/10.1145/2370036.2145849

[2] The State-of-the-Art LCRQ Concurrent Queue Algorithm Does NOT Require CAS2. To appear in Proceedings of PPoPP'23.

[3] Maged M Michael and Michael L Scott. 1998. Nonblocking algorithms and preemption-safe locking on multiprogrammed shared memory multiprocessors. J. Parallel and Distrib. Comput. 51, 1 (1998), 1–26.

[4] Adam Morrison and Yehuda Afek. 2013. Fast Concurrent Queues for X86 Processors. SIGPLAN Not. 48, 8 (feb 2013), 103–112. https://doi.org/10.1145/2517327.2442527

[5] Ruslan Nikolaev. 2019. A Scalable, Portable, and Memory-Efficient Lock-Free FIFO Queue. In 33rd International Symposium on Distributed Computing, DISC 2019, October 14-18, 2019, Budapest, Hungary (LIPIcs, Vol. 146), Jukka Suomela (Ed.). Schloss Dagstuhl - Leibniz-Zentrum für Informatik, 28:1–28:16. https://doi.org/10.4230/LIPIcs.DISC.2019.28

[6] Pedro Ramalhete. 2016. FAAArrayQueue - MPMC lock-free queue. http://concurrencyfreaks.blogspot.com/2016/11/faaarrayqueue-mpmc-lock-free-queue-part.html
