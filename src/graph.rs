use petgraph::graphmap::DiGraphMap;
use std::collections::HashMap;

/// Build the target dependency graph from parsed data
pub fn build_target_graph(tgt_deps: &HashMap<String, Vec<String>>) -> DiGraphMap<String, ()> {
    let mut graph = DiGraphMap::new();
    for (target, deps) in tgt_deps {
        graph.add_node(target.clone());
        for dep in deps {
            graph.add_node(dep.clone());
            graph.add_edge(dep.clone(), target.clone(), ());
        }
    }
    graph
}

/// Build the variable dependency graph from parsed data
pub fn build_var_graph(var_deps: &HashMap<String, Vec<String>>) -> DiGraphMap<String, ()> {
    let mut graph = DiGraphMap::new();
    for (tgt, vars) in var_deps {
        graph.add_node(tgt.clone());
        for var in vars {
            graph.add_node(var.clone());
            graph.add_edge(var.clone(), tgt.clone(), ());
        }
    }
    graph
}
