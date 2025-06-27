// src/dot.rs
use crate::parser::MakeData;
use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// A node in the build graph, with arbitrary styling attributes.
pub struct RenderNode {
    pub id:    String,
    /// attributes like ("color","red") or ("style","filled")
    pub attrs: Vec<(String,String)>,
}

/// A directed edge in the build graph, with arbitrary styling attributes.
pub struct RenderEdge {
    pub from:  String,
    pub to:    String,
    /// attributes like ("style","dotted"), ("color","blue")
    pub attrs: Vec<(String,String)>,
}

/// Build a renderer-agnostic graph: a list of nodes and edges with rich attributes.
pub fn build_graph(
    data: &MakeData,
    _maxthreads: usize,
    nodraw: &[String],
) -> (Vec<RenderNode>, Vec<RenderEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // 1) Collect all nodes (targets) and assign per-node attrs
    for tgt in data.tgt_deps.keys() {
        if nodraw.iter().any(|pat| tgt.contains(pat)) {
            continue;
        }
        let mut attrs = Vec::new();
        // color map, phony fill, goal highlight, etc.
        if tgt == &data.goal {
            attrs.push(("color".into(), "red".into()));
            if data.phony_targets.contains(&data.goal) {
                attrs.push(("style".into(), "rounded,filled".into()));
            } else {
                attrs.push(("style".into(), "bold".into()));
            }
        } else if data.phony_targets.contains(tgt) {
            attrs.push(("style".into(), "dashed".into()));
        }
        nodes.push(RenderNode { id: tgt.clone(), attrs });
    }

    // 2) Collect edges with per-edge attrs (threading, dotted, etc.)
    for (tgt, deps) in &data.tgt_deps {
        for dep in deps {
            if nodraw.iter().any(|pat| dep.contains(pat) || tgt.contains(pat)) {
                continue;
            }
            // no thread-count styling (data.threads no longer exists)
            let attrs = Vec::new();
            edges.push(RenderEdge {
                from: dep.clone(),
                to:   tgt.clone(),
                attrs,
            });
        }
    }

    (nodes, edges)
}

/// Render a Graphviz DOT graph from our generic nodes & edges.
pub fn render_dot(
    nodes: &[RenderNode],
    edges: &[RenderEdge],
) -> String {
    let mut out = String::new();
    out.push_str("digraph G {\n");
    // emit each node
    for node in nodes {
        let mut attr_str = String::new();
        for (k,v) in &node.attrs {
            write!(&mut attr_str, "{}={},", k, v).unwrap();
        }
        writeln!(out, "    \"{}\" [{}];", node.id, attr_str.trim_end_matches(',')).unwrap();
    }
    // emit each edge
    for edge in edges {
        let mut attr_str = String::new();
        for (k,v) in &edge.attrs {
            write!(&mut attr_str, "{}={},", k, v).unwrap();
        }
        writeln!(
            out,
            "    \"{}\" -> \"{}\" [{}];",
            edge.from,
            edge.to,
            attr_str.trim_end_matches(',')
        )
        .unwrap();
    }
    out.push_str("}\n");
    out
}

/// Render a Mermaid flowchart from our generic nodes & edges.
pub fn render_mmd(
    nodes: &[RenderNode],
    edges: &[RenderEdge],
) -> String {
    // helper to sanitize IDs
    fn sanitize(s: &str) -> String {
        s.chars()
         .map(|c| if c.is_alphanumeric() { c } else { '_' })
         .collect()
    }

    let mut out = String::new();
    out.push_str("```mermaid\nflowchart LR\n");
    // Emit edges
    let mut seen = HashSet::new();
    for edge in edges {
        let f = sanitize(&edge.from);
        let t = sanitize(&edge.to);
        if seen.insert((f.clone(), t.clone())) {
            // gather any Mermaid attrs (e.g. dotted)
            let mut arrow = "-->";
            for (k,v) in &edge.attrs {
                if k == "style" && v == "dotted" {
                    arrow = "-.->";
                }
            }
            writeln!(out, "    {} {} {};", f, arrow, t).unwrap();
        }
    }
    // Emit node styles (Mermaid only supports `style id fill:…,...`)
    for node in nodes {
        let id = sanitize(&node.id);
        let mut fills = Vec::new();
        let mut border = None;
        for (k,v) in &node.attrs {
            if k == "style" && v.contains("filled") {
                // assume first attr is fill color if present
                fills.push("lightgrey".to_string());
            }
            if k == "color" {
                border = Some(v.clone());
            }
        }
        if !fills.is_empty() || border.is_some() {
            let mut params = Vec::new();
            if let Some(col) = border { params.push(format!("stroke:{}", col)); }
            if !fills.is_empty() { params.push(format!("fill:{}", fills[0])); }
            // unwrap to avoid `?` in a function returning String
            writeln!(out, "    style {} {};", id, params.join(",")).unwrap();
        }
    }
    out.push_str("```");
    out
}

/// Render the "variables" graph (shallow recursion).
pub fn render_variables(data: &MakeData) -> String {
    let mut out = String::new();
    writeln!(&mut out, "digraph gnumake {{").unwrap();
    writeln!(&mut out, "    node[shape=rect;style=\"rounded,bold\"]; {{").unwrap();

    let mut seen = BTreeSet::new();
    print_vars(&mut out, &data.var_deps, &data.goal, &mut seen);

    writeln!(&mut out, "    }}").unwrap();
    writeln!(
        &mut out,
        "}}\n"
    )
    .unwrap();
    out
}

/// Write the DOT string to a file
pub fn write_dot(path: &str, dot: &str) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(dot.as_bytes())
}

/// Invoke `dot -Tpng` to render a PNG
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

fn print_vars(
    out: &mut String,
    var_deps: &std::collections::HashMap<String, Vec<String>>,
    target: &String,
    seen: &mut BTreeSet<String>,
) {
    if let Some(vars) = var_deps.get(target) {
        for var in vars {
            let edge = format!("\"{}\" -> \"{}\";", var, target);
            if seen.insert(edge.clone()) {
                writeln!(out, "\t{}", edge).unwrap();
                print_vars(out, var_deps, var, seen);
            }
        }
    }
}

