use crate::{benchmarks::BenchConfig, traits::{ConcurrentQueue, Handle}, arguments::Benchmarks};
use std::{fs::OpenOptions, sync::{atomic::{AtomicUsize, Ordering}, Barrier}};
use core_affinity::CoreId;
use log::{debug, error, info, trace};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};


/// Generates the graph, generates the sequential solution and gets which
/// node to start at.
pub fn pre_bfs_work<C>(cqueue: C, bench_conf: &BenchConfig)
    -> (Vec<Vec<usize>>, Vec<usize>, usize)
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{
    let args = match &bench_conf.args.benchmark {
        crate::arguments::Benchmarks::BFS(a) => a,
        _ => panic!(),
    };
    info!("Loading graph now...");
    let graph = create_graph(
        args.graph_file.clone(),
    ).unwrap();
    // Find start node. Currently finds node with most neighbours.
    let mut biggest = 0;
    let mut curr = 0;
    for (i, edge) in graph.iter().enumerate() {
        if  edge.len() > biggest {
            biggest = edge.len();
            curr = i;
        }
    }
    info!("Generating correct solution...");
    debug!("Start node is: {curr}");
    
    let seq_ret_vec = if !args.no_verify {
        sequential_bfs(cqueue, &graph, curr)
    } else {
        vec![]
    };
    (graph, seq_ret_vec, curr)
}

/*
arr[]
dist_to_start_node 
*/

/// Explanation:
/// A benchmark to test how fast your data structure can complete a Breadth-First Search (BFS)
/// and if it does so correctly.
/// Need to send in your data structure and the graph you want to do bfs on (only .mtx files allowed).
/// Benchmark specififc flags:
/// * `--graph-file`                      The .mtx graph file you want to use in your bfs.
/// * `--thread-count`        (OPTIONAL)  The amount of threads you want to have in your benchmark (if left out, standard 20).
/// * `--no-verify`           (OPTIONAL)  Boolean to opt out of verifying the parallel benchmark results against the sequential (standard false).
/// 
///     Ex. run:
///     cargo run -features data_structure,bfs -- bfs --graph-file graph.mtx 
pub fn benchmark_bfs<C> (
    cqueue: C,
    graph: &[Vec<usize>],
    bench_conf: &BenchConfig,
    seq_ret_vec: &[usize],
    start_node: usize,
    ) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{
    assert!(matches!(bench_conf.args.benchmark, Benchmarks::BFS(_)));
    let args = match &bench_conf.args.benchmark {
        crate::arguments::Benchmarks::BFS(a) => a,
        _ => panic!(),
    };
    let thread_count = args.thread_count;
    debug!("Starting parallell BFS now");
    let (dur_par, par_ret_vec) = parallell_bfs(&cqueue, graph, start_node, thread_count, bench_conf);
    debug!("Graph traversal done. Took {:?}.", dur_par);

    if !args.no_verify {
        debug!("Comparing results to the sequential solution");
        for (i, node) in par_ret_vec.iter().enumerate() {
            trace!("Pos: {} Parallell: {} Sequential: {}", i, *node, seq_ret_vec[i]);
            if *node != seq_ret_vec[i] {
                error!("Parallell BFS solution arrived at wrong answer.");
                return Ok(());
            }
        }
        debug!("Solution looks good.");
    }
    let formatted = format!("{},{},{},{}",
            dur_par.as_millis(),
            cqueue.get_id(),
            args.thread_count,
            bench_conf.benchmark_id);
    if !bench_conf.args.write_to_stdout {
        let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&bench_conf.output_filename)?;

        writeln!(file, "{}", formatted)?;

    } else {
        println!("{}", formatted);
    }  

    Ok(())
}

fn parallell_bfs <C> (
    cqueue: &C,
    graph: &[Vec<usize>],
    start_node: usize,
    thread_count: usize,
    bench_conf: &BenchConfig)
-> (std::time::Duration, Vec<usize>)
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{

    let result_vector: Vec<AtomicUsize> = 
        (0..graph.len()).map(|_| AtomicUsize::new(usize::MAX)).collect();

    // Set distance of first node
    result_vector[start_node].store(0, Ordering::Relaxed);

    let idle_count: AtomicUsize = AtomicUsize::new(0);
    let no_work_count: AtomicUsize = AtomicUsize::new(0);
    let barrier = Barrier::new(thread_count + 1);
    // Add start node to queue
    let _ = cqueue.register().push(start_node);

    // Get cores for fairness of threads
    let available_cores: Vec<CoreId> =
        core_affinity::get_core_ids().unwrap_or(vec![CoreId { id: 0 }]);
    let mut core_iter = available_cores.into_iter().cycle();

    let scope_result = std::thread::scope(|s| -> Result<std::time::Duration, ()>{
        let idle_count = &idle_count;
        let no_work_count = &no_work_count;
        let barrier = &barrier;
        let result_vector = &result_vector;
        let mut handles = vec![];
        let is_one_socket = &bench_conf.args.one_socket;
        for i in 0..thread_count {
            handles.push({
                let mut core : CoreId = core_iter.next().unwrap();
                // if is_one_socket is true, make all thread ids even 
                // (this was used for our testing enviroment to get one socket)
                if *is_one_socket {
                    core = core_iter.next().unwrap();
                }
                s.spawn(move|| {
                    core_affinity::set_for_current(core);
                    // Register queue
                    let handle = cqueue.register();
                    // Wait for other queues
                    barrier.wait();
                    // Start BFS
                    pbfs_helper(
                        handle,
                        result_vector,
                        graph,
                        i,
                        no_work_count,
                        idle_count,
                        thread_count
                    );
                })
            });
        }
        barrier.wait();
        let start = std::time::Instant::now();
        for handle in handles {
            handle.join().unwrap();
        }
        let duration = start.elapsed();
        Ok(duration)
    });
    let duration = scope_result.expect("Should never return error");
    let ret_vec: Vec<usize> = result_vector
        .iter()
        .map(|val| val.load(Ordering::Relaxed))
        .collect();
    trace!("Parallell sol: {:?}", ret_vec);
    (duration, ret_vec)
}

fn pbfs_helper(
    mut handle: impl Handle<usize>,
    result_vector: &[AtomicUsize],
    graph: &[Vec<usize>],
    i: usize,
    no_work_count: &AtomicUsize,
    idle_count: &AtomicUsize,
    thread_count: usize) 
{
    let mut next = None;
    loop {
        if next.is_none() {
            next = handle.pop();
        }
        match next {
            Some(node) => {
                next = None;
                trace!("Thread: {i}; Acquired node {node}");
                let distance = result_vector[node].load(Ordering::SeqCst);
                for neighbour in &graph[node] {
                    let mut n_distance = result_vector[*neighbour].load(Ordering::SeqCst);
                    while distance + 1 < n_distance {
                        if result_vector[*neighbour].compare_exchange_weak(
                            n_distance,
                            distance + 1,
                            Ordering::SeqCst,
                            Ordering::SeqCst)
                            .is_ok() 
                        {
                            match handle.push(*neighbour) {
                                Err(e) => {
                                    error!("Failed to push to queue: {e}");
                                    continue;
                                },
                                Ok(_) => {
                                    trace!("Thread: {i}; Pushed {}", *neighbour);
                                    break;
                                },
                            }
                        }
                        n_distance = result_vector[*neighbour].load(Ordering::SeqCst);
                    }
                }
            },
            None => {
                trace!("Thread: {i}; Did not acquire any work");
                no_work_count.fetch_add(1, Ordering::Relaxed);
                loop {
                    next = handle.pop();
                    if next.is_some() { break; }
                    if no_work_count.load(Ordering::Relaxed) >= thread_count
                        && should_terminate(idle_count, no_work_count, thread_count)
                    {
                        return;
                    }
                }
                no_work_count.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

fn should_terminate(idle_count: &AtomicUsize, no_work_count: &AtomicUsize, thread_count: usize) -> bool {
    idle_count.fetch_add(1, Ordering::Relaxed);
    while no_work_count.load(Ordering::Relaxed) >= thread_count {
        if idle_count.load(Ordering::Relaxed) >= thread_count {
            return true;
        }
        //PAUSE? no-op
        std::hint::spin_loop();
    }
    idle_count.fetch_sub(1, Ordering::Relaxed);
    false
}

fn sequential_bfs<C>(cqueue: C, graph: &[Vec<usize>], start_node: usize) -> Vec<usize> 
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{
    let mut result_vector: Vec<usize> = (0..graph.len()).map(|_| usize::MAX).collect();
    let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut q = cqueue.register();
    result_vector[start_node] = 0;
    if q.push(start_node).is_err() {
        error!("Failed to start BFS, couldn't push start node");
    }
    while let Some(node) = q.pop() {
        let distance = result_vector[node];
        for n in &graph[node] {
            if visited.contains(n) {
                continue;
            }
            let n_distance = result_vector[*n];
            if n_distance > distance + 1 {
                result_vector[*n] = distance + 1;
            }
            if q.push(*n).is_err() {
                error!("Failed to push in sequential BFS");
            } 
            visited.insert(*n);
        }
    }
    trace!("Sequential sol: {:?}", result_vector);
    result_vector
}

pub fn create_graph(graph_file: String) -> Result<Vec<Vec<usize>>, std::io::Error> {
    let file = File::open(graph_file)?;
    let reader = BufReader::new(file);
    
    let mut edges = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.starts_with('%') {
            continue;
        }
        
        let edge: Vec<usize> = line.split(' ')
            .map(|n| n.parse::<usize>().expect("File populated with non-integers"))
            .collect();
        
        edges.push(edge);
    }
    
    let size = edges[0][0] + 1; // in case graph is not zero-indexed.
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); size];
    
    for edge in edges.iter().skip(1) {
        let src = edge[0];
        let dst = edge[1];
        graph[src].push(dst);
    }
    
    Ok(graph)
}
// Milliseconds,Queuetype,Thread Count,Test ID

