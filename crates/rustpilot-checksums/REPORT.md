# Checksum port verification

This crate ports all 28 functions declared in ArduPilot's
`libraries/AP_Math/crc.h` at revision
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`. Its production Rust is `no_std`,
forbids unsafe code, allocates no memory and has no Cargo dependencies.
Codex translated the algorithms and wrote the tests. This is local software
evidence; hardware behavior, timing, flight operation and the full ArduPilot
migration remain unverified.

## API mapping

The following list is the complete exported function set. Byte buffers use
`&[u8]`; seeds, polynomials and output XORs retain their unsigned upstream widths.
Arguments keep their upstream order after removing pointer lengths, except
`hash_fnv_1a`, whose removed first length leaves `(data, hash)`.

| Rust API | Upstream API or behavior |
| --- | --- |
| `crc_crc4(&[u16; 8]) -> u16` | Same name; fixed eight-word input |
| `crc_crc8(data) -> u8` | Same name; polynomial 0x07 |
| `crc8_generic(data, polynomial, initial_value) -> u8` | Same name; explicit initial value |
| `crc8_dvb_s2(crc, a) -> u8` | Same name |
| `crc8_dvb(crc, a, seed) -> u8` | Same name; `seed` means polynomial |
| `crc8_dvb_s2_update(crc, data) -> u8` | Same name |
| `crc8_dvb_update(crc, data) -> u8` | Same name |
| `crc8_maxim(data) -> u8` | Same name |
| `crc8_sae(data) -> u8` | Same name |
| `crc8_rds02uf(data) -> u8` | Same name; verbatim upstream table |
| `crc_xor_of_bytes(data) -> u8` | Same name |
| `crc_xmodem_update(crc, data) -> u16` | Same name |
| `crc_xmodem(data) -> u16` | Same name |
| `crc_crc32(crc, data) -> u32` | Same name; no implicit complement |
| `crc32_small(crc, data) -> u32` | Same name; bitwise variant |
| `crc_crc24(data) -> u32` | Same name |
| `crc_crc16_ibm(crc, data) -> u16` | Same name; non-reflected polynomial 0x8005 |
| `crc_sum8_with_carry(data) -> u8` | Same name |
| `crc16_ccitt(data, crc) -> u16` | Same name |
| `crc16_ccitt_r(data, crc, out) -> u16` | Same name; applies output XOR each call |
| `crc16_ccitt_gdl90(data, crc) -> u16` | Upstream `crc16_ccitt_GDL90` |
| `calc_crc_modbus(data) -> u16` | Same name |
| `crc_fletcher16(data) -> u16` | Same name |
| `hash_fnv_1a(data, &mut u64)` | Same name; updates caller's hash |
| `crc_crc64(&[u32]) -> u64` | Same name; native word byte order |
| `parity(byte) -> u8` | Same name |
| `crc_sum_of_bytes(data) -> u8` | Same name |
| `crc_sum_of_bytes_16(data) -> u16` | Same name; actual modulo 65536 behavior |

`FNV_1_OFFSET_BASIS_64` is also exported with upstream's value.

## Behavior changes and retained details

- Slices prevent a caller from supplying a length larger than the buffer.
  They process their full length, including lengths beyond the original `u8`,
  `u16` or `u32` count range. Those larger inputs are an API extension. Direct
  C++ comparisons cover only lengths representable by the upstream API.
- CRC4's eight words are enforced by the Rust type. The source does not mutate
  CRC4 or IBM input buffers, so Rust uses shared borrows for both.
- CRC64 preserves upstream's native byte order within each word using
  `to_ne_bytes()`. The differential run was on a little-endian host. Big-endian
  output is defined by the same rule but has no executed comparison evidence.
- The 16-bit sum wraps modulo 65536. The upstream comment saying modulo 0xFFFF
  is inaccurate; the Rust documentation describes actual arithmetic.
- The RDS02UF table's repeated 0x06 entry is retained. There is no evidence
  establishing that it is a mistake or supporting a corrected replacement.
- CRC8, Maxim, SAE, CCITT, GDL90 and IBM table operations use equivalent bitwise
  steps here. CRC32 retains a table generated at compile time; the small CRC32
  remains bitwise. Flash, performance and timing equivalence are unmeasured.
- `crc16_ccitt_r` applies `out` even on an empty buffer. Streaming callers must
  use zero output XOR for intermediate chunks, then apply the desired XOR once.
- C++ default arguments are unavailable in Rust; `crc8_generic` requires an
  explicit initial value. The GDL90 export uses Rust's snake-case spelling.

## Verification

The tests were written against panic-only API stubs before algorithm logic.
Four behavioral tests failed for missing implementations; the direct C++ test
also failed against those stubs after successfully compiling the reference.
A fifth test subsequently checked the documented slice-length extension.

Fresh checks with Rust/Cargo 1.95.0 passed:

- Five behavioral tests: standard vectors including `123456789`, empty buffers,
  all 256 byte parity values, explicit nonstandard entries, wrapping sums,
  seed-preserving updates, streaming splits and extended slice lengths.
- One source-provenance regression test creates Git replacement objects and
  verifies that source reads retain the original object contents.
- One explicitly enabled differential integration test: 212 fixtures, each
  comparing all 28 functions, for 5,936 equal outputs. The fixtures combine
  21 boundary lengths with four seed patterns and 128 deterministic random
  length/seed pairs. Byte and word count boundaries include 0, 1, 255, 256,
  257, 65534 and 65535. CRC8/carry calls cap each fixture at their original
  255-byte limit. CRC64 fixtures include 65535 words. CRC4 receives eight
  deterministic random words in every fixture.
- Crate release build, formatting and Clippy with warnings denied.

Run the reproducible crate checks from the workspace root:

```sh
cargo +1.95.0 test -p rustpilot-checksums
ARDUPILOT_REFERENCE=/absolute/path/to/ardupilot-upstream \
  cargo +1.95.0 test -p rustpilot-checksums --test upstream -- --ignored --nocapture
cargo +1.95.0 build -p rustpilot-checksums --release
cargo +1.95.0 fmt -p rustpilot-checksums --check
cargo +1.95.0 clippy -p rustpilot-checksums --all-targets -- -D warnings
```

The differential test is marked ignored by default so an ordinary test run
visibly reports its omission. Explicit execution requires `ARDUPILOT_REFERENCE`
and `g++`; missing prerequisites fail. The test requires the exact Git HEAD
above and compares the on-disk `crc.h` and `crc.cpp` bytes to their committed
versions before compilation. Every reference Git command sets
`GIT_NO_REPLACE_OBJECTS=1`, so replacement objects cannot substitute another
commit or source blob during these checks. It compiles the actual upstream `crc.cpp` together
with `tests/reference/main.cpp`. Production Rust has no C++ build or linkage.

The only substituted upstream include is the empty test-only
`tests/reference/AP_HAL/AP_HAL_Boards.h`: CRC source includes that header but
uses none of its declarations or macros. No preprocessor definitions,
algorithm replacements or extracted/reimplemented C++ CRCs are supplied.
The harness only decodes fixtures, calls upstream functions and prints results.
It compiles with `g++ -std=c++17 -O2 -Wall -Wextra -Werror`.

## Attribution and licensing

The crate's `LICENSE` reproduces upstream `COPYING.txt` (GPL version 3).
The source carries the GPL-3.0-or-later notice and preserves the Swift Navigation
copyright and disclaimer from upstream. Algorithm credits in upstream source
are retained for Luis Rodrigues, Betaflight, FastCRC/Frank Boesing, avr-liberty
and Gary S Brown, alongside credit to ArduPilot contributors. No claim of
independent invention or upstream endorsement is made.

All 28 declared checksum functions have implementations. There is no proof of
universal equivalence: comparisons are finite tests on this host, and embedded
firmware builds, target execution, resource measurements and hardware tests
remain outstanding. The coordinator verified library compilation for
`thumbv6m-none-eabi` and `thumbv7em-none-eabihf`; see
[workspace verification](../../docs/VERIFICATION.md).
