use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
// use num_bigint::BigInt; // TODO: Revisit this once u64 things are stable.
// use std::str::FromStr;
use fst::Map;
use std::io::Write;
// use fst::Streamer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();

    let evaluations_path = format!("{}/fst/evaluations.fst", project_root);
    let nullaries_tmp_txt_path = format!("{}/candidates/nullaries.txt", project_root);
    let new_nullaries_tmp_txt_path = format!("{}/candidates/new_nullaries.txt", project_root);

    let file = File::open(nullaries_tmp_txt_path)?;
    let reader = BufReader::new(file);
    let candidate_nullaries: HashSet<String> = reader.lines().collect::<Result<HashSet<_>, _>>()?;

    let nullaries = Map::new(std::fs::read(evaluations_path).unwrap()).unwrap();

    let mut new_nullaries_file = File::create(new_nullaries_tmp_txt_path)?;

    for candidate in candidate_nullaries {
        if !nullaries.contains_key(&candidate) {
            write!(new_nullaries_file, "{}\n", candidate)?;
        }
    }

    return Ok(());
}
