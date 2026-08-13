use std::collections::HashMap;
use std::env;
use std::process;

use evaluations::evaluator::{EvalLookup, Evaluator};
use evaluations::store::MapStore;

struct StoreLookup(MapStore);

impl EvalLookup for StoreLookup {
    fn lookup(&self, key: &str) -> Option<u64> {
        self.0.get(key)
    }
}

pub fn evaluate_recursive_form(form: &str) -> Result<u64, String> {
    let project_root = env::var("PROJECT_ROOT").map_err(|_| "PROJECT_ROOT is not set")?;
    let lookup = StoreLookup(MapStore::open_evaluations(&project_root)?);
    let mut memo = HashMap::new();
    Evaluator::new(&lookup, &mut memo)
        .evaluate_nullary(form)
        .map_err(|error| error.to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <recursive_form>", args[0]);
        eprintln!("Example: {} \"R(^(#),0,^(0))\"", args[0]);
        process::exit(1);
    }

    match evaluate_recursive_form(&args[1]) {
        Ok(value) => println!("{}", value),
        Err(error) => {
            eprintln!("Error: {}", error);
            process::exit(1);
        }
    }
}
