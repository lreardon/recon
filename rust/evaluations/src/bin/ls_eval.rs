use std::env;
use std::process;

use fst::Streamer;

use evaluations::utils::loaders::load_evaluations;

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

    let fst = load_evaluations();
    let mut stream = fst.stream();

    let mut count = 0;
    while count < n {
        match stream.next() {
            Some((key, value)) => {
                let key_str = std::str::from_utf8(key).unwrap();
                println!("{}\t{}", key_str, value);
                count += 1;
            }
            None => break,
        }
    }
}
