// SPDX-License-Identifier: GPL-3.0-or-later
// Ported from ArduPilot libraries/AP_Math/crc.{h,cpp}, revision
// afeb9c4941543d97f3e04fecf257b8f598f49a6a. Credit: ArduPilot contributors;
// Luis Rodrigues (CRC8), Betaflight (DVB CRC8), FastCRC/Frank Boesing
// (Maxim CRC8), avr-liberty (XMODEM), Gary S Brown (CRC32).
//
// Copyright (C) 2010 Swift Navigation Inc.
// Contact: Fergus Noble <fergus@swift-nav.com>
// This source is subject to the license found in the file 'LICENSE' which must
// be be distributed together with this source. All other rights reserved.
// THIS CODE AND INFORMATION IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND,
// EITHER EXPRESSED OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND/OR FITNESS FOR A PARTICULAR PURPOSE.
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
// A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

//! CRCs, checksums, parity and FNV-1a from the pinned ArduPilot collection.
//!
//! Pointer/length pairs become slices; the complete slice is processed, even
//! when its length exceeds the upstream integer length type. CRC4 requires
//! exactly eight words. The input-only C++ mutable pointers become shared
//! borrows. All unsigned truncation and overflow are explicit.
//!
//! These are algorithm ports, with no timing, board or flight qualification.
//! [`crc_crc64`] retains upstream native word byte order. Some upstream lookup
//! tables are evaluated bitwise here; timing and code size can differ.
#![no_std]
#![forbid(unsafe_code)]

/// Initial hash value used for a new FNV-1a 64-bit stream.
pub const FNV_1_OFFSET_BASIS_64: u64 = 14_695_981_039_346_656_037;

/// Four-bit CRC of exactly eight 16-bit words, most significant byte first.
pub fn crc_crc4(data: &[u16; 8]) -> u16 {
    let mut remainder = 0u16;
    for word in data {
        for byte in word.to_be_bytes() {
            remainder ^= u16::from(byte);
            for _ in 0..8 {
                remainder = (remainder << 1) ^ if remainder & 0x8000 != 0 { 0x3000 } else { 0 };
            }
        }
    }
    (remainder >> 12) & 0xf
}

/// CRC8 with polynomial 0x07, zero initial value and no final XOR.
pub fn crc_crc8(data: &[u8]) -> u8 {
    crc8_generic(data, 7, 0)
}

/// CRC8 with a caller-selected polynomial and initial value.
pub fn crc8_generic(data: &[u8], polynomial: u8, initial_value: u8) -> u8 {
    data.iter()
        .fold(initial_value, |crc, &byte| crc8_dvb(byte, crc, polynomial))
}

/// Update the DVB-S2 CRC8 with one byte (polynomial 0xD5).
pub fn crc8_dvb_s2(crc: u8, a: u8) -> u8 {
    crc8_dvb(crc, a, 0xd5)
}

/// Update a CRC8 with one byte; upstream calls the polynomial `seed`.
pub fn crc8_dvb(mut crc: u8, a: u8, seed: u8) -> u8 {
    crc ^= a;
    for _ in 0..8 {
        crc = (crc << 1) ^ if crc & 0x80 != 0 { seed } else { 0 };
    }
    crc
}

/// Update a DVB-S2 CRC8 stream.
pub fn crc8_dvb_s2_update(crc: u8, data: &[u8]) -> u8 {
    data.iter().fold(crc, |crc, &byte| crc8_dvb_s2(crc, byte))
}

/// Update the upstream DVB CRC8 stream with polynomial 0x07.
pub fn crc8_dvb_update(crc: u8, data: &[u8]) -> u8 {
    crc8_generic(data, 7, crc)
}

/// Reflected Maxim CRC8, polynomial 0x8C, zero initial value.
pub fn crc8_maxim(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0x8c } else { 0 };
        }
    }
    crc
}

/// SAE J1850 CRC8, polynomial 0x1D, initial and final XOR 0xFF.
pub fn crc8_sae(data: &[u8]) -> u8 {
    crc8_generic(data, 0x1d, 0xff) ^ 0xff
}

/// RDS02UF's upstream lookup checksum, retaining all table entries verbatim.
pub fn crc8_rds02uf(data: &[u8]) -> u8 {
    data.iter()
        .fold(0, |crc, &byte| RDS02UF[usize::from(crc ^ byte)])
}

/// XOR all input bytes, starting from zero.
pub fn crc_xor_of_bytes(data: &[u8]) -> u8 {
    data.iter().fold(0, |crc, &byte| crc ^ byte)
}

/// Update XMODEM CRC16 with one byte, polynomial 0x1021.
pub fn crc_xmodem_update(crc: u16, data: u8) -> u16 {
    left16(crc ^ (u16::from(data) << 8), 0x1021)
}

/// XMODEM CRC16 with a zero initial value.
pub fn crc_xmodem(data: &[u8]) -> u16 {
    crc16_ccitt(data, 0)
}

/// Update the reflected CRC32 stream without initial or final complement.
pub fn crc_crc32(mut crc: u32, data: &[u8]) -> u32 {
    for &byte in data {
        crc = CRC32_TABLE[usize::from((crc as u8) ^ byte)] ^ (crc >> 8);
    }
    crc
}

/// Bitwise CRC32 variant for small code, with the same seed convention.
pub fn crc32_small(mut crc: u32, data: &[u8]) -> u32 {
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    crc
}

/// CRC24 with polynomial 0x1864CFB and a zero initial value.
pub fn crc_crc24(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &byte in data {
        let mut term = ((crc >> 16) ^ u32::from(byte)) << 16;
        for _ in 0..8 {
            term <<= 1;
            if term & 0x1000000 != 0 {
                term ^= 0x1864cfb;
            }
        }
        crc = ((crc << 8) & 0xffffff) ^ term;
    }
    crc
}

/// Non-reflected IBM/ANSI CRC16 with polynomial 0x8005 and supplied seed.
pub fn crc_crc16_ibm(mut crc: u16, data: &[u8]) -> u16 {
    for &byte in data {
        crc = left16(crc ^ (u16::from(byte) << 8), 0x8005);
    }
    crc
}

/// SPORT/FPort complemented byte sum with an end-around carry after each byte.
pub fn crc_sum8_with_carry(data: &[u8]) -> u8 {
    let mut sum = 0u16;
    for &byte in data {
        sum += u16::from(byte);
        sum += sum >> 8;
        sum &= 0xff;
    }
    (0xff - sum) as u8
}

/// CCITT CRC16 with polynomial 0x1021 and a caller-supplied seed.
/// Copyright (C) 2010 Swift Navigation Inc.; see the notice above and LICENSE.
pub fn crc16_ccitt(data: &[u8], mut crc: u16) -> u16 {
    for &byte in data {
        crc = crc_xmodem_update(crc, byte);
    }
    crc
}

/// Reflected CCITT CRC16 (0x8408), applying `out` XOR even for empty input.
/// For streaming, pass zero `out` until the final chunk.
pub fn crc16_ccitt_r(data: &[u8], mut crc: u16, out: u16) -> u16 {
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0x8408 } else { 0 };
        }
    }
    crc ^ out
}

/// Nonstandard GDL90 CCITT update. Maps upstream `crc16_ccitt_GDL90`.
pub fn crc16_ccitt_gdl90(data: &[u8], mut crc: u16) -> u16 {
    for &byte in data {
        crc = left16(crc & 0xff00, 0x1021) ^ (crc << 8) ^ u16::from(byte);
    }
    crc
}

/// Reflected Modbus CRC16, polynomial 0xA001, initial value 0xFFFF.
pub fn calc_crc_modbus(data: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xa001 } else { 0 };
        }
    }
    crc
}

/// Fletcher16 with both sums reduced modulo 255 after each input byte.
pub fn crc_fletcher16(data: &[u8]) -> u16 {
    let (mut c0, mut c1) = (0u16, 0u16);
    for &byte in data {
        c0 = (c0 + u16::from(byte)) % 255;
        c1 = (c1 + c0) % 255;
    }
    (c1 << 8) | c0
}

/// Update an FNV-1a 64-bit hash in place, using wrapping multiplication.
pub fn hash_fnv_1a(data: &[u8], hash: &mut u64) {
    for &byte in data {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(1_099_511_628_211);
    }
}

/// CRC64-WE over words, preserving upstream native byte order within each word.
/// This produces different results on little-endian and big-endian targets.
pub fn crc_crc64(data: &[u32]) -> u64 {
    let mut crc = u64::MAX;
    for word in data {
        for byte in word.to_ne_bytes() {
            crc ^= u64::from(byte) << 56;
            for _ in 0..8 {
                crc = (crc << 1)
                    ^ if crc & (1 << 63) != 0 {
                        0x42f0e1eba9ea3693
                    } else {
                        0
                    };
            }
        }
    }
    crc ^ u64::MAX
}

/// Return one for an odd number of set bits, otherwise zero.
pub fn parity(mut byte: u8) -> u8 {
    let mut result = 0;
    for _ in 0..8 {
        result ^= byte & 1;
        byte >>= 1;
    }
    result
}

/// Byte sum modulo 256.
pub fn crc_sum_of_bytes(data: &[u8]) -> u8 {
    crc_sum_of_bytes_16(data) as u8
}

/// Byte sum modulo 65536, preserving actual upstream unsigned arithmetic.
/// The upstream comment incorrectly says modulo 0xFFFF.
pub fn crc_sum_of_bytes_16(data: &[u8]) -> u16 {
    data.iter()
        .fold(0u16, |sum, &byte| sum.wrapping_add(u16::from(byte)))
}

fn left16(mut value: u16, polynomial: u16) -> u16 {
    for _ in 0..8 {
        value = (value << 1) ^ if value & 0x8000 != 0 { polynomial } else { 0 };
    }
    value
}

const fn crc32_table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}
const CRC32_TABLE: [u32; 256] = crc32_table();

// Verbatim upstream table, including the repeated 0x06.
const RDS02UF: [u8; 256] = [
    0x93, 0x98, 0xE4, 0x46, 0xEB, 0xBA, 0x04, 0x4C, 0xFA, 0x40, 0xB8, 0x96, 0x0E, 0xB2, 0xB7, 0xC0,
    0x0C, 0x32, 0x9B, 0x80, 0xFF, 0x30, 0x7F, 0x9D, 0xB3, 0x81, 0x58, 0xE7, 0xF1, 0x19, 0x7E, 0xB6,
    0xCD, 0xF7, 0xB4, 0xCB, 0xBC, 0x5C, 0xD6, 0x09, 0x20, 0x0A, 0xE0, 0x37, 0x51, 0x67, 0x24, 0x95,
    0xE1, 0x62, 0xF8, 0x5E, 0x38, 0x15, 0x54, 0x77, 0x63, 0x57, 0x6D, 0xE9, 0x89, 0x76, 0xBE, 0x41,
    0x5D, 0xF9, 0xB1, 0x4D, 0x6C, 0x53, 0x9C, 0xA2, 0x23, 0xC4, 0x8E, 0xC8, 0x05, 0x42, 0x61, 0x71,
    0xC5, 0x00, 0x18, 0x6F, 0x5F, 0xFB, 0x7B, 0x11, 0x65, 0x2D, 0x8C, 0xED, 0x14, 0xAB, 0x88, 0xD5,
    0xD9, 0xC2, 0x36, 0x34, 0x7C, 0x5B, 0x3C, 0xF6, 0x48, 0x0B, 0xEE, 0x02, 0x83, 0x79, 0x17, 0xE6,
    0xA8, 0x78, 0xF5, 0xD3, 0x4E, 0x50, 0x52, 0x91, 0xD8, 0xC6, 0x22, 0xEC, 0x3B, 0xE5, 0x3F, 0x86,
    0x06, 0xCF, 0x2B, 0x2F, 0x3D, 0x59, 0x1C, 0x87, 0xEF, 0x4F, 0x10, 0xD2, 0x7D, 0xDA, 0x72, 0xA0,
    0x9F, 0xDE, 0x6B, 0x75, 0x56, 0xBD, 0xC7, 0xC1, 0x70, 0x1D, 0x25, 0x92, 0xA5, 0x31, 0xE2, 0xD7,
    0xD0, 0x9A, 0xAF, 0xA9, 0xC9, 0x97, 0x08, 0x33, 0x5A, 0x99, 0xC3, 0x16, 0x84, 0x82, 0x8A, 0xF3,
    0x4A, 0xCE, 0xDB, 0x29, 0x0F, 0xAE, 0x6E, 0xE3, 0x8B, 0x07, 0x3A, 0x74, 0x47, 0xB0, 0xBB, 0xB5,
    0x7A, 0xAA, 0x2C, 0xD4, 0x03, 0x3E, 0x1A, 0xA7, 0x27, 0x64, 0x06, 0xBF, 0x55, 0x73, 0x1E, 0xFE,
    0x49, 0x01, 0x39, 0x28, 0xF4, 0x26, 0xDF, 0xDD, 0x44, 0x0D, 0x21, 0xF2, 0x85, 0xB9, 0xEA, 0x4B,
    0xDC, 0x6A, 0xCA, 0xAC, 0x12, 0xFC, 0x2E, 0x2A, 0xA3, 0xF0, 0x66, 0xE8, 0x60, 0x45, 0xA1, 0x8D,
    0x68, 0x35, 0xFD, 0x8F, 0x9E, 0x1F, 0x13, 0xD1, 0xAD, 0x69, 0xCC, 0xA4, 0x94, 0x90, 0x1B, 0x43,
];
