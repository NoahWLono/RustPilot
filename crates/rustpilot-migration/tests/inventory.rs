use rustpilot_migration::inventory;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rustpilot-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn repo(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        git(&path, &["init", "-q", "--initial-branch=main"]);
        git(&path, &["config", "commit.gpgsign", "false"]);
        git(&path, &["config", "core.hooksPath", "/dev/null"]);
        git(&path, &["config", "user.name", "Fixture"]);
        git(&path, &["config", "user.email", "fixture@example.invalid"]);
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}
fn put(repo: &Path, path: &str, text: &str) {
    let target = repo.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, text).unwrap();
}
fn commit(repo: &Path) -> String {
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", "fixture"]);
    git(repo, &["rev-parse", "HEAD"])
}
#[test]
fn inventories_every_blob_hardware_and_static_waf_evidence_deterministically() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "libraries/A/a.cpp", "int value;\n");
    put(&repo, "Tools/a.py", "print('fixture')\n");
    put(&repo, "Tools/a.lua", "return 1\n");
    put(
        &repo,
        "libraries/AP_HAL_ChibiOS/hwdef/A/hwdef.dat",
        "MCU fixture\n",
    );
    put(
        &repo,
        "libraries/AP_HAL_ChibiOS/hwdef/B/hwdef-bl.dat",
        "MCU fixture\n",
    );
    put(
        &repo,
        "Tools/ardupilotwaf/boards.py",
        "class fixture(Board):\n    pass\n",
    );
    put(&repo, "tab\tline\n.txt", "odd name");
    let rev = commit(&repo);
    let first = f.0.join("out1");
    let second = f.0.join("out2");
    let summary = inventory(&repo, &first, &rev).unwrap();
    assert!(summary.contains("Root entries: 7"));
    assert!(summary.contains("Hardware definition directories: 2"));
    assert!(summary.contains("Ported units: 0"));
    inventory(&repo, &second, &rev).unwrap();
    for name in [
        "sources.tsv",
        "hardware.tsv",
        "waf-classes.tsv",
        "dependencies.tsv",
        "summary.txt",
        "languages.tsv",
    ] {
        assert_eq!(
            fs::read(first.join(name)).unwrap(),
            fs::read(second.join(name)).unwrap(),
            "{name}"
        );
    }
    let sources = fs::read_to_string(first.join("sources.tsv")).unwrap();
    assert_eq!(sources.lines().count(), 8);
    assert!(sources.contains("tab\\tline\\n.txt"));
    assert!(sources.contains("\tlua\tnot_ported"));
    let hardware = fs::read_to_string(first.join("hardware.tsv")).unwrap();
    assert!(hardware.contains("hwdef-bl.dat"));
    assert!(hardware.contains("unsupported"));
    assert!(fs::read_to_string(first.join("waf-classes.tsv"))
        .unwrap()
        .contains("\tfixture\t"));
}
#[test]
fn rejects_revision_mismatch_and_tracked_dirt_before_writing() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "source.c", "first");
    let rev = commit(&repo);
    let output = f.0.join("out");
    assert!(
        inventory(&repo, &output, "0000000000000000000000000000000000000000")
            .unwrap_err()
            .contains("revision mismatch")
    );
    put(&repo, "source.c", "changed");
    assert!(inventory(&repo, &output, &rev)
        .unwrap_err()
        .contains("tracked changes"));
    assert!(!output.exists());
}
#[test]
fn reports_missing_submodule_then_recurses_and_rejects_drift() {
    let f = Fixture::new();
    let repo = f.repo("root");
    let child = f.repo("child");
    put(&child, "dependency.rs", "// fixture\n");
    let child_rev = commit(&child);
    put(&repo, "root.txt", "fixture");
    git(
        &repo,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{child_rev},modules/dep"),
        ],
    );
    git(&repo, &["add", "root.txt"]);
    git(&repo, &["commit", "-qm", "fixture"]);
    let rev = git(&repo, &["rev-parse", "HEAD"]);
    let missing = f.0.join("missing");
    inventory(&repo, &missing, &rev).unwrap();
    assert!(fs::read_to_string(missing.join("dependencies.tsv"))
        .unwrap()
        .contains("missing"));
    fs::create_dir_all(repo.join("modules")).unwrap();
    let clone = Command::new("git")
        .args(["clone", "-q"])
        .arg(&child)
        .arg(repo.join("modules/dep"))
        .output()
        .unwrap();
    assert!(clone.status.success());
    let present = f.0.join("present");
    inventory(&repo, &present, &rev).unwrap();
    let sources = fs::read_to_string(present.join("sources.tsv")).unwrap();
    assert!(sources.contains("modules/dep\t"));
    assert!(sources.contains("dependency.rs"));
    put(&child, "second.rs", "// drift\n");
    commit(&child);
    git(&repo.join("modules/dep"), &["fetch", "-q", "origin"]);
    git(
        &repo.join("modules/dep"),
        &["checkout", "-q", "origin/main"],
    );
    assert!(inventory(&repo, &f.0.join("drift"), &rev)
        .unwrap_err()
        .contains("revision mismatch"));
}
#[test]
fn forbids_output_inside_the_reference_checkout() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "source.c", "fixture");
    let rev = commit(&repo);
    assert!(inventory(&repo, &repo.join("generated/inventory"), &rev)
        .unwrap_err()
        .contains("outside"));
}
#[test]
fn cli_requires_the_reviewed_upstream_pin() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "source.c", "fixture");
    commit(&repo);
    let result = Command::new(env!("CARGO_BIN_EXE_rustpilot-migration"))
        .arg("inventory")
        .arg(repo)
        .arg(f.0.join("out"))
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains(rustpilot_migration::UPSTREAM_REVISION)
    );
}
#[test]
fn recursively_pins_nested_submodules_and_rejects_child_tracked_changes() {
    let f = Fixture::new();
    let root = f.repo("root");
    let child = f.repo("child");
    let leaf = f.repo("leaf");
    put(&leaf, "leaf.sh", "#!/bin/sh\n");
    let leaf_rev = commit(&leaf);
    put(&child, "child.py", "# fixture\n");
    git(&child, &["add", "."]);
    git(
        &child,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{leaf_rev},nested"),
        ],
    );
    git(&child, &["commit", "-qm", "child"]);
    let child_rev = git(&child, &["rev-parse", "HEAD"]);
    put(&root, "root.c", "// fixture\n");
    git(&root, &["add", "."]);
    git(
        &root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{child_rev},dependency"),
        ],
    );
    git(&root, &["commit", "-qm", "root"]);
    let rev = git(&root, &["rev-parse", "HEAD"]);
    for (from, to) in [
        (&child, root.join("dependency")),
        (&leaf, root.join("dependency/nested")),
    ] {
        let result = Command::new("git")
            .args(["clone", "-q"])
            .arg(from)
            .arg(to)
            .output()
            .unwrap();
        assert!(result.status.success());
    }
    let out = f.0.join("out");
    let summary = inventory(&root, &out, &rev).unwrap();
    assert!(summary.contains("Inventoried repositories: 3"));
    assert!(fs::read_to_string(out.join("sources.tsv"))
        .unwrap()
        .contains("dependency/nested\t"));
    put(&root.join("dependency/nested"), "leaf.sh", "dirty");
    assert!(inventory(&root, &f.0.join("dirty"), &rev)
        .unwrap_err()
        .contains("tracked changes"));
}
#[test]
fn rejects_index_flags_that_can_hide_tracked_dirt() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "source.c", "fixture");
    let rev = commit(&repo);
    git(&repo, &["update-index", "--assume-unchanged", "source.c"]);
    put(&repo, "source.c", "hidden tracked changes");
    assert!(inventory(&repo, &f.0.join("out"), &rev)
        .unwrap_err()
        .contains("index flags"));
}
#[test]
fn rejects_staged_gitlink_change_even_when_child_matches_head_pin() {
    let f = Fixture::new();
    let root = f.repo("root");
    let child = f.repo("child");
    put(&child, "child.rs", "// original\n");
    let old_pin = commit(&child);
    put(&root, "root.rs", "// fixture\n");
    git(&root, &["add", "."]);
    git(
        &root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{old_pin},dependency"),
        ],
    );
    git(&root, &["commit", "-qm", "root"]);
    let root_pin = git(&root, &["rev-parse", "HEAD"]);
    let result = Command::new("git")
        .args(["clone", "-q"])
        .arg(&child)
        .arg(root.join("dependency"))
        .output()
        .unwrap();
    assert!(result.status.success());
    put(&child, "child.rs", "// changed\n");
    let new_pin = commit(&child);
    git(
        &root,
        &[
            "update-index",
            "--cacheinfo",
            &format!("160000,{new_pin},dependency"),
        ],
    );
    assert_eq!(
        git(&root.join("dependency"), &["rev-parse", "HEAD"]),
        old_pin
    );
    assert!(!git(
        &root,
        &["status", "--porcelain", "--ignore-submodules=none"]
    )
    .is_empty());
    let output = f.0.join("out");
    assert!(inventory(&root, &output, &root_pin)
        .unwrap_err()
        .contains("tracked changes"));
    assert!(!output.exists());
}
#[test]
fn configured_clean_and_process_filters_are_rejected_without_execution() {
    for filter_command in ["clean", "process"] {
        let f = Fixture::new();
        let repo = f.repo("root");
        put(&repo, ".gitattributes", "sample.dat filter=review\n");
        put(&repo, "sample.dat", "original\n");
        put(&repo, "filter.sh", "touch FILTER_EXECUTED\ncat\n");
        let revision = commit(&repo);
        git(
            &repo,
            &[
                "config",
                &format!("filter.review.{filter_command}"),
                "sh filter.sh",
            ],
        );
        put(&repo, "sample.dat", "changed!\n");
        let result = inventory(&repo, &f.0.join("out"), &revision);
        assert!(
            !repo.join("FILTER_EXECUTED").exists(),
            "{filter_command} command executed"
        );
        assert!(result.unwrap_err().contains("configured Git filters"));
    }
}
#[test]
fn configured_fsmonitor_is_disabled_and_replacement_objects_are_ignored() {
    let f = Fixture::new();
    let repo = f.repo("root");
    put(&repo, "original.rs", "// pinned original\n");
    put(
        &repo,
        "monitor.sh",
        "touch MONITOR_EXECUTED\nprintf '\\0'\n",
    );
    let original = commit(&repo);
    put(&repo, "replacement-only.rs", "// outside pin\n");
    let replacement = commit(&repo);
    git(&repo, &["checkout", "-q", &original]);
    git(&repo, &["replace", &original, &replacement]);
    git(&repo, &["config", "core.fsmonitor", "sh monitor.sh"]);
    let out = f.0.join("out");
    inventory(&repo, &out, &original).unwrap();
    assert!(!repo.join("MONITOR_EXECUTED").exists());
    let sources = fs::read_to_string(out.join("sources.tsv")).unwrap();
    assert!(sources.contains("original.rs"));
    assert!(!sources.contains("replacement-only.rs"));
}
#[test]
fn rejects_staged_gitlink_change_with_missing_checkout() {
    let f = Fixture::new();
    let root = f.repo("root");
    let child = f.repo("child");
    put(&child, "child.rs", "// original\n");
    let old_pin = commit(&child);
    put(&root, "root.rs", "// fixture\n");
    git(&root, &["add", "."]);
    git(
        &root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{old_pin},dependency"),
        ],
    );
    git(&root, &["commit", "-qm", "root"]);
    let root_pin = git(&root, &["rev-parse", "HEAD"]);
    put(&child, "child.rs", "// changed\n");
    let new_pin = commit(&child);
    git(
        &root,
        &[
            "update-index",
            "--cacheinfo",
            &format!("160000,{new_pin},dependency"),
        ],
    );
    let output = f.0.join("out");
    assert!(inventory(&root, &output, &root_pin)
        .unwrap_err()
        .contains("tracked changes"));
    assert!(!output.exists());
}
