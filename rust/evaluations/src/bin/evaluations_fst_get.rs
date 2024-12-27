use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use fst::Map;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <nullary_expression>", args[0]);
        std::process::exit(1);
    }
    let nullary_expression = &args[1];
    
    let project_root = env::var("PROJECT_ROOT").unwrap();
    let evaluations_path = format!("{}/fst/evaluations.fst", project_root);
    let nullaries = Map::new(std::fs::read(evaluations_path).unwrap()).unwrap();
    println!("Size of nullaries map: {}", nullaries.len());
    
    if let Some(value) = nullaries.get(nullary_expression) {
        println!("{}", value);
    } else {
        println!("");
    }

    return Ok(());
}
