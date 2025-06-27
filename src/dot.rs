// src/dot.rs
use crate::parser::MakeData;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::process::Command;

/// Render the "targets" graph.
pub fn render_targets(data: &MakeData, maxthreads: usize, nodraw: &[String]) -> String {
    let mut out = String::new();
    writeln!(&mut out, "digraph gnumake {{").unwrap();
    writeln!(&mut out, "    node[shape=rect;style=\"rounded,bold\"]; {{").unwrap();

    // 1) Build color/style map
    let mut colours = build_colour_map(data);

    // 2) Compute all threads & vertex counts
    let (threads, thread_vertices) = compute_threads(&data.tgt_deps, &data.goal);

    // 3) Emit each thread, pruning & deduplicating
    let mut seen_edges = HashSet::new();
    for (start, ends) in threads {
        for (end, dist_map) in ends {
            for (&dist, thread_list) in dist_map.iter().rev() {
                // split into keep vs pruneable
                let mut keep = Vec::new();
                let mut prune = Vec::new();
                for th in thread_list {
                    if contains_endpoint(&th, &thread_vertices) {
                        keep.push(th.clone());
                    } else {
                        prune.push(th.clone());
                    }
                }
                // collapse beyond maxthreads
                let allowed = maxthreads.saturating_sub(keep.len());
                if prune.len() > allowed && allowed > 0 {
                    let summary = format!(
                        "[{} -> {} ({} hops)\n{},{}...\n{} items]",
                        start,
                        end,
                        dist - 2,
                        prune[allowed - 1][1],
                        prune[allowed][1],
                        prune.len() - allowed + 1
                    );
                    colours.insert(summary.clone(), "color=blue".into());
                    prune.truncate(allowed - 1);
                    prune.push(vec![start.clone(), summary.clone(), end.clone()]);
                }
                for th in keep.into_iter().chain(prune.into_iter()) {
                    emit_thread(&mut out, &th, &mut colours, nodraw, &mut seen_edges);
                }
            }
        }
    }

    writeln!(&mut out, "    }}").unwrap();
    writeln!(
        &mut out,
        "}}
"
    )
    .unwrap();
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
        "}}
"
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

// Helpers for the targets graph

fn build_colour_map(data: &MakeData) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    // goal always red, but filling based on whether it's phony
    let goal_style = if data.phony_targets.contains(&data.goal) {
        "color=red,style=\"rounded,filled\""
    } else {
        "color=red"
    };
    m.insert(data.goal.clone(), goal_style.into());

    for tgt in data.tgt_deps.keys() {
        if m.contains_key(tgt) {
            continue;
        }
        let style = if data.phony_targets.contains(tgt) {
            // phony: filled
            "color=green,style=\"rounded,filled\""
        } else if data.intermediate_targets.contains(tgt) {
            // intermediate: dashed
            "color=orange,style=dashed"
        } else if data.tgt_deps.get(tgt).map_or(true, |v| v.is_empty()) {
            // leaf: green
            "color=green"
        } else {
            // internal: orange
            "color=orange"
        };
        m.insert(tgt.clone(), style.into());
    }

    m
}

fn compute_threads(
    graph: &std::collections::HashMap<String, Vec<String>>,
    root: &String,
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
        // endpoint = leaf only
        let is_endpoint = graph.get(node).map_or(true, |d| d.is_empty());
        // advance path & count
        let mut path = stack.clone();
        path.push(node.clone());
        *tv.entry(node.clone()).or_default() += 1;

        if is_endpoint {
            // record thread
            let start = path.first().unwrap().clone();
            let end = node.clone();
            let dist = path.len();
            threads
                .entry(start.clone())
                .or_default()
                .entry(end.clone())
                .or_default()
                .entry(dist)
                .or_default()
                .push(path);
        } else {
            for dep in &graph[node] {
                dfs(graph, dep, &mut path, threads, tv);
            }
        }
    }

    dfs(graph, root, &mut Vec::new(), &mut threads, &mut tv);
    (threads, tv)
}

/// Returns true if any inner vertex is shared among threads
fn contains_endpoint(thread: &[String], tv: &BTreeMap<String, usize>) -> bool {
    for v in thread.iter().skip(1).take(thread.len().saturating_sub(2)) {
        if tv.get(v).cloned().unwrap_or(0) > 1 {
            return true;
        }
    }
    false
}

/// Emit nodes+deduped edges: prereq -> target orientation
fn emit_thread(
    out: &mut String,
    thread: &[String],
    colours: &mut BTreeMap<String, String>,
    nodraw: &[String],
    seen_edges: &mut HashSet<(String, String)>,
) {
    for node in thread {
        if nodraw.iter().any(|pat| node.contains(pat)) {
            return;
        }
        if let Some(col) = colours.remove(node) {
            writeln!(out, "	\"{}\" [ {} ];", node, col).unwrap();
        }
    }
    for win in thread.windows(2) {
        let prereq = win[1].clone();
        let target = win[0].clone();
        if seen_edges.insert((prereq.clone(), target.clone())) {
            writeln!(out, "	\"{}\" -> \"{}\";", prereq, target).unwrap();
        }
    }
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
                writeln!(out, "	{}", edge).unwrap();
                print_vars(out, var_deps, var, seen);
            }
        }
    }
}

/// Sanitize a node name into a Mermaid-safe identifier (alphanumeric + `_`).
fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Render a Mermaid (flowchart LR) diagram of the targets graph.
pub fn render_mermaid_targets(
    data: &MakeData,
    _maxthreads: usize,
    nodraw: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("```mermaid\nflowchart LR\n");

    // For each dependency edge: prereq --> target
    for (tgt, deps) in &data.tgt_deps {
        for dep in deps {
            if nodraw.iter().any(|pat| dep.contains(pat) || tgt.contains(pat)) {
                continue;
            }
            let from = sanitize_id(dep);
            let to = sanitize_id(tgt);
            writeln!(out, "    {} --> {};", from, to).unwrap();
        }
    }

    // Highlight the final goal node
    let goal_id = sanitize_id(&data.goal);
    if data.phony_targets.contains(&data.goal) {
        // match the Perl style for phony: filled node
        writeln!(
            out,
            "    style {} fill:#f96,stroke:#333,stroke-width:2px;",
            goal_id
        )
        .unwrap();
    } else {
        // non-phony: red border only
        writeln!(
            out,
            "    style {} stroke:#f00,stroke-width:2px;",
            goal_id
        )
        .unwrap();
    }

    out.push_str("```");
    out
}
