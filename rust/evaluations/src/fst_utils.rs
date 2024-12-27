use std::env;
use std::sync::Arc;
use fst::Map;
use lazy_static::lazy_static;

// We use lazy_static to ensure the FST is loaded only once and shared across threads
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

// Export the FST_SET for use in other modules
pub fn get_evaluations_fst() -> Arc<Map<Vec<u8>>> {
    EVALUATIONS.clone() // This clones the Arc, not the underlying FST
}