use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
// use num_bigint::BigInt; // TODO: Revisit this once u64 things are stable.
// use std::str::FromStr;
use fst::Set;
use std::io::Write;
// use fst::Streamer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();

    let unaries_path = format!("{}/fst/unaries.fst", project_root);
    let unaries_tmp_txt_path = format!("{}/candidates/unaries.txt", project_root);
    let new_unaries_tmp_txt_path = format!("{}/candidates/new_unaries.txt", project_root);

    let file = File::open(unaries_tmp_txt_path)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let evaluations = Set::new(std::fs::read(unaries_path).unwrap()).unwrap();

    let mut new_unaries_file = File::create(new_unaries_tmp_txt_path)?;

    for line in lines {
        // println!("Does map contain '{}'? {}", line, evaluations.contains_key(&line));
        if !evaluations.contains(&line) {
            write!(new_unaries_file, "{}\n", line)?;
        }
    }

    return Ok(());
}