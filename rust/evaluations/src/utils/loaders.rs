use std::env;

use fst::Map;
use std::fs::File;

use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    static ref EVALUATIONS: Arc<Map<Vec<u8>>> = {
        let project_root = env::var("PROJECT_ROOT").unwrap();
        let evaluations_fst_path = format!("{}/fst/evaluations.fst", project_root);

        let fst_bytes = std::fs::read(evaluations_fst_path)
            .expect("Failed to read FST file");

        // Create the Set and wrap it in Arc for thread-safe sharing
        Arc::new(Map::new(fst_bytes)
            .expect("Failed to create FST set"))
    };
}

pub struct LocalPath {
    pub path: String,
}

pub fn load_fst_map(
    params: LocalPath,
) -> Result<Arc<fst::Map<Vec<u8>>>, Box<dyn std::error::Error>> {
    let project_root: String = env::var("PROJECT_ROOT").unwrap();
    let fst_path: String = format!("{}/fst/{}.fst", project_root, params.path);

    let map = Arc::new(Map::new(std::fs::read(fst_path).unwrap()).unwrap());

    return Ok(map);
}

pub fn load_file(params: LocalPath) -> Result<File, Box<dyn std::error::Error>> {
    let project_root: String = env::var("PROJECT_ROOT").unwrap();
    let file_path: String = format!("{}/{}", project_root, params.path);

    let file = File::open(file_path)?;

    return Ok(file);
}

pub fn load_evaluations() -> Arc<Map<Vec<u8>>> {
    EVALUATIONS.clone() // This clones the Arc, not the underlying FST
}
