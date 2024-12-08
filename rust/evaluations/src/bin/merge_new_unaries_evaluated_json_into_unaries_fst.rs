use fst::Streamer;
use fst::{Set, SetBuilder};
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = env::var("PROJECT_ROOT").unwrap();

    let unaries_fst_path = format!("{}/fst/unaries.fst", project_root);
    let tmp_path = format!("{}/fst/tmp_u.fst", project_root);

    let new_unaries_file_path = format!("{}/candidates/new_unaries.txt", project_root);
    let new_unaries_file = File::open(new_unaries_file_path)?;
    let new_unaries: Vec<String> = BufReader::new(new_unaries_file)
        .lines()
        .collect::<Result<_, _>>()?;
    let mut new_unaries_sorted: Vec<_> = new_unaries.into_iter().collect();
    new_unaries_sorted.sort();

    let mut new_unaries_set_builder = SetBuilder::memory();

    for value in new_unaries_sorted {
        println!("Inserting: {}", value);
        new_unaries_set_builder.insert(value)?;
    }

    let new_unaries_set = new_unaries_set_builder.into_set();

    let unaries_bytes = match fs::read(unaries_fst_path.clone()) {
        Ok(bytes) => bytes,
        Err(_) => Vec::new(),
    };
    let unaries_set = Set::new(unaries_bytes).unwrap();

    let mut u = unaries_set.op().add(&new_unaries_set).union();

    let mut updated_unaries_set_builder = SetBuilder::new(File::create(&tmp_path)?)?;

    while let Some(v) = u.next() {
        updated_unaries_set_builder.insert(v)?;
    }
    updated_unaries_set_builder.finish()?;

    fs::copy(tmp_path, unaries_fst_path.clone())?;

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
