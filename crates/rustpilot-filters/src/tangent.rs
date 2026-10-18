// SPDX-License-Identifier: MIT
// Translated from glibc sysdeps/ieee754/flt-32/s_tanf.c at
// 1848099f063e99d4ffecbd7667766d54862398b9.
// The source was copied from CORE-MATH src/binary32/tan/tanf.c, revision 59d21d7.
//
// Copyright (c) 2022-2024 Alexei Sibidanov.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Tangent matching the pinned glibc binary32 algorithm's output bits.
//!
//! The contract assumes round-to-nearest, ties-to-even and gradual underflow.
//! NaN output bits follow the pinned x86-64 reference explicitly. Floating-point
//! exception flags and C `errno` are outside this API. See `TANGENT_REPORT.md`
//! for provenance, arithmetic assumptions, and the scope of verification.

/// Compute tangent with the pinned glibc algorithm, without a foreign call.
///
/// Signed zeros are preserved. NaNs retain their sign/payload and are quieted;
/// either infinity returns the x86-64 reference's negative quiet NaN.
/// Arithmetic requires round-to-nearest, ties-to-even with gradual underflow.
#[must_use]
pub fn tanf(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff {
        return if bits & 0x007f_ffff != 0 {
            f32::from_bits(bits | 0x0040_0000)
        } else {
            f32::from_bits(0xffc0_0000)
        };
    }
    if exponent < 115 {
        // In nearest-even mode both source fmaf branches round back to x.
        // For the highest binade, |x| < 2^-12 and half an ULP is 2^-37.
        // The cubic correction is < 2^-36 / 3 (plus < 3*2^-24 relative
        // rounding error), hence < 2^-37; lower binades tighten this bound.
        // The e<102 correction is x*abs(x), also strictly below half an ULP.
        // Returning x preserves negative zero and every subnormal's bits.
        return x;
    }
    let (z, quadrant) = if exponent < 155 {
        reduce_small(x)
    } else {
        reduce_large(bits)
    };
    let z2 = z * z;
    let z4 = z2 * z2;
    let mut n = CN[0] + z2 * CN[1];
    let n2 = CN[2] + z2 * CN[3];
    n += z4 * n2;
    let mut d = CD[0] + z2 * CD[1];
    let d2 = CD[2] + z2 * CD[3];
    d += z4 * d2;
    n *= z;
    let select = [0.0_f64, 1.0];
    let s0 = select[(quadrant & 1) as usize];
    let s1 = select[(1 - (quadrant & 1)) as usize];
    let result = (n * s1 - d * s0) / (n * s0 + d * s1);
    let tail = result.to_bits().wrapping_add(7) & (u64::MAX >> 35);
    if tail <= 14 {
        let absolute_bits = bits & 0x7fff_ffff;
        for &(argument, high, low) in &HARD_CASES {
            if absolute_bits == argument {
                let high = f32::from_bits(high);
                let low = f32::from_bits(low);
                return if bits >> 31 != 0 {
                    -high - low
                } else {
                    high + low
                };
            }
        }
    }
    result as f32
}

// Values are exact IEEE encodings of the source hexadecimal literals.
const CN: [f64; 4] = [
    f64::from_bits(0x3ff9_21fb_5444_2d18),
    f64::from_bits(0xbfdf_d226_e573_289f),
    f64::from_bits(0x3f9b_7a60_c8da_c9f6),
    f64::from_bits(0xbf27_25be_b40f_33e5),
];
const CD: [f64; 4] = [
    1.0,
    f64::from_bits(0xbff2_3953_47fb_829d),
    f64::from_bits(0x3fc2_3136_60f2_9c36),
    f64::from_bits(0xbf69_a707_ab98_d1c1),
];
const HARD_CASES: [(u32, u32, u32); 8] = [
    (0x3f8a_1f62, 0x3fee_fcfb, 0xa5c4_8e92),
    (0x4d56_d355, 0x3e74_0182, 0x22a0_cfa3),
    (0x57d7_b0ed, 0x3eb0_68e4, 0xa416_b61d),
    (0x5980_445e, 0x3fe5_0f68, 0x257b_0298),
    (0x63fc_86fe, 0x3f2c_bfce, 0x2549_2cbc),
    (0x6a66_2711, 0xc079_9ac2, 0x266b_92a5),
    (0x6ad3_6709, 0xbf62_b097, 0xa513_619f),
    (0x72b5_05bb, 0xbff2_150f, 0xa58e_e483),
];

fn reduce_small(x: f32) -> (f64, i32) {
    let x = f64::from(x);
    let low = f64::from_bits(0xbdfb_1bbe_ad60_3d8b) * x;
    let high = f64::from_bits(0x3fe4_5f30_6e00_0000) * x;
    // |high| < 2^28: truncation and the adjacent integer are exact in f64.
    // This implements roundeven_finite without a runtime math dependency.
    let integer = high as i64;
    let fraction = high - integer as f64;
    let odd = integer & 1 != 0;
    let nearest = if fraction > 0.5 || (fraction == 0.5 && odd) {
        integer + 1
    } else if fraction < -0.5 || (fraction == -0.5 && odd) {
        integer - 1
    } else {
        integer
    };
    ((high - nearest as f64) + low, nearest as i32)
}

fn reduce_large(bits: u32) -> (f64, i32) {
    const INVERSE_PI: [u64; 4] = [
        0xfe51_63ab_debb_c562,
        0xdb62_9599_3c43_9041,
        0xfc27_57d1_f534_ddc0,
        0xa2f9_836e_4e44_1529,
    ];
    let exponent = (bits >> 23) & 0xff;
    let mantissa = u128::from((bits & 0x007f_ffff) | (1 << 23));
    // Each product is <2^88, so the carry additions cannot overflow u128.
    let p0 = mantissa * u128::from(INVERSE_PI[0]);
    let p1 = mantissa * u128::from(INVERSE_PI[1]) + (p0 >> 64);
    let p2 = mantissa * u128::from(INVERSE_PI[2]) + (p1 >> 64);
    let p3 = mantissa * u128::from(INVERSE_PI[3]) + (p2 >> 64);
    let p3h = (p3 >> 64) as u64;
    let p3l = p3 as u64;
    let p2l = p2 as u64;
    let p1l = p1 as u64;
    // Called only for exponent 155..=254, giving shift 5..=104.
    let shift = exponent - 150;
    let (integer_bits, fraction_bits) = if shift < 64 {
        (
            (p3h << shift) | (p3l >> (64 - shift)),
            (p3l << shift) | (p2l >> (64 - shift)),
        )
    } else if shift == 64 {
        (p3l, p2l)
    } else {
        (
            (p3l << (shift - 64)) | (p2l >> (128 - shift)),
            (p2l << (shift - 64)) | (p1l >> (128 - shift)),
        )
    };
    let fraction = fraction_bits as i64;
    let sign = (bits as i32) >> 31;
    let mut quadrant = (integer_bits as i32).wrapping_sub((fraction >> 63) as i32);
    let reduced = (fraction ^ i64::from(sign)) as f64 * f64::from_bits(0x3bf0_0000_0000_0000);
    quadrant = (quadrant ^ sign).wrapping_sub(sign);
    (reduced, quadrant)
}
