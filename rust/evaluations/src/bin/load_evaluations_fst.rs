use std::env;
use std::process;
use evaluations::fst_utils::get_evaluations_fst;

fn main() {
    let fst = get_evaluations_fst();
    println!("Loaded Evaluations. Length {}", fst.len());
}