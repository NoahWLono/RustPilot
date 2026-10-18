# Binary32 tangent reference and Rust port

`tangent::tanf` implements the tangent algorithm from glibc commit
`1848099f063e99d4ffecbd7667766d54862398b9` in safe Rust. It has no Cargo
library dependency or explicit foreign function call. Its contract is the returned
binary32 bit pattern under round-to-nearest, ties-to-even, with gradual
underflow. It explicitly preserves the pinned x86-64 reference's NaN sign
and payload behavior. C `errno` and floating-point exception flags are
outside this function's contract.

This is a host-reference math port. It does not establish agreement with
other boards' C libraries, different rounding modes, or flush-to-zero
settings. Cross-compiling the library does not execute it on those boards.
The existing compiler, compiler runtime, and host OS remain bootstrap
dependencies.

## Source and license

The original [glibc tangent source](https://github.com/bminor/glibc/blob/1848099f063e99d4ffecbd7667766d54862398b9/sysdeps/ieee754/flt-32/s_tanf.c)
was copied into `tests/reference/glibc_tanf.c` without modification. Its
SHA-256 is:

```text
e63af9308bb8e87d9fcdeba8e0d1ac99435e9ac3c12ffa69993c355a65e1d12b
```

The file credits Alexei Sibidanov, 2022–2024, and says it originated in
CORE-MATH `src/binary32/tan/tanf.c`, revision `59d21d7`. It is licensed under
MIT. The Rust translation and reference fixture retain the full copyright
and permission notice. Other glibc files have different licenses; this
notice applies to the tangent source and its translation.

The reference fixture is compiled with small headers in
`tests/reference/tangent_shims/`. These supply bit conversions, unsigned
128-bit operations, array length, an empty symbol-alias macro, nearest-even
rounding through `nearbyint` under an asserted `FE_TONEAREST`, and the
invalid-float output expression. They do not replace the tangent algorithm.
The tests verify the copied C fixture's SHA-256 before compiling it.

## Arithmetic decisions

The Rust port retains the source's double-precision range reduction,
rational approximation, operation order, and eight exceptional inputs.
Hexadecimal floating literals were converted to exact IEEE encodings with
Python `float.fromhex` and `struct.pack`; the Rust constants use
`f32::from_bits` and `f64::from_bits`. The large-input reduction uses Rust
`u128` for products. Each product uses a 24-bit mantissa and a 64-bit
constant; the carry additions remain below 89 bits. Integer truncation
and wrapping preserve the source's x86-64 integer representation, without
unsafe indexing or signed-overflow behavior in Rust.

Small-input range reduction needs nearest-even integer rounding only for
finite values of magnitude below `2^28`. Truncating to `i64`, checking the
exact residual against `+0.5` and `-0.5`, and adjusting odd ties implements
that operation without calling a math library.

The source's two tiny-input branches use `fmaf`. The Rust function returns
the input directly for exponent fields below 115. This preserves output
bits under the stated rounding mode; it deliberately does not reproduce
the source's floating-point exception flags.

For a normal input in binade `[2^b, 2^(b+1))`, half an ULP is `2^(b-24)`.
For source exponent fields 102 through 114, `-25 <= b <= -13`. The magnitude
of the cubic correction is less than
`2^(3b+3) / 3 * (1 + 2^-24)^3`. Its ratio to half an ULP is less than
`2^(2b+27) / 3 * (1 + 2^-24)^3`, whose largest bound, at `b = -13`, is
`2/3 * (1 + 2^-24)^3 < 1`. The three factors conservatively cover rounding
of the binary32 one-third constant and two multiplications. All products
in this branch are normal. For exponent fields below 102, the source's
correction has magnitude `x^2`. At the highest binade, `b = -26`, it is
strictly less than `2^-50`, which is half an ULP; smaller normal binades
make the bound smaller. Subnormal squares are below half the subnormal
ULP. The correction has the same sign as the input, so the spacing away
from zero at exact powers of two is the relevant spacing. Signed zero is
returned unchanged.

For NaN inputs, the Rust code sets the quiet bit and preserves all other
bits. Either infinity produces `0xffc00000`, the observed pinned x86-64
result. This explicit handling avoids relying on Rust's unspecified NaN
payload selection during arithmetic.

## Host implementation audit

The development host has Arch package
`glibc-2.44+r50+g1848099f063e-1`, GCC `16.2.1 20260810`, and an AMD Ryzen 3
7330U. Installed `/usr/lib/libm.so.6` and the copy extracted from the cached
package both have SHA-256:

```text
965106704753eefe8c31ae7da6daba3ead3d18c52b19b5fcc41178bcc4aa999d
```

The source commit was resolved from the package's commit abbreviation.
This audit did not rebuild the distribution binary from source; the
source and installed binary were compared on the test inputs below.

`readelf` identifies `tanf` as a regular function at library offset
`0x81180`. Disassembly shows separate SSE2 multiply/add instructions in
its rational approximation and a call to `fmaf` in the tiny-input paths.
The actual `fmaf` implementation is selected at runtime.

The host's `cosf` is also selected at runtime. With FMA and AVX2 enabled,
it resolves to offset `0x950b0`; with those capabilities disabled through
`GLIBC_TUNABLES`, it resolves to `0x952f0`. The selected FMA version contains
fused double-precision arithmetic. This matters for any future general
cosine port. The two-pole filter's fixed `cosf(M_PI / 4.0f)` expression
produced `0x3f3504f3` with GCC constant folding and with either runtime
implementation. General cosine behavior still needs a separate port and verification.

## Verification

The four tangent tests passed in debug and release profiles. Each profile
compared 1,010,258 inputs against the compiled pinned C source with exact
`to_bits()` equality. Inputs include all exponent fields and both signs,
selected mantissas, the eight hard cases and 128 neighbors on each side,
and one million deterministic raw bit patterns. NaNs are included with no
payload exemption. The previous rejected-library counterexample
`0x3c82bd0f -> 0x3c82bfe7` has a separate regression test.

A separate exhaustive boundary test passed all 16,777,216 inputs with
exponent field 114. It covers every significand and both signs and checks
both the compiled C function and Rust function return the input bits.
The proof above covers the remaining tiny-input binades; the complete
binary32 domain has not been exhaustively compared.

Both tests also compared every listed input to the installed, hash-checked
host `tanf` when explicitly run with `RUSTPILOT_TEST_HOST_GLIBC=1`. This
passed in both profiles. An additional release run with FMA and AVX2
disabled also passed. The C driver asserts `FE_TONEAREST`, flush-to-zero
disabled, and denormals-are-zero disabled, and prints the observed MXCSR.
The normal reference runs reported `0x00001f80` before computation.

The reference builds use C++17, `-O2`, `-ffp-contract=off`,
`-fno-fast-math`, disabled compiler builtins for `tanf`, `fmaf`, and
`nearbyint`, and undefined-behavior/float-cast-overflow sanitizers with
recovery disabled. No sanitizer failure occurred.

Reproduce the portable pinned-source comparison from the repository root:

```sh
cargo test --locked -p rustpilot-filters --test tangent -- --include-ignored --nocapture
cargo test --locked --release -p rustpilot-filters --test tangent -- --include-ignored --nocapture
```

The reference harness currently requires x86-64 Linux, a C++ compiler,
and `sha256sum`. It reports separately that the host comparison was not
requested. To additionally compare the exact audited host library:

```sh
RUSTPILOT_TEST_HOST_GLIBC=1 cargo test --locked -p rustpilot-filters --test tangent -- --include-ignored --nocapture
```

That additional check fails if the installed library hash differs. A
different host must be audited explicitly; changing a version label does
not establish compatibility. The Rust library also compiled for
`thumbv6m-none-eabi` and `thumbv7em-none-eabihf`; these builds provide no
on-device numerical or timing evidence.
