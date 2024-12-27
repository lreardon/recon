use std::env;
use std::fs::File;
use std::io::BufWriter;

use fst::MapBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();
    let evaluations_path = format!("{}/fst/evaluations.fst", project_root);
    let evaluations_file_handle = File::create(evaluations_path)?;
    let buffered_writer = BufWriter::new(evaluations_file_handle);
    let mut builder = MapBuilder::new(buffered_writer)?;
    builder.insert("1", 1)?;
    let _ = builder.finish();

    return Ok(());
}
