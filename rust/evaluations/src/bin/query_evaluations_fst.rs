use std::env;
use std::process;

use evaluations::store::MapStore;

fn main() {
    // Get the string to check from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <string_to_check>", args[0]);
        eprintln!("Example: {} \"R(^(#),0,^(0))\"", args[0]);
        process::exit(1);
    }

    let project_root = env::var("PROJECT_ROOT").expect("PROJECT_ROOT is not set");
    let store = MapStore::open_evaluations(&project_root).expect("Failed to open evaluations");

    // Reads through both FST layers (recent, then base).
    match store.get(&args[1]) {
        Some(value) => println!("{}", value),
        None => println!(""),
    }
}
