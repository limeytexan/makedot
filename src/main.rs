// src/main.rs
use anyhow::{Result, Context};
use clap::Parser;
use serde_json;
use std::env;
use std::fs;

mod parser;
mod graph;
mod dot;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about = "Generate dependency graphs from a GNU make database")]
struct Args {
    #[arg(long, conflicts_with = "variables")]
    targets: bool,
    #[arg(long, conflicts_with = "targets")]
    variables: bool,
    #[arg(long, default_value_t = 3)]
    maxthreads: usize,
    #[arg(long)]
    rewrite: Vec<String>,
    #[arg(long)]
    nodraw: Vec<String>,
    #[arg(long)]
    png: bool,
    #[arg(long)]
    debug: bool,
    #[arg(value_name = "GNUMAKE_DB")]
    db_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.debug {
        eprintln!("DEBUG: reading make DB from '{}'", args.db_path);
    }
    let data = parser::parse_db(&args.db_path)?;
    if args.debug {
        let json = serde_json::to_string_pretty(&data)?;
        println!("{}", json);
    }

    if args.targets {
        if args.debug {
            eprintln!("DEBUG: rendering targets graph (maxthreads={}, nodraw={:?})", args.maxthreads, args.nodraw);
        }
        let dot_targets = dot::render_targets(&data, args.maxthreads, &args.nodraw);
        if args.debug {
            println!("--- TARGET GRAPH DOT ---\n{}", dot_targets);
        }
        let tgt_path = format!("{}.targets.dot", data.goal);
        dot::write_dot(&tgt_path, &dot_targets)?;
        // Perform rewrites on the targets.dot file if requested
        if !args.rewrite.is_empty() {
            rewrite_file(&tgt_path, &data, &args.rewrite)?;
        }
        if args.png {
            dot::render_png(&tgt_path)?;
        }
    }

    if args.variables {
        if args.debug {
            eprintln!("DEBUG: rendering variables graph");
        }
        let dot_vars = dot::render_variables(&data);
        if args.debug {
            println!("--- VARIABLE GRAPH DOT ---\n{}", dot_vars);
        }
        let var_path = format!("{}.variables.dot", data.goal);
        dot::write_dot(&var_path, &dot_vars)?;
        // Perform rewrites on the variables.dot file if requested
        if !args.rewrite.is_empty() {
            rewrite_file(&var_path, &data, &args.rewrite)?;
        }
        if args.png {
            dot::render_png(&var_path)?;
        }
    }
    Ok(())
}

/// Rewrite a `.dot` file in place, applying substitutions from
/// environment vars and Makefile values.
fn rewrite_file(path: &str, data: &parser::MakeData, rewrites: &[String]) -> Result<()> {
    // Read original
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading file for rewrite: {}", path))?;

    // Apply do_rewrites to each line
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        out.push_str(&do_rewrites(line, data, rewrites));
        out.push('\n');
    }

    // Write to a temp and rename
    let tmp = format!("{}.new", path);
    fs::write(&tmp, out)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Perform string substitutions on one line:
/// 1) replace any env-var values with `$VARNAME`,
/// 2) then for each Makefile var whose name ends in the requested suffix,
///    replace its first literal value with `$(VARNAME)`.
fn do_rewrites(line: &str, data: &parser::MakeData, rewrites: &[String]) -> String {
    let mut result = line.to_string();
    for suffix in rewrites {
        // 1) environment variables
        if let Ok(val) = env::var(suffix) {
            result = result.replace(&val, &format!("${}", suffix));
        }
        // 2) Makefile-parsed values
        for (varname, (_file, _lineno, val)) in &data.values {
            if varname.ends_with(suffix) {
                if let Some(pos) = result.find(val) {
                    result = format!(
                        "{}$({}){}",
                        &result[..pos],
                        varname,
                        &result[pos + val.len()..]
                    );
                }
            }
        }
    }
    result
}
