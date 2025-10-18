use evaluations::utils::loaders::load_evaluations;

fn main() {
    let fst = load_evaluations();
    println!("Loaded Evaluations. Length {}", fst.len());
}
