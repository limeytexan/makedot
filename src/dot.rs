use crate::parser::MakeData;
use petgraph::dot::{Dot, Config};
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Render the target dependency graph to a DOT string
pub fn render_targets(data: &MakeData, _maxthreads: usize, _nodraw: &[String]) -> String {
    let graph = crate::graph::build_target_graph(&data.tgt_deps);
    format!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))
}

/// Render the variable dependency graph to a DOT string
pub fn render_variables(data: &MakeData) -> String {
    let graph = crate::graph::build_var_graph(&data.var_deps);
    format!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]))
}

/// Write the DOT string to a file
pub fn write_dot(path: &str, dot: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(dot.as_bytes())
}

/// Invoke Graphviz `dot` to produce a PNG
pub fn render_png(dot_path: &str) -> std::io::Result<()> {
    let png_path = dot_path.replace(".dot", ".png");
    Command::new("dot")
        .arg("-Tpng")
        .arg(dot_path)
        .arg("-o")
        .arg(png_path)
        .status()?;
    Ok(())
}
