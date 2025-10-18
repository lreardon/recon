use std::env;

use fst::Map;
use std::fs::File;

use std::sync::Arc;

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
