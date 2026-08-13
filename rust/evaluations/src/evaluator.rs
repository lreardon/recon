use std::collections::HashMap;
use std::fmt;

// Why an evaluation could not produce a value. Overflow and Malformed are
// permanent verdicts (they propagate to every superterm); StepLimit only means
// "not evaluated within this run's budget" and the form may be retried later
// with a larger limit.
#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    Overflow(String),
    StepLimit(u64),
    Malformed(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Overflow(context) => write!(f, "u64 overflow while evaluating {}", context),
            EvalError::StepLimit(limit) => write!(f, "step limit of {} exceeded", limit),
            EvalError::Malformed(expression) => write!(
                f,
                "String {} does not correspond to a well-formed expression",
                expression
            ),
        }
    }
}

// Source of already-known nullary values (FST layers, pending batches, ...).
// The evaluator consults this before doing any structural work.
pub trait EvalLookup {
    fn lookup(&self, key: &str) -> Option<u64>;
}

impl EvalLookup for HashMap<String, u64> {
    fn lookup(&self, key: &str) -> Option<u64> {
        self.get(key).copied()
    }
}

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

pub struct Evaluator<'a> {
    lookup: &'a dyn EvalLookup,
    memo: &'a mut HashMap<String, u64>,
    steps: u64,
    step_limit: Option<u64>,
}

impl<'a> Evaluator<'a> {
    pub fn new(lookup: &'a dyn EvalLookup, memo: &'a mut HashMap<String, u64>) -> Self {
        Evaluator {
            lookup,
            memo,
            steps: 0,
            step_limit: None,
        }
    }

    // Bound the number of unary applications so a pathological form errors out
    // instead of wedging the process.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }

    pub fn evaluate_nullary(&mut self, nullary: &str) -> Result<u64, EvalError> {
        if let Some(&value) = self.memo.get(nullary) {
            return Ok(value);
        }
        if let Some(value) = self.lookup.lookup(nullary) {
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

    fn compile_unary(&mut self, unary: &str) -> Result<Unary, EvalError> {
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

    fn apply(&mut self, unary: &Unary, n: u64) -> Result<u64, EvalError> {
        self.count_step()?;
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

    fn iterate(&mut self, op: &Unary, base: u64, countdown: u64) -> Result<u64, EvalError> {
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

    fn count_step(&mut self) -> Result<(), EvalError> {
        self.steps += 1;
        if let Some(limit) = self.step_limit {
            if self.steps > limit {
                return Err(EvalError::StepLimit(limit));
            }
        }
        Ok(())
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

pub fn unfreeze_unary(unary: &str) -> Result<String, EvalError> {
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
fn inner(expression: &str) -> Result<&str, EvalError> {
    if expression.len() < 3 || !expression.ends_with(')') {
        return Err(malformed_nullary(expression));
    }
    Ok(&expression[2..expression.len() - 1])
}

pub fn split_into_top_level_arguments(arguments: &str) -> Vec<&str> {
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

fn malformed_nullary(expression: &str) -> EvalError {
    EvalError::Malformed(expression.to_string())
}

fn malformed_unary(expression: &str) -> EvalError {
    EvalError::Malformed(expression.to_string())
}

fn overflow(context: &str) -> EvalError {
    EvalError::Overflow(context.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(form: &str) -> Result<u64, EvalError> {
        let lookup: HashMap<String, u64> = HashMap::new();
        let mut memo = HashMap::new();
        Evaluator::new(&lookup, &mut memo).evaluate_nullary(form)
    }

    // Pinned to the ruby evaluator's outputs (verified 2026-08-09).
    #[test]
    fn parity_with_ruby_evaluator() {
        assert_eq!(eval("0"), Ok(0));
        assert_eq!(eval("^(0)"), Ok(1));
        assert_eq!(eval("^(^(^(0)))"), Ok(3));
        assert_eq!(eval("R(^(#),0,^(0))"), Ok(1));
        assert_eq!(eval("R(^(#),0,^(^(0)))"), Ok(2));
        assert_eq!(eval("R(^(#),^(^(0)),^(^(^(0))))"), Ok(5));
        assert_eq!(eval("R(R(^(#),#,^(^(0))),0,^(^(0)))"), Ok(4));
        assert_eq!(eval("R(R(^(#),^(^(0)),#),0,^(^(^(0))))"), Ok(6));
        assert_eq!(eval("R(R(R(^(#),^(^(0)),#),#,0),^(0),^(^(^(0))))"), Ok(8));
    }

    #[test]
    fn zero_countdown_returns_base() {
        assert_eq!(eval("R(^(#),^(^(0)),0)"), Ok(2));
    }

    #[test]
    fn doubling_tower_evaluates_in_closed_form() {
        let countdown = format!("{}0{}", "^(".repeat(30), ")".repeat(30));
        let form = format!("R(R(R(^(#),^(^(0)),#),#,0),^(0),{})", countdown);
        assert_eq!(eval(&form), Ok(1 << 30));
    }

    #[test]
    fn lookup_short_circuits_evaluation() {
        let mut lookup = HashMap::new();
        // Deliberately wrong value proves the lookup was used instead of evaluation.
        lookup.insert("^(0)".to_string(), 41);
        let mut memo = HashMap::new();
        let mut evaluator = Evaluator::new(&lookup, &mut memo);
        assert_eq!(evaluator.evaluate_nullary("^(^(0))"), Ok(42));
    }

    #[test]
    fn memo_is_populated_for_subexpressions() {
        let lookup: HashMap<String, u64> = HashMap::new();
        let mut memo = HashMap::new();
        Evaluator::new(&lookup, &mut memo)
            .evaluate_nullary("^(^(0))")
            .unwrap();
        assert_eq!(memo.get("^(0)"), Some(&1));
        assert_eq!(memo.get("^(^(0))"), Some(&2));
    }

    #[test]
    fn malformed_forms_error() {
        assert!(matches!(eval("*"), Err(EvalError::Malformed(_))));
        assert!(matches!(eval("Q(0)"), Err(EvalError::Malformed(_))));
        assert!(matches!(eval("R(^(#),0)"), Err(EvalError::Malformed(_))));
    }

    #[test]
    fn step_limit_stops_runaway_iteration() {
        let lookup: HashMap<String, u64> = HashMap::new();
        let mut memo = HashMap::new();
        // R(^(#),*,0) computes the identity but is represented non-linearly
        // (IterConstBase), so iterating it 2^30 times must actually loop —
        // without ever overflowing. Only the step limit can stop it.
        let countdown = format!("{}0{}", "^(".repeat(30), ")".repeat(30));
        let two_to_thirty = format!("R(R(R(^(#),^(^(0)),#),#,0),^(0),{})", countdown);
        let slow_identity_iter = format!("R(R(^(#),#,0),0,{})", two_to_thirty);
        let result = Evaluator::new(&lookup, &mut memo)
            .with_step_limit(1_000)
            .evaluate_nullary(&slow_identity_iter);
        assert_eq!(result, Err(EvalError::StepLimit(1_000)));
    }

    #[test]
    fn unfreeze_matches_ruby_semantics() {
        assert_eq!(unfreeze_unary("#"), Ok("*".to_string()));
        assert_eq!(unfreeze_unary("^(#)"), Ok("^(*)".to_string()));
        assert_eq!(
            unfreeze_unary("R(^(#),#,^(0))"),
            Ok("R(^(#),*,^(0))".to_string())
        );
        assert_eq!(
            unfreeze_unary("R(^(#),^(0),#)"),
            Ok("R(^(#),^(0),*)".to_string())
        );
        assert!(matches!(
            unfreeze_unary("R(^(#),^(0),^(0))"),
            Err(EvalError::Malformed(_))
        ));
    }
}
