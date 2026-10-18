// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_checksums::*;

#[test]
fn standard_check_vectors_catch_polynomial_seed_and_final_xor_errors() {
    let data = b"123456789";
    assert_eq!(crc_crc8(data), 0xf4);
    assert_eq!(crc8_generic(data, 0x07, 0), 0xf4);
    assert_eq!(crc8_dvb_s2_update(0, data), 0xbc);
    assert_eq!(crc8_dvb_update(0, data), 0xf4);
    assert_eq!(crc8_maxim(data), 0xa1);
    assert_eq!(crc8_sae(data), 0x4b);
    assert_eq!(crc_xmodem(data), 0x31c3);
    assert_eq!(crc_crc32(u32::MAX, data), 0x340b_c6d9);
    assert_eq!(crc32_small(u32::MAX, data), 0x340b_c6d9);
    assert_eq!(crc16_ccitt(data, 0xffff), 0x29b1);
    assert_eq!(crc16_ccitt_r(data, 0xffff, 0xffff), 0x906e);
    assert_eq!(calc_crc_modbus(data), 0x4b37);
    assert_eq!(crc_crc16_ibm(0, data), 0xfee8);
    assert_eq!(crc_crc24(data), 0xcde703);
    assert_eq!(crc_fletcher16(b"abcde"), 0xc8f0);
    let mut hash = FNV_1_OFFSET_BASIS_64;
    hash_fnv_1a(b"hello", &mut hash);
    assert_eq!(hash, 0xa430_d846_80aa_bd0b);
}

#[test]
fn empty_inputs_preserve_seeds_except_documented_output_xors() {
    assert_eq!(crc_crc4(&[0; 8]), 0);
    assert_eq!(crc_crc8(&[]), 0);
    assert_eq!(crc8_generic(&[], 0x97, 0xa5), 0xa5);
    assert_eq!(crc8_dvb_s2_update(0x56, &[]), 0x56);
    assert_eq!(crc8_dvb_update(0x83, &[]), 0x83);
    assert_eq!(crc8_maxim(&[]), 0);
    assert_eq!(crc8_sae(&[]), 0);
    assert_eq!(crc8_rds02uf(&[]), 0);
    assert_eq!(crc_xor_of_bytes(&[]), 0);
    assert_eq!(crc_xmodem(&[]), 0);
    assert_eq!(crc_crc32(0x12345678, &[]), 0x12345678);
    assert_eq!(crc32_small(0x12345678, &[]), 0x12345678);
    assert_eq!(crc_crc24(&[]), 0);
    assert_eq!(crc_crc16_ibm(0xabcd, &[]), 0xabcd);
    assert_eq!(crc_sum8_with_carry(&[]), 0xff);
    assert_eq!(crc16_ccitt(&[], 0x1234), 0x1234);
    assert_eq!(crc16_ccitt_r(&[], 0x1234, 0xffff), 0xedcb);
    assert_eq!(crc16_ccitt_gdl90(&[], 0x1234), 0x1234);
    assert_eq!(calc_crc_modbus(&[]), 0xffff);
    assert_eq!(crc_fletcher16(&[]), 0);
    let mut hash = 0x123456789abcdef0;
    hash_fnv_1a(&[], &mut hash);
    assert_eq!(hash, 0x123456789abcdef0);
    assert_eq!(crc_crc64(&[]), 0);
    assert_eq!(crc_sum_of_bytes(&[]), 0);
    assert_eq!(crc_sum_of_bytes_16(&[]), 0);
}

#[test]
fn byte_operations_and_nonstandard_tables_have_literal_checks() {
    assert_eq!(crc8_dvb_s2(0, 1), 0xd5);
    assert_eq!(crc8_dvb(0, 1, 7), 7);
    assert_eq!(crc_xmodem_update(0, 1), 0x1021);
    assert_eq!(crc8_rds02uf(&[0]), 0x93);
    assert_eq!(crc8_rds02uf(&[0xca]), 0x06); // Preserve upstream's duplicate entry.
    assert_eq!(crc16_ccitt_gdl90(&[1, 2], 0), 0x0102);
    assert_eq!(crc_xor_of_bytes(&[0xff, 0x10, 0x10]), 0xff);
    assert_eq!(crc_sum8_with_carry(&[0xff, 1]), 0xfe);
    assert_eq!(crc_sum_of_bytes(&[255, 255, 2]), 0);
    assert_eq!(crc_sum_of_bytes_16(&[255; 258]), 254); // Mod 65536, not 65535.
    for byte in 0..=255u8 {
        assert_eq!(parity(byte), (byte.count_ones() & 1) as u8);
    }
}

#[test]
fn streaming_updates_are_associative_and_reflected_output_xor_is_explicit() {
    let data: Vec<_> = (0..=255).cycle().take(4097).collect();
    for split in [0, 1, 255, 256, 4096, 4097] {
        let (a, b) = data.split_at(split);
        for seed in [0, 1, 0x5555_aaaa, u32::MAX] {
            assert_eq!(crc_crc32(crc_crc32(seed, a), b), crc_crc32(seed, &data));
            assert_eq!(
                crc32_small(crc32_small(seed, a), b),
                crc32_small(seed, &data)
            );
            assert_eq!(
                crc16_ccitt(b, crc16_ccitt(a, seed as u16)),
                crc16_ccitt(&data, seed as u16)
            );
            assert_eq!(
                crc16_ccitt_gdl90(b, crc16_ccitt_gdl90(a, seed as u16)),
                crc16_ccitt_gdl90(&data, seed as u16)
            );
            assert_eq!(
                crc_crc16_ibm(crc_crc16_ibm(seed as u16, a), b),
                crc_crc16_ibm(seed as u16, &data)
            );
            assert_eq!(
                crc8_dvb_update(crc8_dvb_update(seed as u8, a), b),
                crc8_dvb_update(seed as u8, &data)
            );
            assert_eq!(
                crc8_dvb_s2_update(crc8_dvb_s2_update(seed as u8, a), b),
                crc8_dvb_s2_update(seed as u8, &data)
            );
            assert_eq!(
                crc8_generic(b, 0x97, crc8_generic(a, 0x97, seed as u8)),
                crc8_generic(&data, 0x97, seed as u8)
            );
            assert_eq!(
                crc16_ccitt_r(b, crc16_ccitt_r(a, seed as u16, 0), 0xffff),
                crc16_ccitt_r(&data, seed as u16, 0xffff)
            );
        }
    }
    let mut whole = FNV_1_OFFSET_BASIS_64;
    hash_fnv_1a(&data, &mut whole);
    let mut split = FNV_1_OFFSET_BASIS_64;
    for part in data.chunks(31) {
        hash_fnv_1a(part, &mut split);
    }
    assert_eq!(whole, split);
}

#[test]
fn slice_lengths_extend_upstream_counters_without_truncation() {
    let mut data = vec![0; 255];
    data.push(1);
    assert_eq!(crc_crc8(&data), 7);
    assert_eq!(crc_sum8_with_carry(&data), 0xfe);
    for (length, sum) in [(65_535, 65_281), (65_536, 0), (65_537, 255)] {
        let bytes = vec![255; length];
        assert_eq!(crc_sum_of_bytes_16(&bytes), sum);
        assert_eq!(crc_sum_of_bytes(&bytes), sum as u8);
    }
}
