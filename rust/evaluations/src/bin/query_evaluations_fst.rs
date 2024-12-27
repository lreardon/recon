use std::env;
use std::process;
use evaluations::fst_utils::get_evaluations_fst;

fn main() {
    // Get the string to check from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <string_to_check>", args[0]);
        eprintln!("Example: {} \"R(^(#),1,^(1))\"", args[0]);
        process::exit(1);
    }

    let query = &args[1];
    let fst = get_evaluations_fst();
    
    // Note: get() returns an Option<u64> for Map
    match fst.get(query.as_bytes()) {
        Some(value) => println!("{}", value),
        None => println!(""),
    }
}