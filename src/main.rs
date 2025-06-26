// src/main.rs
use anyhow::Result;
use clap::Parser;
use serde_json;

mod parser;
mod graph;
mod dot;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about = "Generate dependency graphs from a GNU make database")]
struct Args {
    /// Print targets graph
    #[arg(long, conflicts_with = "variables")]
    targets: bool,

    /// Print variables graph
    #[arg(long, conflicts_with = "targets")]
    variables: bool,

    /// Maximum threads to render
    #[arg(long, default_value_t = 3)]
    maxthreads: usize,

    /// Rewrite patterns
    #[arg(long)]
    rewrite: Vec<String>,

    /// Suppress drawing of nodes matching pattern
    #[arg(long)]
    nodraw: Vec<String>,

    /// Also output PNG (requires 'dot' on PATH)
    #[arg(long)]
    png: bool,

    /// Enable debug tracing / JSON output
    #[arg(long)]
    debug: bool,

    /// Path to gnumake.db (use '-' for stdin)
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
        // Print structured JSON of entire MakeData
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
        dot::write_dot(&format!("{}.targets.dot", data.goal), &dot_targets)?;
        if args.png {
            dot::render_png(&format!("{}.targets.dot", data.goal))?;
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
        dot::write_dot(&format!("{}.variables.dot", data.goal), &dot_vars)?;
        if args.png {
            dot::render_png(&format!("{}.variables.dot", data.goal))?;
        }
    }
    Ok(())
}
