// src/graph.rs
#![allow(dead_code)]
use petgraph::Graph;
use petgraph::Directed;
use std::collections::HashMap;

pub fn build_target_graph(tgt_deps: &HashMap<String, Vec<String>>) -> Graph<String, (), Directed> {
    let mut graph = Graph::new();
    let mut indices = HashMap::new();
    for (tgt, deps) in tgt_deps {
        let ti = *indices.entry(tgt.clone()).or_insert_with(|| graph.add_node(tgt.clone()));
        for dep in deps {
            let di = *indices.entry(dep.clone()).or_insert_with(|| graph.add_node(dep.clone()));
            graph.add_edge(di, ti, ());
        }
    }
    graph
}

pub fn build_var_graph(var_deps: &HashMap<String, Vec<String>>) -> Graph<String, (), Directed> {
    let mut graph = Graph::new();
    let mut indices = HashMap::new();
    for (tgt, vars) in var_deps {
        let ti = *indices.entry(tgt.clone()).or_insert_with(|| graph.add_node(tgt.clone()));
        for var in vars {
            let vi = *indices.entry(var.clone()).or_insert_with(|| graph.add_node(var.clone()));
            graph.add_edge(vi, ti, ());
        }
    }
    graph
}
