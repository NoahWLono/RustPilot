// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::tangent::tanf;

#[test]
fn preserves_the_discovered_finite_regression_bit_for_bit() {
    assert_eq!(tanf(f32::from_bits(0x3c82_bd0f)).to_bits(), 0x3c82_bfe7);
    assert_eq!(tanf(f32::from_bits(0xbc82_bd0f)).to_bits(), 0xbc82_bfe7);
}

#[test]
fn special_values_match_reference_bits() {
    for (input, expected) in [
        (0x0000_0000, 0x0000_0000),
        (0x8000_0000, 0x8000_0000),
        (0x7f80_0000, 0xffc0_0000),
        (0xff80_0000, 0xffc0_0000),
        (0x7f80_0001, 0x7fc0_0001),
        (0xff80_0001, 0xffc0_0001),
        (0x7fc1_2345, 0x7fc1_2345),
        (0xffc5_4321, 0xffc5_4321),
    ] {
        assert_eq!(tanf(f32::from_bits(input)).to_bits(), expected);
    }
}

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE_HASH: &str = "e63af9308bb8e87d9fcdeba8e0d1ac99435e9ac3c12ffa69993c355a65e1d12b";
const HOST_LIBM_HASH: &str = "965106704753eefe8c31ae7da6daba3ead3d18c52b19b5fcc41178bcc4aa999d";
static SEQUENCE: AtomicUsize = AtomicUsize::new(0);

struct Reference(PathBuf);
impl Reference {
    fn new() -> Self {
        assert_eq!(
            std::env::consts::ARCH,
            "x86_64",
            "this reference's NaN/FPU contract is x86-64"
        );
        assert_eq!(
            std::env::consts::OS,
            "linux",
            "the pinned-source harness currently requires Linux"
        );
        let root = std::env::temp_dir().join(format!(
            "rustpilot-tangent-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let result = Self(root);
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        fs::copy(
            manifest.join("tests/tangent_reference.cpp"),
            result.0.join("driver.cpp"),
        )
        .unwrap();
        fs::copy(
            manifest.join("tests/reference/glibc_tanf.c"),
            result.0.join("glibc_tanf.c"),
        )
        .unwrap();
        assert_hash(&result.0.join("glibc_tanf.c"), SOURCE_HASH);
        for header in [
            "array_length.h",
            "libm-alias-float.h",
            "math_config.h",
            "math_uint128.h",
        ] {
            fs::copy(
                manifest.join("tests/reference/tangent_shims").join(header),
                result.0.join(header),
            )
            .unwrap();
        }
        if std::env::var_os("RUSTPILOT_TEST_HOST_GLIBC").is_some() {
            assert_eq!(std::env::var("RUSTPILOT_TEST_HOST_GLIBC").unwrap(), "1");
            assert_hash(Path::new("/usr/lib/libm.so.6"), HOST_LIBM_HASH);
        }
        let compiler = std::env::var_os("CXX").unwrap_or_else(|| "c++".into());
        let output = Command::new(compiler)
            .args([
                "-std=c++17",
                "-O2",
                "-ffp-contract=off",
                "-fno-fast-math",
                "-fno-builtin-tanf",
                "-fno-builtin-fmaf",
                "-fno-builtin-nearbyint",
                "-fsanitize=undefined,float-cast-overflow",
                "-fno-sanitize-recover=all",
                "-I",
            ])
            .arg(&result.0)
            .arg(result.0.join("driver.cpp"))
            .args(["-lm", "-ldl", "-o"])
            .arg(result.0.join("reference"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "reference compile failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        result
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.0.join("reference"));
        command
            .env_remove("LD_PRELOAD")
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_AUDIT");
        command
    }
}
impl Drop for Reference {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_hash(path: &Path, expected: &str) {
    let output = Command::new("sha256sum").arg(path).output().unwrap();
    assert!(
        output.status.success(),
        "sha256sum failed for {}",
        path.display()
    );
    let actual = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        actual.split_whitespace().next().unwrap(),
        expected,
        "reference changed: {}",
        path.display()
    );
}

fn corpus() -> Vec<u32> {
    let mut inputs = vec![0x3c82_bd0f, 0xbc82_bd0f];
    // Every exponent, both signs, varied mantissas and threshold neighbors.
    for exponent in 0..=255_u32 {
        for mantissa in [
            0, 1, 2, 3, 0x1f_ffff, 0x3f_ffff, 0x40_0000, 0x40_0001, 0x7f_fffc, 0x7f_fffd,
            0x7f_fffe, 0x7f_ffff,
        ] {
            for sign in [0, 0x8000_0000] {
                inputs.push(sign | (exponent << 23) | mantissa);
            }
        }
    }
    // All eight source hard cases and a dense neighborhood around each.
    for center in [
        0x3f8a_1f62_u32,
        0x4d56_d355,
        0x57d7_b0ed,
        0x5980_445e,
        0x63fc_86fe,
        0x6a66_2711,
        0x6ad3_6709,
        0x72b5_05bb,
    ] {
        for value in center - 128..=center + 128 {
            inputs.extend([value, value | 0x8000_0000]);
        }
    }
    // Deterministic raw bits include all float classes, without NaN exclusions.
    let mut state = 0x5255_5354_5049_4c4f_u64;
    for _ in 0..1_000_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32);
    }
    inputs
}

#[test]
#[ignore = "compiles pinned C source; requires x86-64 Linux C++ compiler and sha256sum"]
fn every_output_bit_matches_pinned_tangent_source() {
    let reference = Reference::new();
    let inputs = corpus();
    let input = reference.0.join("input.bin");
    let output = reference.0.join("output.bin");
    {
        let mut writer = BufWriter::new(File::create(&input).unwrap());
        for &bits in &inputs {
            writer.write_all(&bits.to_le_bytes()).unwrap();
        }
        writer.flush().unwrap();
    }
    let result = reference
        .command()
        .stdin(Stdio::from(File::open(&input).unwrap()))
        .stdout(Stdio::from(File::create(&output).unwrap()))
        .output()
        .unwrap();
    eprintln!("{}", String::from_utf8_lossy(&result.stderr));
    assert!(result.status.success(), "reference execution failed");
    assert_eq!(
        fs::metadata(&output).unwrap().len(),
        inputs.len() as u64 * 4
    );
    let mut reader = BufReader::new(File::open(output).unwrap());
    for bits in &inputs {
        let mut expected = [0; 4];
        reader.read_exact(&mut expected).unwrap();
        assert_eq!(
            tanf(f32::from_bits(*bits)).to_bits(),
            u32::from_le_bytes(expected),
            "tanf input={bits:08x}"
        );
    }
    eprintln!(
        "Rust tangent exactly matches {} pinned-source outputs, including NaN bits",
        inputs.len()
    );
}

#[test]
#[ignore = "exhausts 16,777,216 boundary-binade inputs through pinned C source"]
fn tiny_branch_highest_binade_is_exhaustively_bit_preserving() {
    let reference = Reference::new();
    let result = reference.command().arg("--tiny").output().unwrap();
    eprintln!("{}", String::from_utf8_lossy(&result.stderr));
    assert!(
        result.status.success(),
        "reference tiny-binade check failed"
    );
    for sign in [0, 0x8000_0000] {
        for mantissa in 0..1_u32 << 23 {
            let bits = sign | (114 << 23) | mantissa;
            assert_eq!(tanf(f32::from_bits(bits)).to_bits(), bits);
        }
    }
}
