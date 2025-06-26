// src/main.rs
use anyhow::Result;
use clap::Parser;

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

    /// Path to gnumake.db (use '-' for stdin)
    #[arg(value_name = "GNUMAKE_DB")]
    db_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = parser::parse_db(&args.db_path)?;

    if args.targets {
        let dot = dot::render_targets(&data, args.maxthreads, &args.nodraw);
        dot::write_dot(&format!("{}.targets.dot", data.goal), &dot)?;
        if args.png {
            dot::render_png(&format!("{}.targets.dot", data.goal))?;
        }
    }
    if args.variables {
        let dot = dot::render_variables(&data);
        dot::write_dot(&format!("{}.variables.dot", data.goal), &dot)?;
        if args.png {
            dot::render_png(&format!("{}.variables.dot", data.goal))?;
        }
    }
    Ok(())
}
