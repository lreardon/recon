// Imports the `File` type into this scope and the entire `std::io` module.
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use serde_json::Value;
// use num_bigint::BigInt; // TODO: Revisit this once u64 things are stable.
use std::str::FromStr;


// Imports the `SetBuilder` type from the `fst` module.
// use fst::SetBuilder;
use fst::{Map, MapBuilder};
// use fst::Streamer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// -- initializing evaluations.fst and tmp.fst --
	// create_initial_map(&evaluations_fst_path)?;
	// create_tmp_map(&tmp_fst_path)?;
	// ----

	let project_root = env::var("PROJECT_ROOT").unwrap();
	let _nullaries_tmp_fst_path = format!("{}/fst/tmp/nullaries.fst", project_root);

	let nullaries_tmp_json_path = format!("{}/candidates/tmp/nullaries.json", project_root);
	let file = File::open(nullaries_tmp_json_path)?;
	let reader = BufReader::new(file);

	let nullaries: serde_json::Value = serde_json::from_reader(reader)?;
	let mut map: HashMap<String, u64> = HashMap::new();

	if let Value::Object(obj) = nullaries {
		for (key, value) in obj {
			if let Value::String(value) = value {
				match u64::from_str(&value) {
					Ok(int) => {
						map.insert(key, int);
					},
					Err(e) => eprintln!("Failed to parse string {}: {}", value, e),
				}
			}
		};
	};

	let tmp_path = format!("{}/fst/tmp/nullaries.fst", project_root);
	let mut wtr = io::BufWriter::new(File::create(&tmp_path)?);
	let mut build = MapBuilder::new(wtr)?;
	for (key, value) in map {
		build.insert(key, value).unwrap();
	}

	build.finish()?;

	let map = Map::new(std::fs::read(&tmp_path).unwrap()).unwrap();
	
	println!();
	println!("Does map contain '1'? {}", map.contains_key("1"));
	println!("Does map contain '^(1)'? {}", map.contains_key("^(1)"));
	println!();
	println!("Map value of '1' is {}", map.get("1").unwrap());

	return Ok(());
}