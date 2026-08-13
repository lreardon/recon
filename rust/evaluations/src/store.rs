use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use fst::{Map, MapBuilder, Set, SetBuilder, Streamer};

// Two-layer (LSM-style) persistent stores over immutable FSTs.
//
// Layout under $PROJECT_ROOT/fst/:
//   evaluations.fst          base layer  (map: nullary -> value)
//   evaluations.recent.fst   recent layer (absent = empty)
//   unaries.fst              base layer  (set: unary strings)
//   unaries.recent.fst       recent layer (absent = empty)
//   MANIFEST.json            informational description of the live layers
//
// Commits union the pending batch into the small recent layer (cost bounded by
// the recent layer, not the corpus) via write-to-temp + atomic rename. When the
// recent layer outgrows a threshold it is compacted into the base once. A crash
// at any point leaves the previous layer files intact.

fn write_atomically<F>(path: &Path, build: F) -> Result<(), String>
where
    F: FnOnce(&mut BufWriter<fs::File>) -> Result<(), String>,
{
    let tmp_path = path.with_extension("tmp");
    let file = fs::File::create(&tmp_path)
        .map_err(|e| format!("creating {}: {}", tmp_path.display(), e))?;
    let mut writer = BufWriter::new(file);
    build(&mut writer)?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("renaming {} into place: {}", tmp_path.display(), e))?;
    Ok(())
}

fn read_map(path: &Path) -> Result<Map<Vec<u8>>, String> {
    let bytes = fs::read(path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    Map::new(bytes).map_err(|e| format!("parsing {}: {}", path.display(), e))
}

fn read_set(path: &Path) -> Result<Set<Vec<u8>>, String> {
    let bytes = fs::read(path).map_err(|e| format!("reading {}: {}", path.display(), e))?;
    Set::new(bytes).map_err(|e| format!("parsing {}: {}", path.display(), e))
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub struct MapStore {
    base_path: PathBuf,
    recent_path: PathBuf,
    base: Map<Vec<u8>>,
    recent: Option<Map<Vec<u8>>>,
}

impl MapStore {
    pub fn open_evaluations(project_root: &str) -> Result<Self, String> {
        let fst_dir = Path::new(project_root).join("fst");
        fs::create_dir_all(&fst_dir).map_err(|e| e.to_string())?;
        let base_path = fst_dir.join("evaluations.fst");
        let recent_path = fst_dir.join("evaluations.recent.fst");

        if !base_path.exists() {
            // Same seed as initialize_evaluations_fst: the base nullary.
            write_atomically(&base_path, |writer| {
                let mut builder = MapBuilder::new(writer).map_err(|e| e.to_string())?;
                builder.insert("0", 0).map_err(|e| e.to_string())?;
                builder.finish().map_err(|e| e.to_string())
            })?;
        }

        let base = read_map(&base_path)?;
        let recent = if recent_path.exists() {
            Some(read_map(&recent_path)?)
        } else {
            None
        };

        Ok(MapStore {
            base_path,
            recent_path,
            base,
            recent,
        })
    }

    pub fn get(&self, key: &str) -> Option<u64> {
        if let Some(recent) = &self.recent {
            if let Some(value) = recent.get(key.as_bytes()) {
                return Some(value);
            }
        }
        self.base.get(key.as_bytes())
    }

    pub fn len(&self) -> usize {
        self.base.len() + self.recent.as_ref().map_or(0, |r| r.len())
    }

    pub fn recent_bytes(&self) -> u64 {
        file_len(&self.recent_path)
    }

    // Stream every (key, value) across both layers in key order; stop early
    // when the callback returns false.
    pub fn for_each_while<F>(&self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(&str, u64) -> bool,
    {
        match &self.recent {
            Some(recent) => {
                let mut union = self.base.op().add(recent).union();
                while let Some((key, values)) = union.next() {
                    let key = std::str::from_utf8(key).map_err(|e| e.to_string())?;
                    if !callback(key, values[0].value) {
                        break;
                    }
                }
            }
            None => {
                let mut stream = self.base.stream();
                while let Some((key, value)) = stream.next() {
                    let key = std::str::from_utf8(key).map_err(|e| e.to_string())?;
                    if !callback(key, value) {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    // Union the batch into the recent layer. Existing entries win on key
    // collision (values should be identical anyway).
    pub fn commit(&mut self, batch: &BTreeMap<String, u64>) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut delta_builder = MapBuilder::memory();
        for (key, value) in batch {
            delta_builder
                .insert(key, *value)
                .map_err(|e| e.to_string())?;
        }
        let delta = delta_builder.into_map();

        write_atomically(&self.recent_path, |writer| {
            let mut builder = MapBuilder::new(writer).map_err(|e| e.to_string())?;
            match &self.recent {
                Some(recent) => {
                    let mut union = recent.op().add(&delta).union();
                    while let Some((key, values)) = union.next() {
                        builder
                            .insert(key, values[0].value)
                            .map_err(|e| e.to_string())?;
                    }
                }
                None => {
                    let mut stream = delta.stream();
                    while let Some((key, value)) = stream.next() {
                        builder.insert(key, value).map_err(|e| e.to_string())?;
                    }
                }
            }
            builder.finish().map_err(|e| e.to_string())
        })?;

        self.recent = Some(read_map(&self.recent_path)?);
        Ok(())
    }

    // Fold the recent layer into the base once it has outgrown the threshold.
    pub fn maybe_compact(&mut self, threshold_bytes: u64) -> Result<bool, String> {
        let recent = match &self.recent {
            Some(recent) if self.recent_bytes() > threshold_bytes => recent,
            _ => return Ok(false),
        };

        write_atomically(&self.base_path, |writer| {
            let mut builder = MapBuilder::new(writer).map_err(|e| e.to_string())?;
            let mut union = self.base.op().add(recent).union();
            while let Some((key, values)) = union.next() {
                builder
                    .insert(key, values[0].value)
                    .map_err(|e| e.to_string())?;
            }
            builder.finish().map_err(|e| e.to_string())
        })?;

        fs::remove_file(&self.recent_path).map_err(|e| e.to_string())?;
        self.recent = None;
        self.base = read_map(&self.base_path)?;
        Ok(true)
    }
}

pub struct SetStore {
    base_path: PathBuf,
    recent_path: PathBuf,
    base: Set<Vec<u8>>,
    recent: Option<Set<Vec<u8>>>,
}

impl SetStore {
    pub fn open_unaries(project_root: &str) -> Result<Self, String> {
        Self::open_at(&Path::new(project_root).join("fst"), "unaries")
    }

    // A layered set store at <dir>/<name>.fst + <dir>/<name>.recent.fst.
    pub fn open_at(dir: &Path, name: &str) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let base_path = dir.join(format!("{}.fst", name));
        let recent_path = dir.join(format!("{}.recent.fst", name));

        if !base_path.exists() {
            write_atomically(&base_path, |writer| {
                let builder = SetBuilder::new(writer).map_err(|e| e.to_string())?;
                builder.finish().map_err(|e| e.to_string())
            })?;
        }

        let base = read_set(&base_path)?;
        let recent = if recent_path.exists() {
            Some(read_set(&recent_path)?)
        } else {
            None
        };

        Ok(SetStore {
            base_path,
            recent_path,
            base,
            recent,
        })
    }

    pub fn contains(&self, key: &str) -> bool {
        if let Some(recent) = &self.recent {
            if recent.contains(key.as_bytes()) {
                return true;
            }
        }
        self.base.contains(key.as_bytes())
    }

    pub fn len(&self) -> usize {
        self.base.len() + self.recent.as_ref().map_or(0, |r| r.len())
    }

    pub fn recent_bytes(&self) -> u64 {
        file_len(&self.recent_path)
    }

    pub fn commit(&mut self, batch: &BTreeSet<String>) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut delta_builder = SetBuilder::memory();
        for key in batch {
            delta_builder.insert(key).map_err(|e| e.to_string())?;
        }
        let delta = Set::new(delta_builder.into_inner().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        write_atomically(&self.recent_path, |writer| {
            let mut builder = SetBuilder::new(writer).map_err(|e| e.to_string())?;
            match &self.recent {
                Some(recent) => {
                    let mut union = recent.op().add(&delta).union();
                    while let Some(key) = union.next() {
                        builder.insert(key).map_err(|e| e.to_string())?;
                    }
                }
                None => {
                    let mut stream = delta.stream();
                    while let Some(key) = stream.next() {
                        builder.insert(key).map_err(|e| e.to_string())?;
                    }
                }
            }
            builder.finish().map_err(|e| e.to_string())
        })?;

        self.recent = Some(read_set(&self.recent_path)?);
        Ok(())
    }

    pub fn maybe_compact(&mut self, threshold_bytes: u64) -> Result<bool, String> {
        let recent = match &self.recent {
            Some(recent) if self.recent_bytes() > threshold_bytes => recent,
            _ => return Ok(false),
        };

        write_atomically(&self.base_path, |writer| {
            let mut builder = SetBuilder::new(writer).map_err(|e| e.to_string())?;
            let mut union = self.base.op().add(recent).union();
            while let Some(key) = union.next() {
                builder.insert(key).map_err(|e| e.to_string())?;
            }
            builder.finish().map_err(|e| e.to_string())
        })?;

        fs::remove_file(&self.recent_path).map_err(|e| e.to_string())?;
        self.recent = None;
        self.base = read_set(&self.base_path)?;
        Ok(true)
    }
}

// Per-depth exploration frontiers, stored as FST sets under
// state/chains/{nullaries,unaries}/<depth>.fst. A frontier file is only ever
// appended to (by atomic union-merge) while exploring the previous depth, and
// is immutable once its own depth begins — so enumeration order (lexicographic)
// is a stable, resumable coordinate system. Union-merging also makes appends
// idempotent: re-appending after a crash cannot create duplicates.
//
// Legacy plain-text frontiers (<depth>.txt) are migrated to .fst on first load.
pub struct Frontier;

impl Frontier {
    fn dir(project_root: &str, base: &str, kind: &str) -> PathBuf {
        Path::new(project_root).join("state").join(base).join(kind)
    }

    fn fst_path(project_root: &str, base: &str, kind: &str, index: u64) -> PathBuf {
        Self::dir(project_root, base, kind).join(format!("{}.fst", index))
    }

    fn txt_path(project_root: &str, base: &str, kind: &str, index: u64) -> PathBuf {
        Self::dir(project_root, base, kind).join(format!("{}.txt", index))
    }

    // Indices of every frontier file present for this base/kind.
    pub fn indices(project_root: &str, base: &str, kind: &str) -> Vec<u64> {
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(Self::dir(project_root, base, kind)) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(stem) = name.strip_suffix(".fst") {
                    if let Ok(index) = stem.parse() {
                        found.push(index);
                    }
                }
            }
        }
        found.sort_unstable();
        found
    }

    // Seed the depth-0 frontiers if this is a fresh state directory.
    pub fn ensure_seeds(project_root: &str) -> Result<(), String> {
        for (kind, seed) in [("nullaries", "0"), ("unaries", "^(*)")] {
            let dir = Self::dir(project_root, "chains", kind);
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let fst_path = Self::fst_path(project_root, "chains", kind, 0);
            let txt_path = Self::txt_path(project_root, "chains", kind, 0);
            if !fst_path.exists() && !txt_path.exists() {
                let batch: BTreeSet<String> = [seed.to_string()].into();
                Self::append(project_root, "chains", kind, 0, &batch)?;
            }
        }
        Ok(())
    }

    pub fn load(
        project_root: &str,
        base: &str,
        kind: &str,
        index: u64,
    ) -> Result<Vec<String>, String> {
        let fst_path = Self::fst_path(project_root, base, kind, index);

        if !fst_path.exists() {
            let txt_path = Self::txt_path(project_root, base, kind, index);
            if txt_path.exists() {
                let contents = fs::read_to_string(&txt_path).map_err(|e| e.to_string())?;
                let items: BTreeSet<String> = contents
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect();
                Self::append(project_root, base, kind, index, &items)?;
            } else {
                return Ok(Vec::new());
            }
        }

        let set = read_set(&fst_path)?;
        let mut items = Vec::with_capacity(set.len());
        let mut stream = set.stream();
        while let Some(key) = stream.next() {
            items.push(String::from_utf8(key.to_vec()).map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    pub fn append(
        project_root: &str,
        base: &str,
        kind: &str,
        index: u64,
        batch: &BTreeSet<String>,
    ) -> Result<(), String> {
        if batch.is_empty() {
            return Ok(());
        }
        let dir = Self::dir(project_root, base, kind);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let fst_path = Self::fst_path(project_root, base, kind, index);

        let existing = if fst_path.exists() {
            Some(read_set(&fst_path)?)
        } else {
            None
        };

        let mut delta_builder = SetBuilder::memory();
        for key in batch {
            delta_builder.insert(key).map_err(|e| e.to_string())?;
        }
        let delta = Set::new(delta_builder.into_inner().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        write_atomically(&fst_path, |writer| {
            let mut builder = SetBuilder::new(writer).map_err(|e| e.to_string())?;
            match &existing {
                Some(existing) => {
                    let mut union = existing.op().add(&delta).union();
                    while let Some(key) = union.next() {
                        builder.insert(key).map_err(|e| e.to_string())?;
                    }
                }
                None => {
                    let mut stream = delta.stream();
                    while let Some(key) = stream.next() {
                        builder.insert(key).map_err(|e| e.to_string())?;
                    }
                }
            }
            builder.finish().map_err(|e| e.to_string())
        })
    }
}

// Inverted index: value -> shortest known expression evaluating to it.
// Persisted as a sorted TSV (value <TAB> expression) at fst/shortest_by_value.tsv,
// rewritten atomically at commit time when it changed. Ties on length break
// lexicographically so the index is deterministic.
pub struct MinIndex {
    path: PathBuf,
    entries: BTreeMap<u64, String>,
    dirty: bool,
}

impl MinIndex {
    pub fn open(project_root: &str, evaluations: &MapStore) -> Result<Self, String> {
        let path = Path::new(project_root).join("fst").join("shortest_by_value.tsv");
        let mut index = MinIndex {
            path,
            entries: BTreeMap::new(),
            dirty: false,
        };

        if index.path.exists() {
            let contents = fs::read_to_string(&index.path).map_err(|e| e.to_string())?;
            for line in contents.lines() {
                let (value, expression) = line
                    .split_once('\t')
                    .ok_or_else(|| format!("malformed index line: {}", line))?;
                let value: u64 = value.parse().map_err(|e| format!("{}: {}", line, e))?;
                index.entries.insert(value, expression.to_string());
            }
        } else {
            // Bootstrap from the full corpus so pre-existing entries are covered.
            evaluations.for_each_while(|key, value| {
                index.offer(value, key);
                true
            })?;
            index.save()?;
        }

        Ok(index)
    }

    pub fn offer(&mut self, value: u64, expression: &str) {
        match self.entries.get(&value) {
            Some(current)
                if (expression.len(), expression) >= (current.len(), current.as_str()) => {}
            _ => {
                self.entries.insert(value, expression.to_string());
                self.dirty = true;
            }
        }
    }

    pub fn get(&self, value: u64) -> Option<&str> {
        self.entries.get(&value).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn save_if_dirty(&mut self) -> Result<(), String> {
        if self.dirty {
            self.save()?;
        }
        Ok(())
    }

    fn save(&mut self) -> Result<(), String> {
        let mut rendered = String::new();
        for (value, expression) in &self.entries {
            rendered.push_str(&format!("{}\t{}\n", value, expression));
        }
        let tmp_path = self.path.with_extension("tsv.tmp");
        fs::write(&tmp_path, rendered).map_err(|e| e.to_string())?;
        fs::rename(&tmp_path, &self.path).map_err(|e| e.to_string())?;
        self.dirty = false;
        Ok(())
    }
}

pub fn write_manifest(
    project_root: &str,
    evaluations: &MapStore,
    unaries: &SetStore,
    exhaustive_through_length: u64,
    step_limited_forms: usize,
) -> Result<(), String> {
    let manifest = serde_json::json!({
        "exhaustive_through_length": exhaustive_through_length,
        "step_limited_forms": step_limited_forms,
        "evaluations": {
            "layers": ["evaluations.recent.fst", "evaluations.fst"],
            "entries": evaluations.len(),
            "base_bytes": file_len(&evaluations.base_path),
            "recent_bytes": evaluations.recent_bytes(),
        },
        "unaries": {
            "layers": ["unaries.recent.fst", "unaries.fst"],
            "entries": unaries.len(),
            "base_bytes": file_len(&unaries.base_path),
            "recent_bytes": unaries.recent_bytes(),
        },
    });
    let path = Path::new(project_root).join("fst").join("MANIFEST.json");
    let rendered = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, rendered).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    Ok(())
}
