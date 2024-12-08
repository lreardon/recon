use std::env;
use std::fs::File;
use std::io::BufWriter;

use fst::SetBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();
    let unaries_path = format!("{}/fst/unaries.fst", project_root);
    let unaries_file_handle = File::create(unaries_path)?;
    let buffered_writer = BufWriter::new(unaries_file_handle);
    let set_builder = SetBuilder::new(buffered_writer)?;
    let _ = set_builder.finish();

    return Ok(());
}
