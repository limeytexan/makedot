use anyhow::{Context, Result};
use pest::Parser;
use std::{collections::HashMap, fs::File, io::Read};

#[derive(Parser)]
#[grammar = "mkdb.pest"]
struct MkdbParser;

/// Represents the parsed make database
pub struct MakeData {
    pub goal: String,
    pub tgt_deps: HashMap<String, Vec<String>>,
    pub var_deps: HashMap<String, Vec<String>>,
    pub values: HashMap<String, (String, usize, String)>, // var -> (file, line, val)
}

pub fn parse_db(path: &str) -> Result<MakeData> {
    let mut input = String::new();
    if path == "-" {
        std::io::stdin().read_to_string(&mut input)?;
    } else {
        File::open(path)
            .with_context(|| format!("opening make database at {}", path))?
            .read_to_string(&mut input)?;
    }
    // TODO: use pest to parse lines into structures
    unimplemented!("Parsing logic goes here");
}
