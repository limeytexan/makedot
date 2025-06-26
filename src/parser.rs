// src/parser.rs
#![allow(dead_code)]
#![allow(unused_imports)]
use anyhow::{Context, Result};
use pest_derive::Parser;
use regex::Regex;
use std::{collections::HashMap, fs::File, io::{BufRead, BufReader}};

#[derive(Parser)]
#[grammar = "mkdb.pest"]
struct MkdbParser;

/// Represents the parsed make database
pub struct MakeData {
    pub goal: String,
    pub tgt_deps: HashMap<String, Vec<String>>,
    pub var_deps: HashMap<String, Vec<String>>,
    #[allow(dead_code)]
    pub values: HashMap<String, (String, usize, String)>, // var -> (file, line, val)
}

/// Parse a GNU make "database" file into MakeData
pub fn parse_db(path: &str) -> Result<MakeData> {
    let mut reader: Box<dyn BufRead> = if path == "-" {
        Box::new(BufReader::new(std::io::stdin()))
    } else {
        let f = File::open(path)
            .with_context(|| format!("opening make database at {}", path))?;
        Box::new(BufReader::new(f))
    };

    let mut tgt_deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut var_deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut values: HashMap<String, (String, usize, String)> = HashMap::new();
    let mut default_goal = String::new();
    let mut makecmdgoals = String::new();

    let target_line_re = Regex::new(r"^(\S+):\s*(.*)$").unwrap();
    let assign_re = Regex::new(r"^(\S+)\s*([:?]?=)\s*(.+)$").unwrap();

    let mut buffer = String::new();
    let mut lineno = 0;
    while reader.read_line(&mut buffer)? > 0 {
        lineno += 1;
        let mut line = buffer.trim_end().to_string();
        // Handle continuation lines
        while line.ends_with('\\') {
            line.pop(); // remove '\'
            buffer.clear();
            reader.read_line(&mut buffer)?;
            lineno += 1;
            line.push_str(buffer.trim_end());
        }

        // Match target declarations
        if let Some(cap) = target_line_re.captures(&line) {
            let tgt = cap[1].to_string();
            let deps: Vec<String> = cap[2]
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            tgt_deps.entry(tgt.clone()).or_default().extend(deps);
            buffer.clear();
            continue;
        }

        // Match variable assignments
        if let Some(cap) = assign_re.captures(&line) {
            let var = cap[1].to_string();
            let _op = cap[2].to_string();
            let val = cap[3].to_string();
            // Capture special vars
            match var.as_str() {
                "MAKECMDGOALS" => makecmdgoals = val.clone(),
                ".DEFAULT_GOAL" => default_goal = val.clone(),
                _ => {}
            }
            values.insert(var.clone(), (String::new(), lineno, val.clone()));
            // Simple var references scanning
            for part in val.split_whitespace() {
                if let Some(stripped) = part.strip_prefix("$") {
                    let v = stripped.trim_matches(&['(', ')', '{', '}'][..]).to_string();
                    if v.len() > 1 {
                        var_deps.entry(var.clone()).or_default().push(v);
                    }
                }
            }
        }

        buffer.clear();
    }

    let goal = if !makecmdgoals.is_empty() {
        makecmdgoals.split_whitespace().next().unwrap().to_string()
    } else {
        default_goal
    };

    Ok(MakeData { goal, tgt_deps, var_deps, values })
}
