use std::env;
use std::process;

use evaluations::engine::{Engine, EngineConfig, Mode, RunOutcome};


fn usage(program: &str) -> ! {
    eprintln!("Usage: {} [options]", program);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --mode <length|depth>  exploration mode (default length)");
    eprintln!("                         length: exhaustive by form length (certifies minimum forms)");
    eprintln!("                         depth:  by recursive depth (the original generation search)");
    eprintln!("  --units <n>       strata (length mode) or depths (depth mode) to complete (default 1)");
    eprintln!("  --depths <n>      alias for --units");
    eprintln!("  --max-items <n>   stop after n synthesis items / pairs, committing state");
    eprintln!("  --max-pairs <n>   alias for --max-items");
    eprintln!("  --batch-keys <n>  commit after n new keys accumulate (default 50000)");
    eprintln!("  --batch-secs <n>  commit at least every n seconds (default 30)");
    eprintln!("  --compact-mb <n>  compact recent layer into base beyond n MB (default 4)");
    eprintln!("  --step-limit <n>  abort a single evaluation after n unary applications");
    eprintln!("  --retry-limited   re-attempt step-limited forms (use with a larger --step-limit);");
    eprintln!("                    rolls the sweep back if a resolved form invalidates the watermark");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let project_root = match env::var("PROJECT_ROOT") {
        Ok(root) => root,
        Err(_) => {
            eprintln!("Error: PROJECT_ROOT is not set");
            process::exit(1);
        }
    };

    let mut config = EngineConfig::default();
    let mut mode = Mode::Length;
    let mut units: u64 = 1;
    let mut max_items: Option<u64> = None;
    let mut retry_limited = false;

    let mut index = 1;
    while index < args.len() {
        let parse = |name: &str, value: Option<&String>| -> u64 {
            match value.and_then(|v| v.parse().ok()) {
                Some(parsed) => parsed,
                None => {
                    eprintln!("Error: {} requires a numeric argument", name);
                    usage(&program);
                }
            }
        };
        match args[index].as_str() {
            "--mode" => {
                mode = match args.get(index + 1).map(String::as_str) {
                    Some("length") => Mode::Length,
                    Some("depth") => Mode::Depth,
                    _ => {
                        eprintln!("Error: --mode must be 'length' or 'depth'");
                        usage(&program);
                    }
                }
            }
            "--units" | "--depths" => units = parse("--units", args.get(index + 1)),
            "--max-items" | "--max-pairs" => {
                max_items = Some(parse("--max-items", args.get(index + 1)))
            }
            "--batch-keys" => {
                config.batch_keys = parse("--batch-keys", args.get(index + 1)) as usize
            }
            "--batch-secs" => config.batch_secs = parse("--batch-secs", args.get(index + 1)),
            "--compact-mb" => {
                config.compact_bytes = parse("--compact-mb", args.get(index + 1)) * 1024 * 1024
            }
            "--step-limit" => config.step_limit = parse("--step-limit", args.get(index + 1)),
            "--retry-limited" => {
                retry_limited = true;
                index -= 1; // flag takes no argument
            }
            other => {
                eprintln!("Error: unknown option {}", other);
                usage(&program);
            }
        }
        index += 2;
    }

    let mut engine = match Engine::open(&project_root, config) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("Error opening engine: {}", error);
            process::exit(1);
        }
    };

    if retry_limited {
        match engine.retry_step_limited() {
            Ok(report) => {
                println!(
                    "Retry: {} resolved, {} still step-limited, {} overflowed",
                    report.resolved, report.still_limited, report.overflowed
                );
                if let Some(length) = report.rolled_back_to {
                    println!(
                        "Sweep rolled back: exhaustive through length {} — run `recon` again to \
                         re-certify the lengths above it (the replay is cheap)",
                        length
                    );
                }
            }
            Err(error) => {
                eprintln!("Error retrying step-limited forms: {}", error);
                process::exit(1);
            }
        }
        return;
    }

    let describe = |engine: &Engine| match mode {
        Mode::Length => format!(
            "stratum {} item {} (exhaustive through length {})",
            engine.length_progress().length,
            engine.length_progress().item,
            engine.length_progress().complete_through_length,
        ),
        Mode::Depth => format!(
            "depth {} phase {}",
            engine.depth_progress().depth,
            engine.depth_progress().exploration_phase,
        ),
    };

    println!(
        "Exploring ({} mode) from {} — {} evaluations, {} unaries known",
        if mode == Mode::Length { "length" } else { "depth" },
        describe(&engine),
        engine.evaluations_len(),
        engine.unaries_len(),
    );

    match engine.run(mode, units, max_items) {
        Ok(RunOutcome::Completed) => {
            println!(
                "Done: {} — {} evaluations, {} unaries",
                describe(&engine),
                engine.evaluations_len(),
                engine.unaries_len(),
            );
            if engine.step_limited_count() > 0 {
                println!(
                    "Note: {} forms hit the step limit and await retry (recon --retry-limited \
                     --step-limit <bigger>); certification is conditional until they resolve",
                    engine.step_limited_count()
                );
            }
        }
        Ok(RunOutcome::BudgetExhausted) => {
            println!(
                "Paused at item budget: {} — {} evaluations, {} unaries",
                describe(&engine),
                engine.evaluations_len(),
                engine.unaries_len(),
            );
        }
        Err(error) => {
            eprintln!("Error during exploration: {}", error);
            process::exit(1);
        }
    }
}
