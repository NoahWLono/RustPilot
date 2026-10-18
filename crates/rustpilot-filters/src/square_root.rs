// SPDX-License-Identifier: GPL-3.0-or-later
//! Correctly rounded binary32 and binary64 square roots using integer arithmetic.
//!
//! Finite results use round-to-nearest, ties-to-even independently of the host
//! floating-point mode. Special values match the recorded x86-64 C++ profile:
//! signed zeros survive, input NaNs retain their sign and payload with the quiet
//! bit set, and negative nonzero non-NaNs produce the negative indefinite NaN.
//! Floating-point exception flags and `errno` are not reproduced. Other target
//! builds need separate reference qualification before claiming compatibility.

/// Return the correctly rounded binary32 square root with the module's NaN policy.
pub fn sqrtf(value: f32) -> f32 {
    let bits = value.to_bits();
    let magnitude = bits & 0x7fff_ffff;
    if magnitude > 0x7f80_0000 {
        return f32::from_bits(bits | 0x0040_0000);
    }
    if magnitude == 0 || bits == 0x7f80_0000 {
        return value;
    }
    if bits & 0x8000_0000 != 0 {
        return f32::from_bits(0xffc0_0000);
    }

    let stored_exponent = (bits >> 23) as i32;
    let mut significand = bits & 0x007f_ffff;
    let mut exponent = if stored_exponent == 0 {
        let shift = significand.leading_zeros() - 8;
        significand <<= shift;
        -126 - shift as i32
    } else {
        significand |= 1 << 23;
        stored_exponent - 127
    };
    if exponent & 1 != 0 {
        significand <<= 1;
        exponent -= 1;
    }
    let rounded = rounded_significand::<23>(u64::from(significand)) as u32;
    let result = (((exponent / 2 + 127) as u32) << 23) + (rounded - (1 << 23));
    f32::from_bits(result)
}

/// Return the correctly rounded binary64 square root with the module's NaN policy.
pub fn sqrt(value: f64) -> f64 {
    let bits = value.to_bits();
    let magnitude = bits & 0x7fff_ffff_ffff_ffff;
    if magnitude > 0x7ff0_0000_0000_0000 {
        return f64::from_bits(bits | 0x0008_0000_0000_0000);
    }
    if magnitude == 0 || bits == 0x7ff0_0000_0000_0000 {
        return value;
    }
    if bits & 0x8000_0000_0000_0000 != 0 {
        return f64::from_bits(0xfff8_0000_0000_0000);
    }

    let stored_exponent = (bits >> 52) as i32;
    let mut significand = bits & 0x000f_ffff_ffff_ffff;
    let mut exponent = if stored_exponent == 0 {
        let shift = significand.leading_zeros() - 11;
        significand <<= shift;
        -1022 - shift as i32
    } else {
        significand |= 1 << 52;
        stored_exponent - 1023
    };
    if exponent & 1 != 0 {
        significand <<= 1;
        exponent -= 1;
    }
    let rounded = rounded_significand::<52>(significand);
    let result = (((exponent / 2 + 1023) as u64) << 52) + (rounded - (1 << 52));
    f64::from_bits(result)
}

// Original implementation, derived from the integer-square identity below.
// Callers provide F=23 or F=52 and 2^F <= m < 2^(F+2), with an even exponent e.
// Their input is x=m*2^(e-F). Therefore sqrt(x) has significand sqrt(m*2^F)
// and exponent e/2, so rounding this integer-scaled root rounds the final float.
fn rounded_significand<const F: u32>(significand: u64) -> u64 {
    let radicand = u128::from(significand) << F;
    let mut root = 0_u64;
    let mut remainder = 0_u64;
    // Read the radicand from most significant to least significant base-4 digit.
    // Invariant: the consumed prefix P = root^2 + remainder, 0 <= r < 2*root+1.
    // Appending digit d makes P'=4P+d. Starting with root'=2*root leaves r'=4r+d.
    // Increasing root' by one costs (root'+1)^2-root'^2 = 2*root'+1. This single
    // subtraction establishes the invariant for the next digit and its floor root.
    for pair in (0..=F).rev() {
        remainder = (remainder << 2) | ((radicand >> (2 * pair)) & 3) as u64;
        root <<= 1;
        let trial = (root << 1) | 1;
        if remainder >= trial {
            remainder -= trial;
            root += 1;
        }
    }
    // N=q^2+r. sqrt(N)>q+1/2 iff r>q+1/4, equivalently r>q for integer r.
    // Equality is impossible because (q+1/2)^2 is not an integer, so there is
    // no tie to resolve. This implements round-to-nearest, ties-to-even exactly.
    root + u64::from(remainder > root)
}
