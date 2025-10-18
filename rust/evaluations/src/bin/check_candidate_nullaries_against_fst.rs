use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
// use num_bigint::BigInt; // TODO: Revisit this once u64 things are stable.
// use std::str::FromStr;
use std::io::Write;

mod utils;
use std::sync::Arc;
use utils::fst_inclusion::{fst_contains_nullary, NullaryAndFst};
use utils::loaders::{load_file, load_fst_map, LocalPath};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root: String = env::var("PROJECT_ROOT").unwrap();

    let file = load_file(LocalPath {
        path: "candidates/nullaries.txt".to_string(),
    })?;
    let reader = BufReader::new(file);
    let candidate_nullaries: HashSet<String> = reader.lines().collect::<Result<HashSet<_>, _>>()?;

    let nullaries = load_fst_map(LocalPath {
        path: "evaluations".to_string(),
    })
    .unwrap();

    let new_nullaries_tmp_txt_path = format!("{}/candidates/new_nullaries.txt", project_root);
    let mut new_nullaries_file = File::create(new_nullaries_tmp_txt_path)?;

    for candidate in candidate_nullaries {
        if fst_contains_nullary(NullaryAndFst {
            nullary: &candidate,
            fst: Arc::clone(&nullaries),
        }) {
            write!(new_nullaries_file, "{}\n", candidate)?;
        }
    }

    return Ok(());
}
