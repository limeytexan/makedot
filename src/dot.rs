// src/dot.rs
use crate::parser::MakeData;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Render the target dependency graph to a DOT string, correcting direction and deduplicating edges.
pub fn render_targets(data: &MakeData, maxthreads: usize, nodraw: &[String]) -> String {
    let mut out = String::new();
    writeln!(&mut out, "digraph gnumake {{").unwrap();
    writeln!(&mut out, "    node[shape=rect;style=\"rounded,bold\"]; {{").unwrap();

    let mut colours = build_colour_map(data);
    let (threads, thread_vertices) = compute_threads(&data.tgt_deps, &data.goal);
    let mut seen_edges = BTreeSet::new();

    for (start, ends) in &threads {
        for (end, distmap) in ends {
            for (&distance, thread_list) in distmap.iter().rev() {
                let mut keep = Vec::new();
                let mut prune = Vec::new();
                for t in thread_list {
                    if contains_endpoint(t, &thread_vertices) {
                        keep.push(t.clone());
                    } else {
                        prune.push(t.clone());
                    }
                }
                let allowed = maxthreads.saturating_sub(keep.len());
                if prune.len() > allowed && allowed > 0 {
                    let summary = format!(
                        "[{} -> {} ({} hops)]\n{},{}...\n{} items]",
                        start, end, distance - 2,
                        prune[allowed - 1][1], prune[allowed][1], prune.len() - allowed + 1
                    );
                    colours.insert(summary.clone(), "color=blue".into());
                    prune.truncate(allowed - 1);
                    prune.push(vec![start.clone(), summary.clone(), end.clone()]);
                }
                for t in keep.into_iter().chain(prune.into_iter()) {
                    emit_thread(&mut out, &t, &mut colours, nodraw, &mut seen_edges);
                }
            }
        }
    }

    writeln!(&mut out, "    }}").unwrap();
    writeln!(&mut out, "}}
").unwrap();
    out
}

/// Render the variable dependency graph to a DOT string.
pub fn render_variables(data: &MakeData) -> String {
    let mut out = String::new();
    writeln!(&mut out, "digraph gnumake {{").unwrap();
    writeln!(&mut out, "    node[shape=rect;style=\"rounded,bold\"]; {{").unwrap();

    let mut seen = BTreeSet::new();
    print_vars(&mut out, &data.var_deps, &data.goal, &mut seen);

    writeln!(&mut out, "    }}").unwrap();
    writeln!(&mut out, "}}
").unwrap();
    out
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

// Helpers for targets graph

fn build_colour_map(data: &MakeData) -> BTreeMap<String,String> {
    let mut m = BTreeMap::new();
    m.insert(data.goal.clone(), "color=red".into());
    for tgt in data.tgt_deps.keys() {
        if m.contains_key(tgt) { continue; }
        let style = if data.tgt_deps.get(tgt).map_or(true, |v| v.is_empty()) {
            "color=green"
        } else {
            "color=orange"
        };
        m.insert(tgt.clone(), style.into());
    }
    m
}

fn compute_threads(
    graph: &std::collections::HashMap<String, Vec<String>>,
    start: &str,
) -> (
    BTreeMap<String, BTreeMap<String, BTreeMap<usize, Vec<Vec<String>>>>>,
    BTreeMap<String, usize>,
) {
    let mut threads = BTreeMap::new();
    let mut tv = BTreeMap::new();
    fn dfs(
        graph: &std::collections::HashMap<String, Vec<String>>,
        node: &String,
        stack: &mut Vec<String>,
        threads: &mut BTreeMap<String, BTreeMap<String, BTreeMap<usize, Vec<Vec<String>>>>>,
        tv: &mut BTreeMap<String, usize>,
    ) {
        let endpoint = graph.get(node).map_or(true, |d| d.is_empty());
        if endpoint {
            let mut s = stack.clone();
            s.push(node.clone());
            *tv.entry(node.clone()).or_default() += 1;
            let st = s.first().unwrap().clone();
            let en = node.clone();
            let dist = s.len();
            threads
                .entry(st)
                .or_default()
                .entry(en)
                .or_default()
                .entry(dist)
                .or_default()
                .push(s);
        } else {
            let mut s = stack.clone();
            s.push(node.clone());
            *tv.entry(node.clone()).or_default() += 1;
            for dep in &graph[node] {
                dfs(graph, dep, &mut s, threads, tv);
            }
        }
    }
    dfs(graph, &start.to_string(), &mut Vec::new(), &mut threads, &mut tv);
    (threads, tv)
}

fn contains_endpoint(thread: &[String], tv: &BTreeMap<String, usize>) -> bool {
    for v in thread.iter().skip(1).take(thread.len().saturating_sub(2)) {
        if tv.get(v).cloned().unwrap_or(0) > 1 {
            return true;
        }
    }
    false
}

/// Emit a thread, reversing edge direction and deduping via seen_edges.
fn emit_thread(
    out: &mut String,
    thread: &[String],
    colours: &mut BTreeMap<String,String>,
    nodraw: &[String],
    seen_edges: &mut BTreeSet<(String,String)>,
) {
    for node in thread {
        if nodraw.iter().any(|pat| node.contains(pat)) { return; }
        if let Some(col) = colours.remove(node) {
            writeln!(out, "    \"{}\" [ {} ];", node, col).unwrap();
        }
    }
    // print edges prereq -> target
    for win in thread.windows(2) {
        let src = win[1].clone();
        let dst = win[0].clone();
        if seen_edges.insert((src.clone(), dst.clone())) {
            writeln!(out, "    \"{}\" -> \"{}\";", src, dst).unwrap();
        }
    }
}

fn print_vars(
    out: &mut String,
    var_deps: &std::collections::HashMap<String, Vec<String>>,
    target: &str,
    seen: &mut BTreeSet<String>,
) {
    if let Some(vars) = var_deps.get(target) {
        for var in vars {
            let edge = format!("\"{}\" -> \"{}\";", var, target);
            if seen.insert(edge.clone()) {
                writeln!(out, "    {}", edge).unwrap();
                print_vars(out, var_deps, var, seen);
            }
        }
    }
}
