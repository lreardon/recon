use std::env;
use std::process;

use evaluations::store::Frontier;

// Print every member of a stratum (or chains) frontier, one per line.
// Usage: ls_stratum <nullaries|unaries> <length> [chains]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <nullaries|unaries> <length> [chains]", args[0]);
        process::exit(1);
    }
    let project_root = env::var("PROJECT_ROOT").expect("PROJECT_ROOT is not set");
    let base = if args.get(3).map(String::as_str) == Some("chains") {
        "chains"
    } else {
        "strata"
    };
    let length: u64 = args[2].parse().expect("length must be a number");
    let items = Frontier::load(&project_root, base, &args[1], length)
        .expect("failed to load frontier");
    for item in items {
        println!("{}", item);
    }
}
