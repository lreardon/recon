// Imports the `File` type into this scope and the entire `std::io` module.
use std::fs::{self, File};
use std::io;

// Imports the `SetBuilder` type from the `fst` module.
// use fst::SetBuilder;
use fst::{Map, MapBuilder};
// use fst::{MapBuilder};
use fst::Streamer;



fn main()  -> Result<(), Box<dyn std::error::Error>> {
	let file_handle = File::create("data/evaluations.fst")?;

	let buffered_writer = io::BufWriter::new(file_handle);
	let mut map_builder = MapBuilder::new(buffered_writer)?;

	// Inserts are the same as before, except we include a value with each key.
	map_builder.insert("bruce", 1972).unwrap();
	map_builder.insert("clarence", 1972).unwrap();
	map_builder.insert("stevie", 1975).unwrap();

	// These steps are exactly the same as before.
	map_builder.finish()?;

	let _created_tmp_map = create_tmp_map();

	// Read the tmp file at runtime
	let evaluations_bytes = match fs::read("data/evaluations.fst") {
			Ok(bytes) => bytes,
			Err(_) => Vec::new(), // Handle the case where the file does not exist
	};
	let evaluations = Map::new(evaluations_bytes).unwrap();

	println!("Evaluations map length: {}", evaluations.len());

	// Read the tmp file at runtime
	let tmp_bytes = match fs::read("data/tmp.fst") {
			Ok(bytes) => bytes,
			Err(_) => Vec::new(), // Handle the case where the file does not exist
	};
	let tmp = Map::new(tmp_bytes)?;

	let mut u = evaluations.op().add(&tmp).union();

	let mut new_evaluations_map_builder = MapBuilder::new(File::create("data/new_evaluations.fst")?)?;

	while let Some((k, vs)) = u.next() {
    new_evaluations_map_builder.insert(k, vs.to_vec()[0].value)?;
	}

	new_evaluations_map_builder.finish()?;

	
	fs::copy("data/new_evaluations.fst", "data/evaluations.fst")?;


	let evaluations_bytes = match fs::read("data/evaluations.fst") {
			Ok(bytes) => bytes,
			Err(_) => Vec::new(), // Handle the case where the file does not exist
	};
	let new_evaluations_map = Map::new(evaluations_bytes).unwrap();

	println!("Evaluations map length: {}", new_evaluations_map.len());


	Ok(())
}

fn create_tmp_map() -> Result<(), Box<dyn std::error::Error>> {
	let file_handle = File::create("data/tmp.fst")?;
	let buffered_writer = io::BufWriter::new(file_handle);
	let mut map_builder = MapBuilder::new(buffered_writer)?;
	map_builder.insert("prince", 1975).unwrap();
	map_builder.insert("randy", 1972).unwrap();
	map_builder.insert("tom", 1972).unwrap();
	map_builder.finish()?;

	Ok(())
}