// SPDX-License-Identifier: GPL-3.0-or-later
// Shared by separately compiled integration-test binaries.
#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

pub const PIN: &str = "afeb9c4941543d97f3e04fecf257b8f598f49a6a";

fn git(reference: &Path) -> Command {
    let mut cmd = Command::new("git");
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        cmd.env_remove(name);
    }
    cmd.env("GIT_NO_REPLACE_OBJECTS", "1")
        .arg("-C")
        .arg(reference);
    cmd
}

/// Verify the exact input bytes against the original Git object, then return
/// those bytes so compilation does not reread a mutable reference checkout.
pub fn verified_source(reference: &Path, revision: &str, path: &str) -> Result<Vec<u8>, String> {
    let object = git(reference)
        .args(["show", &format!("{revision}:{path}")])
        .output()
        .map_err(|error| format!("git show {path}: {error}"))?;
    if !object.status.success() {
        return Err(format!(
            "git show {path}: {}",
            String::from_utf8_lossy(&object.stderr)
        ));
    }
    let bytes = fs::read(reference.join(path)).map_err(|error| format!("read {path}: {error}"))?;
    if bytes != object.stdout {
        return Err(format!(
            "reference source differs from pinned object: {path}"
        ));
    }
    Ok(bytes)
}

pub struct Reference {
    scratch: PathBuf,
    reference: PathBuf,
}

impl Drop for Reference {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.scratch);
    }
}

impl Reference {
    /// Copy only the selected, pinned Filter sources into a separate directory.
    pub fn new(label: &str, sources: &[&str]) -> Self {
        let reference = PathBuf::from(
            std::env::var_os("ARDUPILOT_REFERENCE").expect("set ARDUPILOT_REFERENCE"),
        )
        .canonicalize()
        .unwrap();
        let head = git(&reference)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(head.status.success(), "cannot inspect reference HEAD");
        assert_eq!(
            String::from_utf8_lossy(&head.stdout).trim(),
            PIN,
            "wrong upstream revision"
        );
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let scratch = std::env::temp_dir().join(format!(
            "rustpilot-{label}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&scratch).unwrap();
        let result = Self { scratch, reference };
        for source in sources {
            result.copy_verified(&format!("libraries/Filter/{source}"), source);
        }
        result
    }

    /// Copy another pinned source, retaining verification before compilation.
    pub fn copy_verified(&self, upstream_path: &str, destination: &str) {
        let bytes = verified_source(&self.reference, PIN, upstream_path)
            .unwrap_or_else(|error| panic!("{error}"));
        self.write(destination, bytes);
    }

    /// Read verified bytes for an explicitly documented source extraction.
    pub fn read_verified(&self, upstream_path: &str) -> Vec<u8> {
        verified_source(&self.reference, PIN, upstream_path)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Write a test driver or explicitly disclosed dependency shim.
    pub fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.scratch.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    pub fn compile(&self, driver: &str) {
        self.compile_with_flags(
            driver,
            &[
                "-fsanitize=undefined,float-cast-overflow",
                "-fno-sanitize-recover=all",
            ],
        );
    }

    /// Compile with explicit extra flags. NaN payload selection can depend on
    /// instrumentation, so strict float references record their own profile.
    pub fn compile_with_flags(&self, driver: &str, extra_flags: &[&str]) {
        self.compile_sources_with_flags(driver, &[], extra_flags);
    }

    /// Keep algorithms in separate translation units when the recorded build
    /// profile excludes interprocedural optimization with the test driver.
    pub fn compile_sources_with_flags(&self, driver: &str, sources: &[&str], extra_flags: &[&str]) {
        self.write("driver.cpp", driver);
        let compiler = std::env::var_os("CXX").unwrap_or_else(|| "c++".into());
        let out = Command::new(compiler)
            .args(["-std=c++17", "-O2", "-ffp-contract=off"])
            .args(extra_flags)
            .arg("-I")
            .arg(&self.scratch)
            .arg(self.scratch.join("driver.cpp"))
            .args(sources.iter().map(|source| self.scratch.join(source)))
            .arg("-o")
            .arg(self.scratch.join("reference"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "C++ compile: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Use files for both streams so large fixtures cannot deadlock on pipe
    /// capacity while the child writes output before consuming all input.
    pub fn run(&self, input: &str) -> String {
        self.write("input.txt", input);
        let out = Command::new(self.scratch.join("reference"))
            .env_remove("LD_PRELOAD")
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_AUDIT")
            .stdin(Stdio::from(
                fs::File::open(self.scratch.join("input.txt")).unwrap(),
            ))
            .stdout(Stdio::from(
                fs::File::create(self.scratch.join("output.txt")).unwrap(),
            ))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "C++ run: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        fs::read_to_string(self.scratch.join("output.txt")).unwrap()
    }
}
