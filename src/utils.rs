use petgraph::dot::{Config, Dot};
use petgraph::{Directed, Graph, Undirected};
use std::{
    io::Write,
    process::{Command, Stdio},
};

static mut UNIQUE_VALUE: usize = 0;

fn get_unique_value() -> usize {
    unsafe {
        let cur_val = UNIQUE_VALUE;
        UNIQUE_VALUE += 1;
        cur_val
    }
}

pub fn get_new_block() -> String {
    format!("_block{}", get_unique_value())
}

pub fn graph_to_svg<N, E, Ty, Ix>(filename: &str, graph: &Graph<N, E, Ty, Ix>)
where
    Ix: std::fmt::Debug + petgraph::adj::IndexType,
    E: std::fmt::Debug,
    N: std::fmt::Debug,
    Ty: petgraph::EdgeType,
{
    let contents = format!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));
    let mut child = Command::new("dot")
        .arg("-T")
        .arg("svg")
        .arg("-o")
        .arg(format!("{filename}.svg"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();

    let child_stdin = child.stdin.as_mut().unwrap();
    child_stdin.write_all(contents.as_bytes()).unwrap();
}

pub fn assoc_list_to_directed_graph<N>(list: Vec<(N, N)>) -> Graph<N, (), Directed, u32>
where
    N: std::fmt::Debug,
{
    let mut graph = Graph::default();
    for (src, dst) in list {
        let src = graph.add_node(src);
        let dst = graph.add_node(dst);
        graph.add_edge(src, dst, ());
    }
    graph
}

pub fn assoc_list_to_undirected_graph<N>(list: Vec<(N, N)>) -> Graph<N, (), Undirected, u32>
where
    N: std::fmt::Debug,
{
    let mut graph: Graph<N, (), Undirected> = Graph::default();
    for (src, dst) in list {
        let src = graph.add_node(src);
        let dst = graph.add_node(dst);
        graph.add_edge(src, dst, ());
    }
    graph
}

pub fn extract_first_part(name: &str) -> &str {
    name.split('.').next().unwrap()
}
