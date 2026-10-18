//! Deterministic source accounting, not evidence of a completed migration.
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const UPSTREAM_REVISION: &str = "afeb9c4941543d97f3e04fecf257b8f598f49a6a";

type Result<T> = std::result::Result<T, String>;

struct Entry {
    path: Vec<u8>,
    kind: String,
    oid: String,
    size: String,
}

#[derive(Default)]
struct Inventory {
    sources: Vec<String>,
    hardware: Vec<String>,
    waf: Vec<String>,
    dependencies: Vec<String>,
    languages: BTreeMap<String, usize>,
    root_entries: usize,
    repositories: usize,
    missing: usize,
    board_dirs: BTreeSet<String>,
    libraries: BTreeSet<String>,
    visited: BTreeSet<PathBuf>,
}

fn git(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(["--no-optional-locks", "-c", "core.fsmonitor=false", "-C"])
        .arg(repo)
        .args(args)
        // Inventory uses repository configuration plus explicit command-scope
        // environment values. Host-wide filters (for example Git LFS) and
        // attributes must not execute or change the inspected source behavior.
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .map_err(|error| format!("cannot run Git: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git {:?} in {} failed: {}",
            args,
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

fn reject_executable_filters(repo: &Path) -> Result<()> {
    // Inspect names separately: config subsection names and command values can
    // contain newlines, so `config --list` key/value text is ambiguous. This
    // query includes repository-local and explicit command-scope config.
    // The Git wrapper disables inherited system/global configuration.
    let names = git(repo, &["config", "--null", "--name-only", "--list"])?;
    for key in names.split(|byte| *byte == 0) {
        if key.starts_with(b"filter.") && (key.ends_with(b".clean") || key.ends_with(b".process")) {
            let name = std::str::from_utf8(key)
                .map_err(|_| "configured Git filters with non-UTF8 keys are unsupported")?;
            let values = git(repo, &["config", "--null", "--get-all", name])?;
            if values
                .split(|byte| *byte == 0)
                .any(|value| !value.is_empty())
            {
                return Err(format!("configured Git filters could execute code in {}; remove clean/process commands before inventory", repo.display()));
            }
        }
    }
    Ok(())
}

fn check(repo: &Path, expected: &str) -> Result<()> {
    reject_executable_filters(repo)?;
    let actual = git(repo, &["rev-parse", "--verify", "HEAD"])?;
    let actual = String::from_utf8_lossy(&actual);
    if actual.trim() != expected {
        return Err(format!(
            "revision mismatch in {}: expected {expected}, found {}",
            repo.display(),
            actual.trim()
        ));
    }
    let index = git(repo, &["ls-files", "-v", "-z"])?;
    if index
        .split(|byte| *byte == 0)
        .filter_map(|entry| entry.first())
        .any(|flag| flag.is_ascii_lowercase() || *flag == b'S')
    {
        return Err(format!(
            "index flags may hide tracked changes in {} (assume-unchanged or skip-worktree)",
            repo.display()
        ));
    }
    // Parent status intentionally ignores submodule worktrees so absent
    // checkouts can be reported. Compare the index separately: ignoring all
    // submodules would otherwise also hide staged gitlink pin changes.
    let staged = git(
        repo,
        &[
            "diff-index",
            "--cached",
            "--no-ext-diff",
            "--name-only",
            "-z",
            "--ignore-submodules=none",
            "HEAD",
            "--",
        ],
    )?;
    if !staged.is_empty() {
        return Err(format!(
            "tracked changes in {} (index differs from HEAD)",
            repo.display()
        ));
    }
    let dirty = git(
        repo,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=no",
            "--ignore-submodules=all",
        ],
    )?;
    if !dirty.is_empty() {
        return Err(format!("tracked changes in {}", repo.display()));
    }
    Ok(())
}

// Escape backslashes first and every byte that could break a TSV record. Non-UTF8
// names remain reversible as \xNN; output is ASCII even for non-ASCII names.
fn escape(bytes: &[u8]) -> String {
    let mut result = String::new();
    for byte in bytes {
        match *byte {
            b'\\' => result.push_str("\\\\"),
            b'\t' => result.push_str("\\t"),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            0x20..=0x7e => result.push(char::from(*byte)),
            other => result.push_str(&format!("\\x{other:02x}")),
        }
    }
    result
}

fn parse_tree(tree: &[u8]) -> Result<Vec<Entry>> {
    tree.split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let tab = record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or("invalid Git tree record")?;
            let metadata =
                std::str::from_utf8(&record[..tab]).map_err(|_| "invalid Git tree metadata")?;
            let fields: Vec<_> = metadata.split_whitespace().collect();
            if fields.len() != 4 || !matches!(fields[1], "blob" | "commit") {
                return Err(format!("unsupported Git tree metadata: {metadata}"));
            }
            Ok(Entry {
                path: record[tab + 1..].to_vec(),
                kind: if fields[1] == "commit" {
                    "gitlink"
                } else {
                    "blob"
                }
                .into(),
                oid: fields[2].into(),
                size: fields[3].into(),
            })
        })
        .collect()
}

fn native_path(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(bytes.to_vec())))
    }
    #[cfg(not(unix))]
    {
        Ok(PathBuf::from(OsString::from(
            std::str::from_utf8(bytes)
                .map_err(|_| "non-UTF8 Git path is unsupported on this host")?,
        )))
    }
}

fn component(path: &str) -> String {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or("");
    if first == "libraries" {
        format!("libraries/{}", parts.next().unwrap_or(""))
    } else if first == "Tools" {
        format!("tools/{}", parts.next().unwrap_or(""))
    } else {
        first.to_owned()
    }
}

fn language(path: &str, kind: &str) -> String {
    if kind == "gitlink" {
        return "gitlink".into();
    }
    let filename = path.rsplit('/').next().unwrap_or(path);
    // Every blob is accounted for, including extensionless scripts, binary data,
    // unknown language extensions and build/configuration input.
    filename.rsplit_once('.').map_or_else(
        || "extensionless".into(),
        |(_, extension)| {
            if extension.is_empty() {
                "extensionless".into()
            } else {
                extension.to_ascii_lowercase()
            }
        },
    )
}

fn same_checkout_root(path: &Path) -> bool {
    git(path, &["rev-parse", "--show-prefix"]).is_ok_and(|prefix| prefix == b"\n")
        && git(path, &["rev-parse", "--is-inside-work-tree"])
            .is_ok_and(|answer| answer == b"true\n")
}

impl Inventory {
    fn visit(&mut self, repo: &Path, label: &str, revision: &str) -> Result<()> {
        if !same_checkout_root(repo) {
            return Err(format!("{} is not a Git checkout root", repo.display()));
        }
        let canonical = repo.canonicalize().map_err(|error| error.to_string())?;
        if !self.visited.insert(canonical) {
            return Err(format!("repeated submodule checkout: {}", repo.display()));
        }
        check(repo, revision)?;
        let entries = parse_tree(&git(
            repo,
            &["ls-tree", "-r", "-z", "-l", "--full-tree", "HEAD"],
        )?)?;
        self.repositories += 1;
        if label == "." {
            self.root_entries = entries.len();
        }
        for entry in entries {
            let path = escape(&entry.path);
            let lang = language(&path, &entry.kind);
            *self.languages.entry(lang.clone()).or_default() += 1;
            self.sources.push(format!(
                "{label}\t{revision}\t{path}\t{}\t{}\t{}\t{}\t{lang}\tnot_ported",
                entry.oid,
                entry.kind,
                entry.size,
                component(&path)
            ));
            if label == "." && path.starts_with("libraries/") {
                if let Some(directory) = path.split('/').nth(1) {
                    self.libraries.insert(directory.to_owned());
                }
            }
            if entry.kind == "gitlink" {
                let child = repo.join(native_path(&entry.path)?);
                let child_label = if label == "." {
                    path.clone()
                } else {
                    format!("{label}/{path}")
                };
                // An absent submodule directory may still resolve to its parent
                // checkout with `git -C`, so verify it is its own root.
                if !child.is_dir() || !same_checkout_root(&child) {
                    self.missing += 1;
                    self.dependencies.push(format!("{label}\t{revision}\t{path}\t{}\tmissing\tuninitialized_or_absent_checkout", entry.oid));
                } else {
                    self.visit(&child, &child_label, &entry.oid)?;
                    self.dependencies.push(format!(
                        "{label}\t{revision}\t{path}\t{}\tinventoried\tpinned_checkout_validated",
                        entry.oid
                    ));
                }
            } else {
                let filename = path.rsplit('/').next().unwrap_or(&path);
                if filename == "hwdef.dat" || filename == "hwdef-bl.dat" {
                    let directory = path.rsplit_once('/').map_or("", |(dir, _)| dir);
                    if label == "." {
                        self.board_dirs.insert(directory.to_owned());
                    }
                    self.hardware.push(format!("{label}\t{revision}\t{directory}\t{path}\t{}\tunsupported\tdefinition_only", entry.oid));
                }
                if path == "Tools/ardupilotwaf/boards.py" {
                    let content = git(repo, &["cat-file", "blob", &entry.oid])?;
                    for (index, line) in String::from_utf8_lossy(&content).lines().enumerate() {
                        // Static syntactic evidence only. Waf also dynamically
                        // registers boards; executing it is deliberately excluded.
                        if let Some(rest) = line.strip_prefix("class ") {
                            let name = rest.split(['(', ':', ' ']).next().unwrap_or("");
                            if !name.is_empty() {
                                self.waf.push(format!("{label}\t{revision}\t{path}\t{}\t{}\t{}\tunsupported\tstatic_class_declaration", entry.oid, index + 1, escape(name.as_bytes())));
                            }
                        }
                    }
                }
            }
        }
        check(repo, revision)?;
        Ok(())
    }
}

fn write_table(output: &Path, name: &str, header: &str, rows: &mut [String]) -> Result<()> {
    rows.sort();
    let mut text = format!("{header}\n");
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    fs::write(output.join(name), text).map_err(|error| error.to_string())
}

fn resolved_destination(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut resolved = PathBuf::new();
    for part in absolute.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            part => {
                resolved.push(part.as_os_str());
                if resolved.exists() {
                    resolved = resolved.canonicalize().map_err(|error| error.to_string())?;
                }
            }
        }
    }
    Ok(resolved)
}

/// Inventory a clean pinned checkout and any available recursively pinned Git
/// submodules. The CLI always uses [`UPSTREAM_REVISION`]; the explicit revision
/// parameter permits independent local fixtures and future reviewed baselines.
/// Paths in TSV fields use reversible ASCII escaping. No upstream code runs.
/// Untracked files are excluded. All rows remain `not_ported` or `unsupported`.
pub fn inventory(source: &Path, output: &Path, expected_revision: &str) -> Result<String> {
    if expected_revision.len() != 40
        || !expected_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("expected revision must be a full 40-character Git SHA-1".into());
    }
    let destination = resolved_destination(output)?;
    let source_root = source.canonicalize().map_err(|error| error.to_string())?;
    if destination.starts_with(&source_root) {
        return Err("output directory must be outside the reference checkout".into());
    }
    for name in [
        "sources.tsv",
        "hardware.tsv",
        "waf-classes.tsv",
        "dependencies.tsv",
        "languages.tsv",
        "summary.txt",
    ] {
        if fs::symlink_metadata(output.join(name))
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!("refusing symlink output file: {name}"));
        }
    }
    let mut data = Inventory::default();
    data.visit(source, ".", expected_revision)?;
    data.dependencies.push(".\t-\tEXTERNAL_CLOSURE\t-\tunresolved\texternally_fetched_SDKs_packages_toolchains_and_vendor_blobs_not_fully_captured_by_Git".into());
    let summary = format!("RustPilot source inventory\nUpstream revision: {expected_revision}\nRoot entries: {}\nAll entries including recursive submodules: {}\nInventoried repositories: {}\nMissing submodule checkouts: {}\nRoot library directories: {}\nHardware definition directories: {}\nHardware definition files: {}\nStatic Waf class declarations: {}\nPorted units: 0\nSupported firmware targets: 0\nExternal SDK/package/toolchain closure: unresolved\nInventory counts describe source scope only. All hardware and Waf evidence is unsupported.\n", data.root_entries, data.sources.len(), data.repositories, data.missing, data.libraries.len(), data.board_dirs.len(), data.hardware.len(), data.waf.len());
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    write_table(output, "sources.tsv", "repository\trevision\tpath\tobject_id\tkind\tbytes\tcomponent\tlanguage_extension\tmigration_status", &mut data.sources)?;
    write_table(output, "hardware.tsv", "repository\trevision\tboard_directory\tdefinition_path\tobject_id\tsupport_status\tevidence", &mut data.hardware)?;
    write_table(
        output,
        "waf-classes.tsv",
        "repository\trevision\tpath\tobject_id\tline\tclass\tsupport_status\tevidence",
        &mut data.waf,
    )?;
    write_table(
        output,
        "dependencies.tsv",
        "repository\trevision\tpath\tpinned_revision\tstatus\tevidence",
        &mut data.dependencies,
    )?;
    let mut languages: Vec<_> = data
        .languages
        .iter()
        .map(|(lang, count)| format!("{lang}\t{count}"))
        .collect();
    write_table(
        output,
        "languages.tsv",
        "language_extension\tentries",
        &mut languages,
    )?;
    fs::write(output.join("summary.txt"), &summary).map_err(|error| error.to_string())?;
    Ok(summary)
}
