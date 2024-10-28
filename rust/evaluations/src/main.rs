// Imports the `File` type into this scope and the entire `std::io` module.
use std::fs::File;
use std::io;

// Imports the `SetBuilder` type from the `fst` module.
// use fst::SetBuilder;
use fst::{Map, MapBuilder};
// use fst::{MapBuilder};
use fst::Streamer;



fn main()  -> Result<(), Box<dyn std::error::Error>> {
	// Create a file handle that will write to "set.fst" in the current directory.
	let file_handle = File::create("data/evaluations.fst")?;

	// Make sure writes to the file are buffered.
	let buffered_writer = io::BufWriter::new(file_handle);

	// Create a set builder that streams the data structure to set.fst.
	// We could use a socket here, or an in memory buffer, or anything that
	// is "writable" in Rust.
	// let mut set_builder = SetBuilder::new(buffered_writer)?;

	// // Insert a few keys from the greatest band of all time.
	// // An insert can fail in one of two ways: either a key was inserted out of
	// // order or there was a problem writing to the underlying file.
	// set_builder.insert("bruce")?;
	// set_builder.insert("clarence")?;
	// set_builder.insert("stevie")?;

	// // Finish building the set and make sure the entire data structure is flushed
	// // to disk. After this is called, no more inserts are allowed. (And indeed,
	// // are prevented by Rust's type/ownership system!)
	// set_builder.finish()?;

	// Create a map builder that streams the data structure to memory.
	let mut map_builder = MapBuilder::new(buffered_writer)?;

	// Inserts are the same as before, except we include a value with each key.
	map_builder.insert("bruce", 1972).unwrap();
	map_builder.insert("clarence", 1972).unwrap();
	map_builder.insert("stevie", 1975).unwrap();

	// These steps are exactly the same as before.
	map_builder.finish()?;

	let _created_tmp_map = create_tmp_map();

	static EVALUATIONS_FST: &[u8] = include_bytes!("../data/evaluations.fst");
	let evaluations = Map::new(EVALUATIONS_FST).unwrap();

	println!("Evaluations map length: {}", evaluations.len());

	static TMP_FST: &[u8] = include_bytes!("../data/tmp.fst");
	let tmp = Map::new(TMP_FST).unwrap();

	let mut u = evaluations.op().add(&tmp).union();
	let mut kvs = vec![];
	while let Some((k, vs)) = u.next() {
    kvs.push((k.to_vec(), vs.to_vec()[0].value));
	}

	let new_evaluations = Map::from_iter(kvs).unwrap();

	println!("new_evaluations length: {}", new_evaluations.len());
	println!("new_evaluation of prince: {}", new_evaluations.get("prince").unwrap().to_string());

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