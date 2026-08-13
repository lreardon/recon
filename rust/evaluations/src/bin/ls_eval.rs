use std::env;
use std::process;

use evaluations::store::MapStore;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <n>", args[0]);
        eprintln!("Example: {} 10", args[0]);
        process::exit(1);
    }

    let n: usize = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: <n> must be a non-negative integer");
            process::exit(1);
        }
    };

    let project_root = env::var("PROJECT_ROOT").expect("PROJECT_ROOT is not set");
    let store = MapStore::open_evaluations(&project_root).expect("Failed to open evaluations");

    // Streams the union of both FST layers in key order.
    let mut count = 0;
    store
        .for_each_while(|key, value| {
            if count >= n {
                return false;
            }
            println!("{}\t{}", key, value);
            count += 1;
            true
        })
        .expect("Failed to stream evaluations");
}
