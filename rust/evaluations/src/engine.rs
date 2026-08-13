use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::evaluator::{EvalError, EvalLookup, Evaluator};
use crate::store::{write_manifest, Frontier, MapStore, MinIndex, SetStore};

pub const PHASE_READY: &str = "exploring_ready";
pub const PHASE_NEW_NULLARIES: &str = "exploring_new_nullaries_all_unaries";
pub const PHASE_OLD_NULLARIES: &str = "exploring_old_nullaries_new_unaries";

// ---------------------------------------------------------------------------
// Cursors. Each exploration mode has its own independently persisted cursor,
// so switching modes pauses one and resumes the other without interference.
// Both are written atomically, only after the batch they describe has been
// committed. All discovered values land in the same shared stores, so neither
// mode ever re-evaluates a form the other already handled.
// ---------------------------------------------------------------------------

// Depth (generation) mode cursor — same shape as the original ruby Progress
// record in state/progress.json. Indices name the NEXT pair to process.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Progress {
    pub depth: u64,
    pub nullary_index: u64,
    pub unary_index: u64,
    pub exploration_phase: String,
    pub nullary_file_index: u64,
    pub unary_file_index: u64,
}

impl Progress {
    pub fn start_of_depth(depth: u64) -> Self {
        Progress {
            depth,
            nullary_index: 0,
            unary_index: 0,
            exploration_phase: PHASE_READY.to_string(),
            nullary_file_index: 0,
            unary_file_index: 0,
        }
    }

    pub fn load(project_root: &str) -> Result<Self, String> {
        let path = Path::new(project_root).join("state").join("progress.json");
        if !path.exists() {
            return Ok(Progress::start_of_depth(0));
        }
        let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut progress: Progress = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
        // Legacy alias used by an old ruby default.
        if progress.exploration_phase == "exploration_ready" {
            progress.exploration_phase = PHASE_READY.to_string();
        }
        Ok(progress)
    }

    pub fn save(&self, project_root: &str) -> Result<(), String> {
        save_json(project_root, "progress.json", self)
    }
}

// Length (exhaustive) mode cursor. Stratum `length` is being synthesized;
// `item` counts fully-processed synthesis items within it (the synthesis order
// is deterministic, so an item index is a complete resume coordinate).
// `complete_through_length`: every well-formed form of length <= this has been
// enumerated and evaluated — the min-index is definitive up to it.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LengthProgress {
    pub length: u64,
    pub item: u64,
    pub complete_through_length: u64,
}

impl LengthProgress {
    pub fn load(project_root: &str) -> Result<Self, String> {
        let path = Path::new(project_root)
            .join("state")
            .join("length_progress.json");
        if !path.exists() {
            return Ok(LengthProgress {
                length: 1,
                item: 0,
                complete_through_length: 0,
            });
        }
        let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&contents).map_err(|e| e.to_string())
    }

    pub fn save(&self, project_root: &str) -> Result<(), String> {
        save_json(project_root, "length_progress.json", self)
    }
}

fn save_json<T: Serialize>(project_root: &str, file: &str, value: &T) -> Result<(), String> {
    let dir = Path::new(project_root).join("state");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let rendered = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp_path = dir.join(format!("{}.tmp", file));
    fs::write(&tmp_path, rendered).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, dir.join(file)).map_err(|e| e.to_string())?;
    Ok(())
}

fn step_limited_path(project_root: &str) -> std::path::PathBuf {
    Path::new(project_root).join("state").join("step_limited.tsv")
}

fn load_step_limited(project_root: &str) -> Result<BTreeSet<String>, String> {
    let path = step_limited_path(project_root);
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn save_step_limited(project_root: &str, forms: &BTreeSet<String>) -> Result<(), String> {
    let path = step_limited_path(project_root);
    let mut rendered = String::new();
    for form in forms {
        rendered.push_str(form);
        rendered.push('\n');
    }
    let tmp_path = path.with_extension("tsv.tmp");
    fs::write(&tmp_path, rendered).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Length,
    Depth,
}

enum Cursor {
    Depth(Progress),
    Length(LengthProgress),
}

pub struct EngineConfig {
    pub batch_keys: usize,
    pub batch_secs: u64,
    pub compact_bytes: u64,
    pub step_limit: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            batch_keys: 50_000,
            batch_secs: 30,
            compact_bytes: 4 * 1024 * 1024,
            step_limit: 50_000_000,
        }
    }
}

#[derive(Default)]
pub struct Stats {
    pub items: u64,
    pub dedup_hits: u64,
    pub evaluations: u64,
    pub overflows: u64,
    pub step_limited: u64,
    pub new_evaluations: u64,
    pub new_unaries: u64,
    pub commits: u64,
    pub compactions: u64,
}

struct TieredLookup<'a> {
    store: &'a MapStore,
    pending: &'a BTreeMap<String, u64>,
}

impl EvalLookup for TieredLookup<'_> {
    fn lookup(&self, key: &str) -> Option<u64> {
        self.pending
            .get(key)
            .copied()
            .or_else(|| self.store.get(key))
    }
}

pub enum RunOutcome {
    Completed,
    BudgetExhausted,
}

pub struct Engine {
    root: String,
    config: EngineConfig,
    evaluations: MapStore,
    unaries: SetStore,
    min_index: MinIndex,
    depth_progress: Progress,
    length_progress: LengthProgress,

    // Depth mode only: everything ever placed in a depth frontier. This is
    // deliberately separate from the value store — a form the *length* sweep
    // discovered is value-known but must still enter a depth frontier the
    // first time depth mode produces it, or its descendants would be missed.
    depth_seen_nullaries: SetStore,
    depth_seen_unaries: SetStore,

    memo: HashMap<String, u64>,
    failed: HashSet<String>,
    // Forms whose evaluation exceeded the step limit. Unlike overflow, this is
    // NOT a permanent verdict — they are persisted (state/step_limited.tsv),
    // excluded from strata/frontiers and from certification, and can be
    // revisited later via retry_step_limited with a larger limit.
    step_limited: BTreeSet<String>,
    step_limited_dirty: bool,
    strata_cache: HashMap<(&'static str, u64), Rc<Vec<String>>>,

    pending_evaluations: BTreeMap<String, u64>,
    pending_unaries: BTreeSet<String>,
    pending_frontier_nullaries: BTreeSet<String>,
    pending_frontier_unaries: BTreeSet<String>,
    pending_seen_nullaries: BTreeSet<String>,
    pending_seen_unaries: BTreeSet<String>,

    pub stats: Stats,
    last_commit: Instant,
    items_budget: Option<u64>,
}

impl Engine {
    pub fn open(project_root: &str, config: EngineConfig) -> Result<Self, String> {
        Frontier::ensure_seeds(project_root)?;
        let evaluations = MapStore::open_evaluations(project_root)?;
        let unaries = SetStore::open_unaries(project_root)?;
        let min_index = MinIndex::open(project_root, &evaluations)?;
        let depth_progress = Progress::load(project_root)?;
        let length_progress = LengthProgress::load(project_root)?;

        let state_dir = Path::new(project_root).join("state");
        let seen_missing = !state_dir.join("depth_seen_nullaries.fst").exists();
        let mut depth_seen_nullaries = SetStore::open_at(&state_dir, "depth_seen_nullaries")?;
        let mut depth_seen_unaries = SetStore::open_at(&state_dir, "depth_seen_unaries")?;

        // First open with an existing chains directory: everything already in a
        // depth frontier counts as seen.
        if seen_missing {
            for (kind, seen) in [
                ("nullaries", &mut depth_seen_nullaries),
                ("unaries", &mut depth_seen_unaries),
            ] {
                let mut all = BTreeSet::new();
                for index in Frontier::indices(project_root, "chains", kind) {
                    all.extend(Frontier::load(project_root, "chains", kind, index)?);
                }
                seen.commit(&all)?;
            }
        }

        Ok(Engine {
            root: project_root.to_string(),
            config,
            evaluations,
            unaries,
            min_index,
            depth_progress,
            length_progress,
            depth_seen_nullaries,
            depth_seen_unaries,
            memo: HashMap::new(),
            failed: HashSet::new(),
            step_limited: load_step_limited(project_root)?,
            step_limited_dirty: false,
            strata_cache: HashMap::new(),
            pending_evaluations: BTreeMap::new(),
            pending_unaries: BTreeSet::new(),
            pending_frontier_nullaries: BTreeSet::new(),
            pending_frontier_unaries: BTreeSet::new(),
            pending_seen_nullaries: BTreeSet::new(),
            pending_seen_unaries: BTreeSet::new(),
            stats: Stats::default(),
            last_commit: Instant::now(),
            items_budget: None,
        })
    }

    pub fn depth_progress(&self) -> &Progress {
        &self.depth_progress
    }

    pub fn length_progress(&self) -> &LengthProgress {
        &self.length_progress
    }

    pub fn evaluations_len(&self) -> usize {
        self.evaluations.len()
    }

    pub fn unaries_len(&self) -> usize {
        self.unaries.len()
    }

    // Explore `units` full units (strata in Length mode, depths in Depth
    // mode), stopping early once `max_items` items have been processed.
    // On-disk state is always consistent on return.
    pub fn run(&mut self, mode: Mode, units: u64, max_items: Option<u64>) -> Result<RunOutcome, String> {
        self.items_budget = max_items;
        for _ in 0..units {
            let outcome = match mode {
                Mode::Length => self.run_one_stratum()?,
                Mode::Depth => self.run_one_depth()?,
            };
            if let RunOutcome::BudgetExhausted = outcome {
                return Ok(RunOutcome::BudgetExhausted);
            }
        }
        Ok(RunOutcome::Completed)
    }

    // -----------------------------------------------------------------------
    // Length mode: synthesize every well-formed form of exact length L, from
    // strictly shorter (already complete) strata. Grammar:
    //   N: "0" (L=1) | ^(N[L-3]) | R(U[lu],N[la],N[lb]) with lu+la+lb = L-5
    //   U: "^(*)" (L=4) | ^(U[L-3]) | R(U[lu],N[ln],*) / R(U[lu],*,N[ln])
    //      with lu+ln = L-6
    // Synthesis order is deterministic, so a single item counter is the cursor.
    // -----------------------------------------------------------------------

    fn run_one_stratum(&mut self) -> Result<RunOutcome, String> {
        let length = self.length_progress.length;
        let resume_item = if self.length_progress.length == length {
            self.length_progress.item
        } else {
            0
        };
        let watermark = self.length_progress.complete_through_length;
        let mut item: u64 = 0;

        // Emits one synthesized form; returns false when the budget stops the run.
        macro_rules! emit {
            ($self:ident, $form:expr, $is_nullary:expr) => {{
                if item < resume_item {
                    item += 1;
                } else {
                    if $self.budget_exhausted() {
                        $self.commit(Cursor::Length(LengthProgress {
                            length,
                            item,
                            complete_through_length: watermark,
                        }))?;
                        return Ok(RunOutcome::BudgetExhausted);
                    }
                    if $is_nullary {
                        $self.process_stratum_nullary($form);
                    } else {
                        $self.process_stratum_unary($form);
                    }
                    item += 1;
                    $self.maybe_commit(Cursor::Length(LengthProgress {
                        length,
                        item,
                        complete_through_length: watermark,
                    }))?;
                }
            }};
        }

        // --- Nullary rules ---
        if length == 1 {
            emit!(self, "0".to_string(), true);
        }
        if length >= 4 {
            let inners = self.stratum("nullaries", length - 3)?;
            for inner in inners.iter() {
                emit!(self, format!("^({})", inner), true);
            }
        }
        if length >= 11 {
            // R(U,A,B): lu+la+lb = length-5, lu >= 4, la,lb >= 1
            for lu in 4..=(length - 7) {
                let unaries = self.stratum("unaries", lu)?;
                if unaries.is_empty() {
                    continue;
                }
                for la in 1..=(length - 6 - lu) {
                    let lb = length - 5 - lu - la;
                    let firsts = self.stratum("nullaries", la)?;
                    let seconds = self.stratum("nullaries", lb)?;
                    if firsts.is_empty() || seconds.is_empty() {
                        continue;
                    }
                    for unary in unaries.iter() {
                        let frozen = unary.replacen('*', "#", 1);
                        for first in firsts.iter() {
                            for second in seconds.iter() {
                                emit!(self, format!("R({},{},{})", frozen, first, second), true);
                            }
                        }
                    }
                }
            }
        }

        // --- Unary rules ---
        if length == 4 {
            emit!(self, "^(*)".to_string(), false);
        }
        if length >= 7 {
            let inners = self.stratum("unaries", length - 3)?;
            for inner in inners.iter() {
                emit!(self, format!("^({})", inner), false);
            }
        }
        if length >= 11 {
            // R(U,N,*) and R(U,*,N): lu+ln = length-6, lu >= 4, ln >= 1
            for lu in 4..=(length - 7) {
                let ln = length - 6 - lu;
                let unaries = self.stratum("unaries", lu)?;
                let nullaries = self.stratum("nullaries", ln)?;
                for unary in unaries.iter() {
                    let frozen = unary.replacen('*', "#", 1);
                    for nullary in nullaries.iter() {
                        emit!(self, format!("R({},{},*)", frozen, nullary), false);
                        emit!(self, format!("R({},*,{})", frozen, nullary), false);
                    }
                }
            }
        }

        // Stratum complete: watermark advances to this length.
        self.commit(Cursor::Length(LengthProgress {
            length: length + 1,
            item: 0,
            complete_through_length: length,
        }))?;
        Ok(RunOutcome::Completed)
    }

    fn stratum(&mut self, kind: &'static str, length: u64) -> Result<Rc<Vec<String>>, String> {
        if let Some(cached) = self.strata_cache.get(&(kind, length)) {
            return Ok(Rc::clone(cached));
        }
        let items = Rc::new(Frontier::load(&self.root, "strata", kind, length)?);
        self.strata_cache.insert((kind, length), Rc::clone(&items));
        Ok(items)
    }

    // A stratum nullary belongs in its stratum file by construction (that is
    // what makes the sweep exhaustive) — unless it overflows, which soundly
    // prunes it and everything built on it.
    fn process_stratum_nullary(&mut self, form: String) {
        self.stats.items += 1;
        if self.failed.contains(&form) || self.step_limited.contains(&form) {
            self.stats.dedup_hits += 1;
            return;
        }
        if self.nullary_value_known(&form) {
            self.stats.dedup_hits += 1;
            self.pending_frontier_nullaries.insert(form);
            return;
        }
        match self.evaluate(&form) {
            Ok(value) => {
                self.min_index.offer(value, &form);
                self.pending_evaluations.insert(form.clone(), value);
                self.pending_frontier_nullaries.insert(form);
                self.stats.new_evaluations += 1;
            }
            Err(error) => self.record_failure(form, error),
        }
    }

    fn process_stratum_unary(&mut self, form: String) {
        self.stats.items += 1;
        if !self.unaries.contains(&form) && !self.pending_unaries.contains(&form) {
            self.pending_unaries.insert(form.clone());
            self.stats.new_unaries += 1;
        } else {
            self.stats.dedup_hits += 1;
        }
        self.pending_frontier_unaries.insert(form);
    }

    // -----------------------------------------------------------------------
    // Depth mode: the original generation-based search, kept for exploring by
    // recursive depth. Candidate unaries now include the ^-wrap.
    // -----------------------------------------------------------------------

    fn run_one_depth(&mut self) -> Result<RunOutcome, String> {
        let depth = self.depth_progress.depth;

        if self.depth_progress.exploration_phase == PHASE_READY {
            self.depth_progress = Progress {
                depth,
                exploration_phase: PHASE_NEW_NULLARIES.to_string(),
                ..Progress::start_of_depth(depth)
            };
        }

        // Phase 1: new nullaries (this depth's frontier) x all unary frontiers.
        if self.depth_progress.exploration_phase == PHASE_NEW_NULLARIES {
            let nullaries = Frontier::load(&self.root, "chains", "nullaries", depth)?;
            let resume_uf = self.depth_progress.unary_file_index;
            let resume_ni = self.depth_progress.nullary_index as usize;
            let resume_ui = self.depth_progress.unary_index as usize;

            for uf in resume_uf..=depth {
                let unaries = Frontier::load(&self.root, "chains", "unaries", uf)?;
                let first_uf = uf == resume_uf;
                let ni_start = if first_uf { resume_ni } else { 0 };

                for ni in ni_start..nullaries.len() {
                    let ui_start = if first_uf && ni == ni_start { resume_ui } else { 0 };

                    for ui in ui_start..unaries.len() {
                        if self.budget_exhausted() {
                            let cursor = Progress {
                                depth,
                                nullary_index: ni as u64,
                                unary_index: ui as u64,
                                exploration_phase: PHASE_NEW_NULLARIES.to_string(),
                                nullary_file_index: depth,
                                unary_file_index: uf,
                            };
                            self.commit(Cursor::Depth(cursor))?;
                            return Ok(RunOutcome::BudgetExhausted);
                        }

                        self.process_pair(&unaries[ui], &nullaries[ni]);

                        let cursor = Progress {
                            depth,
                            nullary_index: ni as u64,
                            unary_index: ui as u64 + 1,
                            exploration_phase: PHASE_NEW_NULLARIES.to_string(),
                            nullary_file_index: depth,
                            unary_file_index: uf,
                        };
                        self.maybe_commit(Cursor::Depth(cursor))?;
                    }
                }
            }

            let cursor = Progress {
                depth,
                nullary_index: 0,
                unary_index: 0,
                exploration_phase: PHASE_OLD_NULLARIES.to_string(),
                nullary_file_index: 0,
                unary_file_index: depth,
            };
            self.commit(Cursor::Depth(cursor))?;
        }

        // Phase 2: new unaries (this depth's frontier) x old nullary frontiers.
        if self.depth_progress.exploration_phase == PHASE_OLD_NULLARIES {
            let unaries = Frontier::load(&self.root, "chains", "unaries", depth)?;
            let resume_nf = self.depth_progress.nullary_file_index;
            let resume_ui = self.depth_progress.unary_index as usize;
            let resume_ni = self.depth_progress.nullary_index as usize;

            let mut nf = resume_nf;
            while nf < depth {
                let nullaries = Frontier::load(&self.root, "chains", "nullaries", nf)?;
                let first_nf = nf == resume_nf;
                let ui_start = if first_nf { resume_ui } else { 0 };

                for ui in ui_start..unaries.len() {
                    let ni_start = if first_nf && ui == ui_start { resume_ni } else { 0 };

                    for ni in ni_start..nullaries.len() {
                        if self.budget_exhausted() {
                            let cursor = Progress {
                                depth,
                                nullary_index: ni as u64,
                                unary_index: ui as u64,
                                exploration_phase: PHASE_OLD_NULLARIES.to_string(),
                                nullary_file_index: nf,
                                unary_file_index: depth,
                            };
                            self.commit(Cursor::Depth(cursor))?;
                            return Ok(RunOutcome::BudgetExhausted);
                        }

                        self.process_pair(&unaries[ui], &nullaries[ni]);

                        let cursor = Progress {
                            depth,
                            nullary_index: ni as u64 + 1,
                            unary_index: ui as u64,
                            exploration_phase: PHASE_OLD_NULLARIES.to_string(),
                            nullary_file_index: nf,
                            unary_file_index: depth,
                        };
                        self.maybe_commit(Cursor::Depth(cursor))?;
                    }
                }

                nf += 1;
            }

            self.commit(Cursor::Depth(Progress::start_of_depth(depth + 1)))?;
        }

        Ok(RunOutcome::Completed)
    }

    // One (unary, nullary) pair. The frontier gate is depth_seen (not the
    // value store): a form whose value the length sweep already found must
    // still join a depth frontier the first time depth mode produces it.
    fn process_pair(&mut self, unary: &str, nullary: &str) {
        self.stats.items += 1;

        let candidate = unary.replacen('*', nullary, 1);
        if self.failed.contains(&candidate) || self.step_limited.contains(&candidate) {
            self.stats.dedup_hits += 1;
        } else if self.nullary_value_known(&candidate) {
            self.stats.dedup_hits += 1;
            self.mark_depth_frontier_nullary(candidate);
        } else {
            match self.evaluate(&candidate) {
                Ok(value) => {
                    self.min_index.offer(value, &candidate);
                    self.pending_evaluations.insert(candidate.clone(), value);
                    self.stats.new_evaluations += 1;
                    self.mark_depth_frontier_nullary(candidate);
                }
                Err(error) => self.record_failure(candidate, error),
            }
        }

        let frozen = unary.replacen('*', "#", 1);
        for candidate_unary in [
            format!("R({},{},*)", frozen, nullary),
            format!("R({},*,{})", frozen, nullary),
            format!("^({})", unary),
        ] {
            if self.depth_seen_unaries.contains(&candidate_unary)
                || self.pending_seen_unaries.contains(&candidate_unary)
            {
                self.stats.dedup_hits += 1;
                continue;
            }
            if !self.unaries.contains(&candidate_unary)
                && !self.pending_unaries.contains(&candidate_unary)
            {
                self.pending_unaries.insert(candidate_unary.clone());
                self.stats.new_unaries += 1;
            }
            self.pending_frontier_unaries.insert(candidate_unary.clone());
            self.pending_seen_unaries.insert(candidate_unary);
        }
    }

    fn mark_depth_frontier_nullary(&mut self, candidate: String) {
        if !self.depth_seen_nullaries.contains(&candidate)
            && !self.pending_seen_nullaries.contains(&candidate)
        {
            self.pending_frontier_nullaries.insert(candidate.clone());
            self.pending_seen_nullaries.insert(candidate);
        }
    }

    // -----------------------------------------------------------------------
    // Shared machinery
    // -----------------------------------------------------------------------

    fn evaluate(&mut self, form: &str) -> Result<u64, EvalError> {
        self.stats.evaluations += 1;
        let lookup = TieredLookup {
            store: &self.evaluations,
            pending: &self.pending_evaluations,
        };
        Evaluator::new(&lookup, &mut self.memo)
            .with_step_limit(self.config.step_limit)
            .evaluate_nullary(form)
    }

    fn record_failure(&mut self, form: String, error: EvalError) {
        match error {
            EvalError::StepLimit(_) => {
                self.step_limited.insert(form);
                self.step_limited_dirty = true;
                self.stats.step_limited += 1;
            }
            _ => {
                self.failed.insert(form);
                self.stats.overflows += 1;
            }
        }
    }

    pub fn step_limited_count(&self) -> usize {
        self.step_limited.len()
    }

    // Re-attempt every step-limited form under the current step limit (pass a
    // larger --step-limit to give them more budget). Resolved forms join the
    // corpus and their stratum; if any resolved form is at or below the
    // exhaustiveness watermark, the length cursor rolls back so the sweep can
    // regenerate the superterms that were skipped while the form was missing —
    // the dedup gate makes that replay cheap.
    pub fn retry_step_limited(&mut self) -> Result<RetryReport, String> {
        let forms: Vec<String> = self.step_limited.iter().cloned().collect();
        let mut report = RetryReport::default();
        let mut resolved_by_length: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();

        for form in forms {
            match self.evaluate(&form) {
                Ok(value) => {
                    self.min_index.offer(value, &form);
                    self.pending_evaluations.insert(form.clone(), value);
                    resolved_by_length
                        .entry(form.len() as u64)
                        .or_default()
                        .insert(form.clone());
                    self.step_limited.remove(&form);
                    self.step_limited_dirty = true;
                    report.resolved += 1;
                }
                Err(EvalError::StepLimit(_)) => report.still_limited += 1,
                Err(_) => {
                    // Permanent after all: with more budget it reached overflow.
                    self.step_limited.remove(&form);
                    self.failed.insert(form);
                    self.step_limited_dirty = true;
                    report.overflowed += 1;
                }
            }
        }

        for (length, batch) in &resolved_by_length {
            Frontier::append(&self.root, "strata", "nullaries", *length, batch)?;
            self.strata_cache.remove(&("nullaries", *length));
        }
        self.evaluations.commit(&self.pending_evaluations)?;
        self.pending_evaluations.clear();
        self.min_index.save_if_dirty()?;
        if self.step_limited_dirty {
            save_step_limited(&self.root, &self.step_limited)?;
            self.step_limited_dirty = false;
        }

        // Roll the sweep back to just after the smallest resolved length so
        // strata built without this form get its superterms on the next run.
        if let Some((&smallest, _)) = resolved_by_length.iter().next() {
            if self.length_progress.complete_through_length >= smallest {
                let rolled_back = LengthProgress {
                    length: smallest + 1,
                    item: 0,
                    complete_through_length: smallest,
                };
                rolled_back.save(&self.root)?;
                self.length_progress = rolled_back;
                report.rolled_back_to = Some(smallest);
            }
        }
        write_manifest(
            &self.root,
            &self.evaluations,
            &self.unaries,
            self.length_progress.complete_through_length,
            self.step_limited.len(),
        )?;

        Ok(report)
    }

    fn nullary_value_known(&self, form: &str) -> bool {
        self.memo.contains_key(form)
            || self.pending_evaluations.contains_key(form)
            || self.evaluations.get(form).is_some()
    }

    fn budget_exhausted(&self) -> bool {
        match self.items_budget {
            Some(budget) => self.stats.items >= budget,
            None => false,
        }
    }

    fn maybe_commit(&mut self, cursor: Cursor) -> Result<(), String> {
        let pending = self.pending_evaluations.len() + self.pending_unaries.len();
        if pending >= self.config.batch_keys
            || self.last_commit.elapsed().as_secs() >= self.config.batch_secs
        {
            self.commit(cursor)?;
        }
        Ok(())
    }

    // Durable commit, in crash-safe order:
    //   1. frontier appends (idempotent union — a replayed append is a no-op)
    //   2. store commits (atomic layer swaps)
    //   3. min-index + manifest
    //   4. the mode's cursor (atomic) — only now is the batch "spent"
    // A crash between any two steps loses at most one batch of *work*, never
    // consistency: replay after restart re-derives the same candidates and the
    // dedup gate / union-merges absorb anything already on disk.
    fn commit(&mut self, cursor: Cursor) -> Result<(), String> {
        let started = Instant::now();
        let new_evaluations = self.pending_evaluations.len();
        let new_unaries = self.pending_unaries.len();

        match &cursor {
            Cursor::Depth(progress) => {
                let target_depth = progress.depth
                    + if progress.exploration_phase == PHASE_READY {
                        0
                    } else {
                        1
                    };
                Frontier::append(
                    &self.root,
                    "chains",
                    "nullaries",
                    target_depth,
                    &self.pending_frontier_nullaries,
                )?;
                Frontier::append(
                    &self.root,
                    "chains",
                    "unaries",
                    target_depth,
                    &self.pending_frontier_unaries,
                )?;
                self.depth_seen_nullaries
                    .commit(&self.pending_seen_nullaries)?;
                self.depth_seen_unaries.commit(&self.pending_seen_unaries)?;
            }
            Cursor::Length(progress) => {
                // Mid-stratum commits target the stratum in progress; the
                // stratum-complete commit carries length+1, so target the one
                // just finished.
                let target_length = if progress.item == 0 && progress.length > 1 {
                    progress.length - 1
                } else {
                    progress.length
                };
                Frontier::append(
                    &self.root,
                    "strata",
                    "nullaries",
                    target_length,
                    &self.pending_frontier_nullaries,
                )?;
                Frontier::append(
                    &self.root,
                    "strata",
                    "unaries",
                    target_length,
                    &self.pending_frontier_unaries,
                )?;
                // Newly written strata invalidate any cached copy.
                self.strata_cache.remove(&("nullaries", target_length));
                self.strata_cache.remove(&("unaries", target_length));
            }
        }

        self.evaluations.commit(&self.pending_evaluations)?;
        self.unaries.commit(&self.pending_unaries)?;

        let compacted = self.evaluations.maybe_compact(self.config.compact_bytes)?
            | self.unaries.maybe_compact(self.config.compact_bytes)?;
        if compacted {
            self.stats.compactions += 1;
        }

        self.min_index.save_if_dirty()?;
        if self.step_limited_dirty {
            save_step_limited(&self.root, &self.step_limited)?;
            self.step_limited_dirty = false;
        }
        write_manifest(
            &self.root,
            &self.evaluations,
            &self.unaries,
            self.length_progress.complete_through_length.max(
                if let Cursor::Length(p) = &cursor {
                    p.complete_through_length
                } else {
                    0
                },
            ),
            self.step_limited.len(),
        )?;

        let label = match cursor {
            Cursor::Depth(progress) => {
                progress.save(&self.root)?;
                let label = format!(
                    "depth={} phase={}",
                    progress.depth, progress.exploration_phase
                );
                self.depth_progress = progress;
                label
            }
            Cursor::Length(progress) => {
                progress.save(&self.root)?;
                let label = format!(
                    "stratum={} item={} exhaustive<={}",
                    progress.length, progress.item, progress.complete_through_length
                );
                self.length_progress = progress;
                label
            }
        };

        self.pending_evaluations.clear();
        self.pending_unaries.clear();
        self.pending_frontier_nullaries.clear();
        self.pending_frontier_unaries.clear();
        self.pending_seen_nullaries.clear();
        self.pending_seen_unaries.clear();
        self.stats.commits += 1;
        self.last_commit = Instant::now();

        println!(
            "[commit] {} items={} evals={} dedup={} overflow={} limited={} +evals={} +unaries={} totals: evals={} unaries={} values={} ({} ms{})",
            label,
            self.stats.items,
            self.stats.evaluations,
            self.stats.dedup_hits,
            self.stats.overflows,
            self.stats.step_limited,
            new_evaluations,
            new_unaries,
            self.evaluations.len(),
            self.unaries.len(),
            self.min_index.len(),
            started.elapsed().as_millis(),
            if compacted { ", compacted" } else { "" },
        );

        Ok(())
    }
}

#[derive(Default)]
pub struct RetryReport {
    pub resolved: u64,
    pub still_limited: u64,
    pub overflowed: u64,
    pub rolled_back_to: Option<u64>,
}
