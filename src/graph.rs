use std::fs;

pub fn create_adj_matrix(graph_file: String, node_amount: usize) -> Result<Vec<Vec<usize>>, std::io::Error> {
    let graph_contents = fs::read_to_string(graph_file)?;
    let edges: Vec<Vec<usize>> = graph_contents.lines()
        .filter(|line| !line.starts_with('%'))
        .map(|line| {
                line.split(" ")
                    .map(|n| n.parse::<usize>().expect("File populated with non-integers"))
                    .collect::<Vec<usize>>()
        }).collect();
    let mut adj_mat: Vec<Vec<usize>> = vec![Vec::new(); node_amount];
    for edge in edges.iter() {
        let src = edge[0];
        let dst = edge[1];
        adj_mat[src].push(dst); 
    }
    Ok(adj_mat)
}
