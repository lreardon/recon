use fst::Streamer;
use fst::{Map, MapBuilder};
// use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
// use std::io::{self, BufWriter, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();

    let evaluations_fst_path = format!("{}/fst/evaluations.fst", project_root);
    let tmp_path = format!("{}/fst/tmp_n.fst", project_root);

    let new_nullaries_evaluated_json_file =
        format!("{}/candidates/new_nullaries_evaluated.json", project_root);

    let new_nullaries_evaluated: HashMap<String, u64> =
        serde_json::from_reader(fs::File::open(new_nullaries_evaluated_json_file)?)?;
    let mut new_nullaries_evaluated_sorted: Vec<_> = new_nullaries_evaluated.into_iter().collect();
    new_nullaries_evaluated_sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut new_nullaries_map_builder = MapBuilder::memory();

    for (key, value) in new_nullaries_evaluated_sorted {
        println!("Inserting key: {}, value: {}", key, value);
        new_nullaries_map_builder.insert(key, value)?;
    }

    let new_nullaries_map = new_nullaries_map_builder.into_map();

    let evaluations_bytes = match fs::read(evaluations_fst_path.clone()) {
        Ok(bytes) => bytes,
        Err(_) => Vec::new(),
    };
    let evaluations_map = Map::new(evaluations_bytes).unwrap();

    let mut u = evaluations_map.op().add(&new_nullaries_map).union();
    let mut new_evaluations_map_builder = MapBuilder::new(File::create(&tmp_path)?)?;

    while let Some((k, vs)) = u.next() {
        new_evaluations_map_builder.insert(k, vs.to_vec()[0].value)?;
    }
    new_evaluations_map_builder.finish()?;

    fs::copy(tmp_path, evaluations_fst_path.clone())?;

    Ok(())

    // Create a Map from new_nullaries_evaluated.json
    // let new_nullaries_evaluated_json_bytes = match fs::read(new_nullaries_evaluated_json_file) {
    //     Ok(bytes) => bytes,
    //     Err(_) => Vec::new(), // Handle the case where the file does not exist
    // };
    // let new_nullaries_evaluated_json =
    //     serde_json::from_slice::<Map<String, u64>>(&new_nullaries_evaluated_json_bytes)?;
    // println!(
    //     "new_nullaries_evaluated_json length: {}",
    //     new_nullaries_evaluated_json.len()
    // );

    // merge_maps(&evaluations_fst_path, &tmp_fst_path)?;
    // ----

    // -- Observing the updated evaluations.fst --
    // let evaluations_bytes = match fs::read(evaluations_fst_path) {
    // 		Ok(bytes) => bytes,
    // 		Err(_) => Vec::new(), // Handle the case where the file does not exist
    // };
    // let new_evaluations_map = Map::new(evaluations_bytes).unwrap();

    // println!("Evaluations map length: {}", new_evaluations_map.len());
    // println!("Is prince in map?: {}", new_evaluations_map.get("prince").unwrap_or(0));
    // println!("Is nobody in map?: {}", new_evaluations_map.get("nobody").unwrap_or(0));
    // // ----

    // 	let data = <usize as TryInto<u64>>::try_into(new_evaluations_map.len()).unwrap().to_string();
    //   let f = File::create("output.txt").expect("Unable to create file");
    //   let mut f = BufWriter::new(f);
    //   f.write_all(data.as_bytes()).expect("Unable to write data");

    // Ok(())
}

// fn create_tmp_map(path: &String) -> Result<(), Box<dyn std::error::Error>> {
// 	let data = vec![
// 		("prince", 1975),
// 		("randy", 1972),
// 		("tom", 1972),
// 	];

// 	let file_handle = File::create(path)?;
// 	let buffered_writer = io::BufWriter::new(file_handle);
// 	let mut map_builder = MapBuilder::new(buffered_writer)?;

// 	for (key, value) in data {
// 		map_builder.insert(key, value).unwrap();
// 	}

// 	map_builder.finish()?;
// 	Ok(())
// }

// fn create_initial_map(path: &String) -> Result<(), Box<dyn std::error::Error>> {
// 	let file_handle = File::create(path)?;
// 	let buffered_writer = io::BufWriter::new(file_handle);
// 	let mut map_builder = MapBuilder::new(buffered_writer)?;
// 	map_builder.insert("bruce", 1972).unwrap();
// 	map_builder.insert("clarence", 1972).unwrap();
// 	map_builder.insert("stevie", 1975).unwrap();
// 	map_builder.finish()?;
// 	Ok(())
// }

// fn merge_maps(base_path: &String, merge_path: &String) -> Result<(), Box<dyn std::error::Error>> {
//     let project_root = env::var("PROJECT_ROOT").unwrap();
//     let tmp_path = format!("{}/fst/new_evaluations.fst", project_root);

//     let evaluations_bytes = match fs::read(base_path) {
//         Ok(bytes) => bytes,
//         Err(_) => Vec::new(),
//     };
//     let evaluations = Map::new(evaluations_bytes).unwrap(); // Read the base file

//     println!("Evaluations map length: {}", evaluations.len());

//     // Read the tmp file at runtime
//     let tmp_bytes = match fs::read(merge_path) {
//         Ok(bytes) => bytes,
//         Err(_) => Vec::new(), // Handle the case where the file does not exist
//     };
//     let tmp = Map::new(tmp_bytes)?; // Read the tmp file

//     let mut u = evaluations.op().add(&tmp).union(); // Merge the two maps

//     let mut new_evaluations_map_builder = MapBuilder::new(File::create(&tmp_path)?)?; // Create a new map at a temporary location

//     while let Some((k, vs)) = u.next() {
//         new_evaluations_map_builder.insert(k, vs.to_vec()[0].value)?; // Insert the merged key-value pairs into the new map. vs is a stream of values, but it should only have one value.
//     }
//     new_evaluations_map_builder.finish()?; // Finish writing the new map

//     fs::copy(tmp_path, base_path)?; // Overwrite the base map with the merged map

//     Ok(())
// }
