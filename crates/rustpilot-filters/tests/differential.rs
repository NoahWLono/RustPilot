// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::{calc_lowpass_alpha_dt, DigitalLpf, LowPassFilter, LowPassFilterConstDt};
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const PIN: &str = "afeb9c4941543d97f3e04fecf257b8f598f49a6a";
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Compare bytes independently of index flags, then compile these verified
// bytes from scratch rather than rereading a mutable working-tree path.
fn verified_source(reference: &Path, revision: &str, path: &str) -> Result<Vec<u8>, String> {
    let object = Command::new("git")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .arg("-C")
        .arg(reference)
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

#[test]
fn hidden_worktree_source_changes_are_rejected() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "rustpilot-filters-integrity-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    fs::create_dir_all(&scratch.0).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .arg("-C")
            .arg(&scratch.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["init", "--quiet"]);
    fs::write(scratch.0.join("source.cpp"), "original source\n").unwrap();
    git(&["add", "source.cpp"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "fixture",
    ]);
    let revision = git(&["rev-parse", "HEAD"]);
    assert_eq!(
        verified_source(&scratch.0, revision.trim(), "source.cpp").unwrap(),
        b"original source\n"
    );
    for flag in ["skip-worktree", "assume-unchanged"] {
        git(&["update-index", &format!("--{flag}"), "source.cpp"]);
        fs::write(scratch.0.join("source.cpp"), "hidden changed source\n").unwrap();
        assert!(git(&["status", "--porcelain", "--untracked-files=no"]).is_empty());
        let error = verified_source(&scratch.0, revision.trim(), "source.cpp")
            .expect_err("hidden source changes must fail pin verification");
        assert!(error.contains("source.cpp"));
        fs::write(scratch.0.join("source.cpp"), "original source\n").unwrap();
        git(&["update-index", &format!("--no-{flag}"), "source.cpp"]);
    }
}

#[test]
fn replacement_objects_do_not_override_pinned_source() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "rustpilot-filters-replace-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    fs::create_dir_all(&scratch.0).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .arg("-C")
            .arg(&scratch.0)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["init", "--quiet"]);
    fs::write(scratch.0.join("source.cpp"), "original source\n").unwrap();
    git(&["add", "source.cpp"]);
    let commit = || {
        git(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-am",
            "fixture",
        ]);
    };
    commit();
    let pinned = git(&["rev-parse", "HEAD"]);
    fs::write(scratch.0.join("source.cpp"), "replacement source\n").unwrap();
    commit();
    let replacement = git(&["rev-parse", "HEAD"]);
    git(&["replace", pinned.trim(), replacement.trim()]);
    assert_eq!(git(&["replace", "-l"]).trim(), pinned.trim());
    assert!(
        verified_source(&scratch.0, pinned.trim(), "source.cpp").is_err(),
        "replacement bytes must not pass as the original pinned object"
    );
    fs::write(scratch.0.join("source.cpp"), "original source\n").unwrap();
    assert_eq!(
        verified_source(&scratch.0, pinned.trim(), "source.cpp").unwrap(),
        b"original source\n"
    );
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and C++ compiler; run explicitly"]
fn actual_pinned_cpp_scalar_filters_and_extracted_alpha_match() {
    let reference =
        PathBuf::from(std::env::var_os("ARDUPILOT_REFERENCE").expect("set ARDUPILOT_REFERENCE"))
            .canonicalize()
            .unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .arg("-C")
            .arg(&reference)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    };
    assert_eq!(
        git(&["rev-parse", "HEAD"]).trim(),
        PIN,
        "wrong upstream revision"
    );
    assert!(
        git(&["status", "--porcelain", "--untracked-files=no"]).is_empty(),
        "reference must be clean"
    );
    let scratch = Scratch(std::env::temp_dir().join(
        format!("rustpilot-filters-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
    ));
    fs::create_dir_all(scratch.0.join("AP_Math")).unwrap();
    fs::create_dir_all(scratch.0.join("AP_InternalError")).unwrap();
    fs::write(
        scratch.0.join("AP_Math/AP_Math.h"),
        include_str!("reference_math.h"),
    )
    .unwrap();
    fs::write(
        scratch.0.join("AP_InternalError/AP_InternalError.h"),
        "#pragma once\n",
    )
    .unwrap();
    for name in ["LowPassFilter.cpp", "LowPassFilter.h"] {
        let bytes = verified_source(&reference, PIN, &format!("libraries/Filter/{name}")).unwrap();
        fs::write(scratch.0.join(name), bytes).unwrap();
    }
    // Extract this function verbatim; compiling the entire AP_Math.cpp would
    // require HAL/configuration dependencies outside this scalar harness.
    let math = String::from_utf8(
        verified_source(&reference, PIN, "libraries/AP_Math/AP_Math.cpp").unwrap(),
    )
    .unwrap();
    let start = math
        .find("float calc_lowpass_alpha_dt(float dt, float cutoff_freq)\n{")
        .unwrap();
    let end = math[start..].find("\n}").unwrap() + start + 2;
    fs::write(
        scratch.0.join("alpha.cpp"),
        format!("#include <AP_Math/AP_Math.h>\n{}\n", &math[start..end]),
    )
    .unwrap();
    let compiler = std::env::var_os("CXX").unwrap_or_else(|| "c++".into());
    let build = Command::new(compiler)
        .args([
            "-std=c++17",
            "-O2",
            "-ffp-contract=off",
            "-DHAL_DEBUG_BUILD",
        ])
        .arg("-I")
        .arg(&scratch.0)
        .arg("-I")
        .arg(&scratch.0)
        .arg(scratch.0.join("LowPassFilter.cpp"))
        .arg(scratch.0.join("alpha.cpp"))
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference.cpp"))
        .arg("-o")
        .arg(scratch.0.join("reference"))
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "C++ compile: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    let edge = [
        0.0,
        -0.0,
        f32::EPSILON / 2.0,
        -f32::EPSILON / 2.0,
        f32::EPSILON,
        -f32::EPSILON,
        0.001,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for dt in edge {
        for cutoff in edge {
            writeln!(input, "a {} {}", dt.to_bits(), cutoff.to_bits()).unwrap();
            expected.push(calc_lowpass_alpha_dt(dt, cutoff));
        }
    }
    let mut d = DigitalLpf::new();
    let mut v = LowPassFilter::new(2.0);
    let mut c = LowPassFilterConstDt::new(50.0, 2.0);
    let mut random = 0x1234_5678_u32;
    for i in 0..1024 {
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        let sample = ((random >> 8) as f32 / 65536.0) - 128.0;
        let dt = (i % 43) as f32 * 0.001;
        let alpha = (i % 17) as f32 * 0.1;
        if i % 19 == 0 {
            input.push_str("dr\nvr\ncr\n");
            d.reset();
            v.reset();
            c.reset();
        }
        if i % 31 == 0 {
            for op in ["ds", "vs", "cs"] {
                writeln!(input, "{op} {}", sample.to_bits()).unwrap();
            }
            d.reset_to(sample);
            v.reset_to(sample);
            c.reset_to(sample);
        }
        if i % 29 == 0 {
            let cutoff = (i % 37) as f32 - 2.0;
            let sample_rate = (i % 59) as f32 - 5.0;
            writeln!(
                input,
                "vc {}\ncc {} {}",
                cutoff.to_bits(),
                sample_rate.to_bits(),
                cutoff.to_bits()
            )
            .unwrap();
            v.set_cutoff_frequency(cutoff);
            c.set_cutoff_frequency(sample_rate, cutoff);
        }
        writeln!(
            input,
            "d {} {}\nv {} {}\nc {}",
            sample.to_bits(),
            alpha.to_bits(),
            sample.to_bits(),
            dt.to_bits(),
            sample.to_bits()
        )
        .unwrap();
        expected.extend([d.apply(sample, alpha), v.apply(sample, dt), c.apply(sample)]);
    }
    for value in edge {
        input.push_str("dr\nvr\ncr\n");
        d.reset();
        v.reset();
        c.reset();
        for sample in [value, 1.0, 2.0] {
            writeln!(
                input,
                "d {} {}\nv {} {}\nc {}",
                sample.to_bits(),
                value.to_bits(),
                sample.to_bits(),
                value.to_bits(),
                sample.to_bits()
            )
            .unwrap();
            expected.extend([
                d.apply(sample, value),
                v.apply(sample, value),
                c.apply(sample),
            ]);
        }
    }
    for sample_frequency in edge {
        for cutoff in edge {
            writeln!(
                input,
                "cc {} {}\ncr",
                sample_frequency.to_bits(),
                cutoff.to_bits()
            )
            .unwrap();
            c.set_cutoff_frequency(sample_frequency, cutoff);
            c.reset();
            for sample in [1.0_f32, -2.0, 4.0] {
                writeln!(input, "c {}", sample.to_bits()).unwrap();
                expected.push(c.apply(sample));
            }
        }
    }
    for _ in 0..1024 {
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        let dt = f32::from_bits(random);
        random = random.wrapping_mul(1664525).wrapping_add(1013904223);
        let cutoff = f32::from_bits(random);
        writeln!(input, "a {} {}", dt.to_bits(), cutoff.to_bits()).unwrap();
        expected.push(calc_lowpass_alpha_dt(dt, cutoff));
    }
    fs::write(scratch.0.join("input"), input).unwrap();
    let output = Command::new(scratch.0.join("reference"))
        .stdin(fs::File::open(scratch.0.join("input")).unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    let actual: Vec<f32> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| f32::from_bits(line.parse().unwrap()))
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (i, (a, b)) in actual.iter().zip(&expected).enumerate() {
        assert!(
            (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits(),
            "case {i}: C++ {a:?}, Rust {b:?}"
        );
    }
    eprintln!("{} scalar outputs matched (NaN payloads ignored); actual pinned LowPassFilter.cpp plus extracted alpha", expected.len());
}
