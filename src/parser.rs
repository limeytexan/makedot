// src/parser.rs
#![allow(dead_code)]
use anyhow::{Context, Result};
use pest_derive::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, fs::File, io::{BufRead, BufReader}};

#[derive(Parser)]
#[grammar = "mkdb.pest"]
struct MkdbParser;

#[derive(Debug, Serialize, Deserialize)]
pub struct MakeData {
    pub goal: String,
    pub tgt_deps: HashMap<String, Vec<String>>,
    pub var_deps: HashMap<String, Vec<String>>,
    pub phony_targets: HashSet<String>,
    pub intermediate_targets: HashSet<String>,
    #[allow(dead_code)]
    pub values: HashMap<String, (String, usize, String)>,
}

/// Scan a Makefile prerequisite string for target tokens by scanning the
/// string for targets (stop at `|`, drop the token before `=` and return).
fn scanstring_for_targets(input: &str) -> Vec<String> {
    let mut retval = Vec::new();
    let mut accum = String::new();
    let mut in_quotes = false;
    for ch in input.chars() {
        match ch {
            '|' => break,
            '=' => { retval.pop(); accum.clear(); break; },
            '"' | '\'' => in_quotes = !in_quotes,
            ' ' if !in_quotes => {
                if !accum.is_empty() {
                    retval.push(std::mem::take(&mut accum));
                }
            }
            c => accum.push(c),
        }
    }
    if !accum.is_empty() {
        retval.push(accum);
    }
    retval
}

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
    let mut phony_targets: HashSet<String> = HashSet::new();
    let mut intermediate_targets: HashSet<String> = HashSet::new();
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
        while line.ends_with('\\') {
            line.pop();
            buffer.clear();
            reader.read_line(&mut buffer)?;
            lineno += 1;
            line.push_str(buffer.trim_end());
        }
        if let Some(cap) = target_line_re.captures(&line) {
            let tgt = cap[1].to_string();
            let deps: Vec<String> = scanstring_for_targets(&cap[2]);
            match tgt.as_str() {
                ".PHONY" => {
                    for d in deps { phony_targets.insert(d); }
                }
                ".INTERMEDIATE" => {
                    for d in deps { intermediate_targets.insert(d); }
                }
                _ => {
                    tgt_deps.entry(tgt.clone()).or_default().extend(deps);
                }
            }
            buffer.clear();
            continue;
        }
        if let Some(cap) = assign_re.captures(&line) {
            let var = cap[1].to_string();
            let val = cap[3].to_string();
            match var.as_str() {
                "MAKECMDGOALS" => makecmdgoals = val.clone(),
                ".DEFAULT_GOAL" => default_goal = val.clone(),
                _ => {}
            }
            values.insert(var.clone(), (String::new(), lineno, val.clone()));
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
    Ok(MakeData { goal, tgt_deps, var_deps, phony_targets, intermediate_targets, values })
}
