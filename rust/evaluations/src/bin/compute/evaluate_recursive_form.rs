use std::collections::HashMap;
use std::env;
use std::process;
use std::sync::Arc;

use fst::Map;

use evaluations::utils::loaders::load_evaluations;

// A compiled unary operator. Constant nullary arguments are evaluated once at
// compile time, and affine operators (λn. mul*n + add) are kept in closed form
// so applying them — even iterated — never loops.
enum Unary {
    // λn. mul*n + add   (covers *, successor chains, and collapsed iterations)
    Linear { mul: u64, add: u64 },
    // λn. inner(n) + 1  (only when inner could not be collapsed to Linear)
    Succ(Box<Unary>),
    // R(u,*,c): λn. u applied n times starting from base
    IterConstBase { op: Box<Unary>, base: u64 },
    // R(u,b,*): λn. u applied count times starting from n
    IterFixedCount { op: Box<Unary>, count: u64 },
}

pub struct Evaluator {
    fst: Arc<Map<Vec<u8>>>,
    memo: HashMap<String, u64>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            fst: load_evaluations(),
            memo: HashMap::new(),
        }
    }

    pub fn evaluate_nullary(&mut self, nullary: &str) -> Result<u64, String> {
        if let Some(&value) = self.memo.get(nullary) {
            return Ok(value);
        }
        if let Some(value) = self.fst.get(nullary.as_bytes()) {
            self.memo.insert(nullary.to_string(), value);
            return Ok(value);
        }

        let evaluation = if nullary == "0" {
            0
        } else if nullary.starts_with('^') {
            let inner_value = self.evaluate_nullary(inner(nullary)?)?;
            inner_value
                .checked_add(1)
                .ok_or_else(|| overflow(nullary))?
        } else if nullary.starts_with('R') {
            let arguments = split_into_top_level_arguments(inner(nullary)?);
            if arguments.len() != 3 {
                return Err(malformed_nullary(nullary));
            }

            let unary = self.compile_unary(&unfreeze_unary(arguments[0])?)?;
            let base = self.evaluate_nullary(arguments[1])?;
            let countdown = self.evaluate_nullary(arguments[2])?;

            self.iterate(&unary, base, countdown)?
        } else {
            return Err(malformed_nullary(nullary));
        };

        self.memo.insert(nullary.to_string(), evaluation);
        Ok(evaluation)
    }

    fn compile_unary(&mut self, unary: &str) -> Result<Unary, String> {
        if unary == "*" {
            return Ok(Unary::Linear { mul: 1, add: 0 });
        }

        if unary.starts_with('^') {
            let compiled_inner = self.compile_unary(inner(unary)?)?;
            return Ok(match compiled_inner {
                Unary::Linear { mul, add } => match add.checked_add(1) {
                    Some(add) => Unary::Linear { mul, add },
                    None => Unary::Succ(Box::new(Unary::Linear { mul, add })),
                },
                other => Unary::Succ(Box::new(other)),
            });
        }

        if !unary.starts_with('R') {
            return Err(malformed_unary(unary));
        }

        let arguments = split_into_top_level_arguments(inner(unary)?);
        if arguments.len() != 3 {
            return Err(malformed_unary(unary));
        }

        let op = self.compile_unary(&unfreeze_unary(arguments[0])?)?;

        if arguments[1] == "*" {
            let base = self.evaluate_nullary(arguments[2])?;
            return Ok(Unary::IterConstBase {
                op: Box::new(op),
                base,
            });
        }

        if arguments[2] != "*" {
            return Err(malformed_unary(unary));
        }

        let count = self.evaluate_nullary(arguments[1])?;

        // Iterating an affine operator a fixed number of times is itself
        // affine — collapse it now so applications are O(1). Fall back to the
        // looping form if the collapsed coefficients overflow u64.
        if let Unary::Linear { mul, add } = op {
            if let Some((mul, add)) = compose_linear(mul, add, count) {
                return Ok(Unary::Linear { mul, add });
            }
        }

        Ok(Unary::IterFixedCount {
            op: Box::new(op),
            count,
        })
    }

    fn apply(&mut self, unary: &Unary, n: u64) -> Result<u64, String> {
        match unary {
            Unary::Linear { mul, add } => mul
                .checked_mul(n)
                .and_then(|product| product.checked_add(*add))
                .ok_or_else(|| overflow("unary application")),
            Unary::Succ(op) => {
                let value = self.apply(op, n)?;
                value.checked_add(1).ok_or_else(|| overflow("successor"))
            }
            Unary::IterConstBase { op, base } => self.iterate(op, *base, n),
            Unary::IterFixedCount { op, count } => self.iterate(op, n, *count),
        }
    }

    fn iterate(&mut self, op: &Unary, base: u64, countdown: u64) -> Result<u64, String> {
        // Affine operators iterate in closed form: no loop, no overflow risk
        // beyond what the true result itself would incur.
        if let Unary::Linear { mul, add } = op {
            if let Some((mul_k, add_k)) = compose_linear(*mul, *add, countdown) {
                return mul_k
                    .checked_mul(base)
                    .and_then(|product| product.checked_add(add_k))
                    .ok_or_else(|| overflow("iteration"));
            }
            return Err(overflow("iteration"));
        }

        let mut current_value = base;
        let mut current_countdown = countdown;
        while current_countdown > 0 {
            current_value = self.apply(op, current_value)?;
            current_countdown -= 1;
        }
        Ok(current_value)
    }
}

// The k-fold composition of λn. mul*n + add is λn. mul^k * n + add * (mul^(k-1) + ... + 1).
// Returns None if the coefficients don't fit in u64.
fn compose_linear(mul: u64, add: u64, k: u64) -> Option<(u64, u64)> {
    if k == 0 {
        return Some((1, 0));
    }
    if mul == 1 {
        return Some((1, add.checked_mul(k)?));
    }
    let exponent = u32::try_from(k).ok()?;
    let mul_k = mul.checked_pow(exponent)?;
    let geometric_sum = if mul == 0 { 1 } else { (mul_k - 1) / (mul - 1) };
    Some((mul_k, add.checked_mul(geometric_sum)?))
}

fn unfreeze_unary(unary: &str) -> Result<String, String> {
    if unary == "#" {
        return Ok("*".to_string());
    }
    if unary.starts_with('^') {
        return Ok(format!("^({})", unfreeze_unary(inner(unary)?)?));
    }
    if !unary.starts_with('R') {
        return Err(malformed_unary(unary));
    }

    let arguments = split_into_top_level_arguments(inner(unary)?);
    if arguments.len() != 3 {
        return Err(malformed_unary(unary));
    }
    if arguments[1] == "#" {
        return Ok(format!("R({},*,{})", arguments[0], arguments[2]));
    }
    if arguments[2] != "#" {
        return Err(malformed_unary(unary));
    }
    Ok(format!("R({},{},*)", arguments[0], arguments[1]))
}

// "^(x)" -> "x", "R(a,b,c)" -> "a,b,c"
fn inner(expression: &str) -> Result<&str, String> {
    if expression.len() < 3 || !expression.ends_with(')') {
        return Err(malformed_nullary(expression));
    }
    Ok(&expression[2..expression.len() - 1])
}

fn split_into_top_level_arguments(arguments: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&arguments[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&arguments[start..]);
    parts
}

fn malformed_nullary(expression: &str) -> String {
    format!(
        "String {} does not correspond to a nullary expression",
        expression
    )
}

fn malformed_unary(expression: &str) -> String {
    format!(
        "String {} does not correspond to a unary expression",
        expression
    )
}

fn overflow(context: &str) -> String {
    format!("u64 overflow while evaluating {}", context)
}

pub fn evaluate_recursive_form(form: &str) -> Result<u64, String> {
    Evaluator::new().evaluate_nullary(form)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <recursive_form>", args[0]);
        eprintln!("Example: {} \"R(^(#),0,^(0))\"", args[0]);
        process::exit(1);
    }

    match evaluate_recursive_form(&args[1]) {
        Ok(value) => println!("{}", value),
        Err(error) => {
            eprintln!("Error: {}", error);
            process::exit(1);
        }
    }
}
