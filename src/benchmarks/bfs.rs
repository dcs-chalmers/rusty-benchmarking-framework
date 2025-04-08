
use crate::{benchmarks::BenchConfig, traits::{ConcurrentQueue, Handle}, arguments::Benchmarks};
use std::sync::{atomic::{AtomicUsize, Ordering}, Barrier};
use log::{error, debug};
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
    let node_amount = bench_conf.get_node_amount().unwrap();
    debug!("Loading graph now.");
    let graph = create_graph(
        bench_conf.get_graph_filename().unwrap(),
        node_amount
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
    debug!("Starting parallell BFS now");
    for _ in 0..10 {

        let (dur_par, par_ret_vec) = parallell_bfs(&cqueue, &graph, node_amount, curr, thread_count);
        println!("Graph traversal done. Took {:?}.", dur_par);
    }
    // let seq_ret_vec = sequential_bfs();
    // for (i, node) in par_ret_vec.iter().enumerate() {
    //     if *node != seq_ret_vec[i] {
    //         error!("Parallell BFS solution arrived at wrong answer.");
    //         break;
    //     }
    // }
    Ok(())
}

fn parallell_bfs <C> (cqueue: &C, graph: &Vec<Vec<usize>>, node_amount: usize, start_node: usize, thread_count: usize) -> (std::time::Duration, Vec<usize>)
where
C: ConcurrentQueue<usize>,
    for<'a> &'a C: Send
{
    let result_vector: Vec<AtomicUsize> = (0..node_amount).map(|_| AtomicUsize::new(usize::MAX)).collect();
    result_vector[start_node].store(0, Ordering::Relaxed);
    let idle_count: AtomicUsize = AtomicUsize::new(0);
    let no_work_count: AtomicUsize = AtomicUsize::new(0);
    let barrier = Barrier::new(thread_count + 1);
    let scope_result = std::thread::scope(|s| -> Result<std::time::Duration, ()>{
        let cqueue = cqueue;
        let graph = graph;
        let idle_count = &idle_count;
        let no_work_count = &no_work_count;
        let barrier = &barrier;
        let result_vector = &result_vector;
        let mut handles = vec![];
        for _ in 0..thread_count {
            handles.push(s.spawn(move|| {
                let mut handle = cqueue.register();
                barrier.wait();
                loop {
                    match handle.pop() {
                        Some(node) => {
                            let distance = result_vector[node].load(Ordering::SeqCst);
                            for neighbour in &graph[node] {
                                let mut n_distance = result_vector[*neighbour].load(Ordering::SeqCst);
                                while distance + 1 < n_distance {
                                    if result_vector[*neighbour].compare_exchange_weak(
                                        n_distance,
                                        distance + 1,
                                        Ordering::SeqCst,
                                        Ordering::SeqCst)
                                        .is_ok() {
                                            match handle.push(*neighbour) {
                                                Err(e) => {
                                                    error!("Failed to push to queue: {e}");
                                                    continue;
                                                },
                                                Ok(_) => break,
                                            }
                                    }
                                    n_distance = result_vector[*neighbour].load(Ordering::SeqCst);
                                }
                            }
                        },
                        None => {
                            no_work_count.fetch_add(1, Ordering::Relaxed);
                            while let None = handle.pop() {
                                if no_work_count.load(Ordering::Relaxed) >= thread_count {
                                    if should_terminate(idle_count, no_work_count, thread_count) {
                                        return;
                                    }
                                }
                            }
                            no_work_count.fetch_sub(1, Ordering::Relaxed);
                        }
                    }
                }
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
    (duration, ret_vec)
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
    return false;
}

fn sequential_bfs() -> Vec<usize> {
    todo!()
}

pub fn create_graph(graph_file: String, node_amount: usize) -> Result<Vec<Vec<usize>>, std::io::Error> {
    let graph_contents = fs::read_to_string(graph_file)?;
    let edges: Vec<Vec<usize>> = graph_contents.lines()
        .filter(|line| !line.starts_with('%'))
        .map(|line| {
                line.split(" ")
                    .map(|n| n.parse::<usize>().expect("File populated with non-integers"))
                    .collect::<Vec<usize>>()
        }).collect();
    let mut graph: Vec<Vec<usize>> = vec![Vec::new(); node_amount];
    for edge in edges.iter() {
        let src = edge[0];
        let dst = edge[1];
        graph[src].push(dst); 
    }
    Ok(graph)
}