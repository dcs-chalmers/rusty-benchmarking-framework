
use crate::{benchmarks::BenchConfig, traits::{ConcurrentQueue, Handle}, arguments::Benchmarks};
use std::sync::{atomic::{AtomicUsize, Ordering}, Barrier};
use log::{debug, error, trace};
use std::fs;

/*
arr[]
dist_to_start_node 
*/
pub fn benchmark_bfs<C> (cqueue: C, bench_conf: &BenchConfig) -> Result<(), std::io::Error>
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{
    assert!(matches!(bench_conf.args.benchmark, Benchmarks::BFS(_)));
    debug!("Loading graph now.");
    let graph = create_graph(
        bench_conf.get_graph_filename().unwrap(),
    ).unwrap();
    let thread_count = bench_conf.get_thread_count().unwrap();
    let mut biggest = 0;
    let mut curr = 0;
    for (i, edge) in graph.iter().enumerate() {
        if  edge.len() > biggest {
            biggest = edge.len();
            curr = i;
        }
    }
    debug!("Start node is: {curr}");
    println!("Starting parallell BFS now");
    let (dur_par, par_ret_vec) = parallell_bfs(&cqueue, &graph, curr, thread_count);
    println!("Graph traversal done. Took {:?}.", dur_par);
    debug!("Starting sequential BFS now");
    println!("Checking solution...");
    let seq_ret_vec = sequential_bfs(&cqueue, &graph, curr);
    for (i, node) in par_ret_vec.iter().enumerate() {
        debug!("Pos: {} Parallell: {} Sequential: {}", i, *node, seq_ret_vec[i]);
        if *node != seq_ret_vec[i] {
            error!("Parallell BFS solution arrived at wrong answer.");
            return Ok(());
        }
    }
    println!("Solution looks good.");
    Ok(())
}

fn parallell_bfs <C> (
    cqueue: &C,
    graph: &[Vec<usize>],
    start_node: usize,
    thread_count: usize)
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
    let _ = cqueue.register().push(start_node);

    let scope_result = std::thread::scope(|s| -> Result<std::time::Duration, ()>{
        let idle_count = &idle_count;
        let no_work_count = &no_work_count;
        let barrier = &barrier;
        let result_vector = &result_vector;
        let mut handles = vec![];
        for i in 0..thread_count {
            handles.push(s.spawn(move|| {
                let handle = cqueue.register();
                barrier.wait();
                pbfs_helper(
                    handle,
                    result_vector,
                    graph,
                    i,
                    no_work_count,
                    idle_count,
                    thread_count);
            }));
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
    debug!("Parallell sol: {:?}", ret_vec);
    (duration, ret_vec)
}

fn pbfs_helper(
    mut handle: impl Handle<usize>,
    result_vector: &Vec<AtomicUsize>,
    graph: &[Vec<usize>],
    i: usize,
    no_work_count: &AtomicUsize,
    idle_count: &AtomicUsize,
    thread_count: usize) 
{
    loop {
        match handle.pop() {
            Some(node) => {
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
                while handle.is_empty() {
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

fn sequential_bfs<C>(cqueue: &C, graph: &[Vec<usize>], start_node: usize) -> Vec<usize> 
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
    debug!("Sequential sol: {:?}", result_vector);
    result_vector
}

pub fn create_graph(graph_file: String) -> Result<Vec<Vec<usize>>, std::io::Error> {
    let graph_contents = fs::read_to_string(graph_file)?;
    let edges: Vec<Vec<usize>> = graph_contents.lines()
        .filter(|line| !line.starts_with('%'))
        .map(|line| {
                line.split(" ")
                    .map(|n| n.parse::<usize>().expect("File populated with non-integers"))
                    .collect::<Vec<usize>>()
        }).collect();
    let size = edges[0][0] + 1; // in case graph is not zero-indexed.
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); size];
    for edge in edges.iter().skip(1) {
        let src = edge[0];
        let dst = edge[1];
        graph[src].push(dst); 
    }
    Ok(graph)
}
