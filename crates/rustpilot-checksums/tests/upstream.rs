// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_checksums::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const PIN: &str = "afeb9c4941543d97f3e04fecf257b8f598f49a6a";
const LABELS: [&str; 28] = [
    "crc_crc4",
    "crc_crc8",
    "crc8_generic",
    "crc8_dvb_s2",
    "crc8_dvb",
    "crc8_dvb_s2_update",
    "crc8_dvb_update",
    "crc8_maxim",
    "crc8_sae",
    "crc8_rds02uf",
    "crc_xor_of_bytes",
    "crc_xmodem_update",
    "crc_xmodem",
    "crc_crc32",
    "crc32_small",
    "crc_crc24",
    "crc_crc16_ibm",
    "crc_sum8_with_carry",
    "crc16_ccitt",
    "crc16_ccitt_r",
    "crc16_ccitt_gdl90",
    "calc_crc_modbus",
    "crc_fletcher16",
    "hash_fnv_1a",
    "crc_crc64",
    "parity",
    "crc_sum_of_bytes",
    "crc_sum_of_bytes_16",
];

fn run(command: &mut Command) -> Output {
    let output = command.output().expect("run reference prerequisite");
    assert!(
        output.status.success(),
        "{command:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn git(reference: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .arg("-C")
        .arg(reference);
    command
}

struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and g++; run explicitly"]
fn all_28_functions_match_actual_pinned_upstream_cpp() {
    let reference = std::env::var_os("ARDUPILOT_REFERENCE")
        .expect("set ARDUPILOT_REFERENCE to the pinned upstream checkout");
    let reference = Path::new(&reference)
        .canonicalize()
        .expect("reference path");
    let head = run(git(&reference).args(["rev-parse", "HEAD"]));
    assert_eq!(
        String::from_utf8(head.stdout).unwrap().trim(),
        PIN,
        "wrong upstream revision"
    );
    for file in ["libraries/AP_Math/crc.cpp", "libraries/AP_Math/crc.h"] {
        let committed = run(git(&reference).args(["show", &format!("{PIN}:{file}")]));
        assert_eq!(
            fs::read(reference.join(file)).unwrap(),
            committed.stdout,
            "modified upstream {file}"
        );
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = TempDir(
        std::env::temp_dir().join(format!("rustpilot-crc-{}-{unique}", std::process::id())),
    );
    fs::create_dir(&temp.0).unwrap();
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
    let exe = temp.0.join("crc-reference");
    run(Command::new("g++")
        .args(["-std=c++17", "-O2", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(&local)
        .arg("-I")
        .arg(reference.join("libraries/AP_Math"))
        .arg(local.join("main.cpp"))
        .arg(reference.join("libraries/AP_Math/crc.cpp"))
        .arg("-o")
        .arg(&exe));
    let lengths = [
        0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 127, 128, 254, 255, 256, 257, 1023, 4096, 65534,
        65535,
    ];
    let mut cases = Vec::new();
    for n in lengths {
        for seed in [0, u64::MAX, 0x5555aaaa5555aaaa, 0x0123456789abcdef] {
            cases.push((n, seed));
        }
    }
    let mut state = 0x4d595df4d0f33173;
    for _ in 0..128 {
        cases.push(((next(&mut state) % 2048) as usize, next(&mut state)));
    }
    let mut fixture = Vec::new();
    fixture.extend_from_slice(&(cases.len() as u32).to_le_bytes());
    let mut expected = Vec::new();
    for &(n, seed) in &cases {
        let data: Vec<_> = (0..n).map(|_| next(&mut state) as u8).collect();
        let words: Vec<_> = (0..n).map(|_| next(&mut state) as u32).collect();
        let prom: [u16; 8] = core::array::from_fn(|_| next(&mut state) as u16);
        fixture.extend_from_slice(&(n as u32).to_le_bytes());
        fixture.extend_from_slice(&seed.to_le_bytes());
        fixture.extend_from_slice(&data);
        fixture.extend_from_slice(&(words.len() as u16).to_le_bytes());
        for word in &words {
            fixture.extend_from_slice(&word.to_le_bytes());
        }
        for word in prom {
            fixture.extend_from_slice(&word.to_le_bytes());
        }
        let a = data.first().copied().unwrap_or(0);
        let short = &data[..n.min(255)];
        let mut hash = seed;
        hash_fnv_1a(&data, &mut hash);
        expected.push([
            crc_crc4(&prom) as u64,
            crc_crc8(short) as u64,
            crc8_generic(&data, (seed >> 8) as u8, seed as u8) as u64,
            crc8_dvb_s2(seed as u8, a) as u64,
            crc8_dvb(seed as u8, a, (seed >> 8) as u8) as u64,
            crc8_dvb_s2_update(seed as u8, &data) as u64,
            crc8_dvb_update(seed as u8, &data) as u64,
            crc8_maxim(&data) as u64,
            crc8_sae(&data) as u64,
            crc8_rds02uf(&data) as u64,
            crc_xor_of_bytes(&data) as u64,
            crc_xmodem_update(seed as u16, a) as u64,
            crc_xmodem(&data) as u64,
            crc_crc32(seed as u32, &data) as u64,
            crc32_small(seed as u32, &data) as u64,
            crc_crc24(&data) as u64,
            crc_crc16_ibm(seed as u16, &data) as u64,
            crc_sum8_with_carry(short) as u64,
            crc16_ccitt(&data, seed as u16) as u64,
            crc16_ccitt_r(&data, seed as u16, (seed >> 16) as u16) as u64,
            crc16_ccitt_gdl90(&data, seed as u16) as u64,
            calc_crc_modbus(&data) as u64,
            crc_fletcher16(&data) as u64,
            hash,
            crc_crc64(&words),
            parity(a) as u64,
            crc_sum_of_bytes(&data) as u64,
            crc_sum_of_bytes_16(&data) as u64,
        ]);
    }
    let input = temp.0.join("fixtures.bin");
    fs::write(&input, fixture).unwrap();
    let output = run(Command::new(&exe).arg(input));
    let text = String::from_utf8(output.stdout).unwrap();
    let rows: Vec<_> = text.lines().collect();
    assert_eq!(rows.len(), expected.len());
    for (case, (row, expected)) in rows.iter().zip(expected).enumerate() {
        let actual: Vec<u64> = row.split_whitespace().map(|s| s.parse().unwrap()).collect();
        assert_eq!(actual.len(), LABELS.len());
        for (i, (&actual, &expected)) in actual.iter().zip(&expected).enumerate() {
            assert_eq!(
                expected, actual,
                "{} case {case}: {:?}",
                LABELS[i], cases[case]
            );
        }
    }
    eprintln!(
        "Compared {} fixtures x 28 functions = {} outputs against actual {PIN} crc.cpp",
        cases.len(),
        cases.len() * 28
    );
}

#[test]
fn reference_source_reads_ignore_git_replacement_objects() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp = TempDir(std::env::temp_dir().join(format!(
        "rustpilot-crc-replace-{}-{unique}",
        std::process::id()
    )));
    fs::create_dir(&temp.0).unwrap();
    run(git(&temp.0).arg("init"));
    for source in ["pinned original\n", "replacement source\n"] {
        fs::write(temp.0.join("crc.cpp"), source).unwrap();
        run(git(&temp.0).args(["add", "crc.cpp"]));
        run(git(&temp.0).args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-m",
            "fixture",
        ]));
    }
    let original =
        String::from_utf8(run(git(&temp.0).args(["rev-parse", "HEAD~"])).stdout).unwrap();
    let original = original.trim();
    run(git(&temp.0).args(["replace", original, "HEAD"]));
    let spoofed = run(Command::new("git")
        .arg("-C")
        .arg(&temp.0)
        .env_remove("GIT_NO_REPLACE_OBJECTS")
        .args(["show", &format!("{original}:crc.cpp")]));
    assert_eq!(spoofed.stdout, b"replacement source\n");
    let pinned = run(git(&temp.0).args(["show", &format!("{original}:crc.cpp")]));
    assert_eq!(pinned.stdout, b"pinned original\n");
}
