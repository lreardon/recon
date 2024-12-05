// Imports the `File` type into this scope and the entire `std::io` module.
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use serde_json::Value;
// use num_bigint::BigInt; // TODO: Revisit this once u64 things are stable.
use std::str::FromStr;
use fst::Streamer;


// Imports the `SetBuilder` type from the `fst` module.
// use fst::SetBuilder;
use fst::{Map, MapBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {


	let project_root = env::var("PROJECT_ROOT").unwrap();

	let _nullaries_tmp_fst_path = format!("{}/fst/tmp/nullaries.fst", project_root);
	let evaluations_path = format!("{}/fst/evaluations.fst", project_root);
	let nullaries_tmp_txt_path = format!("{}/candidates/tmp/nullaries.txt", project_root);
	let file = File::open(nullaries_tmp_txt_path)?;
	let reader = BufReader::new(file);
	
	let lines: Vec<String> = reader
    .lines()          // Creates an iterator of Results containing lines
    .collect::<Result<_, _>>()?; 
	
	println!("lines: {:?}", lines);

	let evaluations = Map::new(std::fs::read(evaluations_path).unwrap()).unwrap();

	let mut stream = evaluations.keys();
	let mut keys:Vec<String> = Vec::new();
	while let Some(k) = stream.next() {
    let key_string = String::from_utf8_lossy(k).into_owned();
		keys.push(key_string);
	}

	println!("keys: {:?}", keys);

	for line in lines {
		println!("Does map contain '{}'? {}", line, evaluations.contains_key(&line));
	}

	return Ok(());
}