use std::env;
use std::fs;
use std::path::Path;
use std::process;

// Look up the shortest known expression for a value in fst/shortest_by_value.tsv.
//   min_form            print the whole index
//   min_form <value>    print the shortest known expression for <value>
//   min_form <value> -v also report whether the answer is definitive: the
//                       length sweep's watermark certifies every form length
//                       up to it as exhaustively enumerated.
fn main() {
    let args: Vec<String> = env::args().collect();
    let project_root = match env::var("PROJECT_ROOT") {
        Ok(root) => root,
        Err(_) => {
            eprintln!("Error: PROJECT_ROOT is not set");
            process::exit(1);
        }
    };

    let path = Path::new(&project_root).join("fst").join("shortest_by_value.tsv");
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("Error reading {}: {}", path.display(), error);
            process::exit(1);
        }
    };

    match args.get(1) {
        None => print!("{}", contents),
        Some(wanted) => {
            let verbose = args.get(2).map(String::as_str) == Some("-v");
            for line in contents.lines() {
                if let Some((value, expression)) = line.split_once('\t') {
                    if value == wanted {
                        if verbose {
                            let watermark = watermark(&project_root);
                            let expression_len = expression.len() as u64;
                            let shorter_limited =
                                step_limited_shorter_than(&project_root, expression_len);
                            let status = if expression_len > watermark {
                                "provisional".to_string()
                            } else if shorter_limited > 0 {
                                format!(
                                    "conditional ({} step-limited forms shorter than this await retry)",
                                    shorter_limited
                                )
                            } else {
                                "definitive".to_string()
                            };
                            println!(
                                "{}\t{}\t{} (exhaustive through length {})",
                                expression,
                                expression.len(),
                                status,
                                watermark
                            );
                        } else {
                            println!("{}", expression);
                        }
                        return;
                    }
                }
            }
            process::exit(1);
        }
    }
}

fn step_limited_shorter_than(project_root: &str, length: u64) -> usize {
    let path = Path::new(project_root).join("state").join("step_limited.tsv");
    fs::read_to_string(path)
        .map(|contents| {
            contents
                .lines()
                .filter(|line| !line.trim().is_empty() && (line.len() as u64) < length)
                .count()
        })
        .unwrap_or(0)
}

fn watermark(project_root: &str) -> u64 {
    let path = Path::new(project_root)
        .join("state")
        .join("length_progress.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .and_then(|json| json["complete_through_length"].as_u64())
        .unwrap_or(0)
}
