// SPDX-License-Identifier: GPL-3.0-or-later
// Test protocol only. Every checksum below is implemented by actual upstream crc.cpp.
#include <cstdint>
#include <fstream>
#include <iostream>
#include <vector>
#include "crc.h"

static uint64_t read_le(std::istream &in, unsigned bytes) {
    uint64_t value = 0;
    for (unsigned i = 0; i < bytes; ++i) {
        const int byte = in.get();
        if (byte == EOF) { std::cerr << "truncated fixture"; std::exit(2); }
        value |= uint64_t(uint8_t(byte)) << (8 * i);
    }
    return value;
}
int main(int argc, char **argv) {
    if (argc != 2) { return 2; }
    std::ifstream input(argv[1], std::ios::binary);
    if (!input) { return 2; }
    const auto count = read_le(input, 4);
    for (uint64_t c = 0; c < count; ++c) {
        const auto n = read_le(input, 4);
        const auto seed = read_le(input, 8);
        // All byte fixtures fit upstream uint16_t, and short functions use <=255.
        if (n > 65535) { return 2; }
        std::vector<uint8_t> data(n + 1, 0);
        for (uint64_t i = 0; i < n; ++i) { data[i] = read_le(input, 1); }
        const auto nw = read_le(input, 2);
        std::vector<uint32_t> words(nw + 1, 0);
        for (uint64_t i = 0; i < nw; ++i) { words[i] = read_le(input, 4); }
        uint16_t prom[8];
        for (auto &word : prom) { word = read_le(input, 2); }
        auto *p = data.data();
        const auto short_n = uint8_t(n < 255 ? n : 255);
        uint64_t hash = seed;
        hash_fnv_1a(n, p, &hash);
        const uint64_t values[] = {
            crc_crc4(prom), crc_crc8(p, short_n),
            crc8_generic(p, n, uint8_t(seed >> 8), seed),
            crc8_dvb_s2(seed, p[0]), crc8_dvb(seed, p[0], seed >> 8),
            crc8_dvb_s2_update(seed, p, n), crc8_dvb_update(seed, p, n),
            crc8_maxim(p, n), crc8_sae(p, n), crc8_rds02uf(p, n),
            crc_xor_of_bytes(p, n), crc_xmodem_update(seed, p[0]),
            crc_xmodem(p, n), crc_crc32(seed, p, n), crc32_small(seed, p, n),
            crc_crc24(p, n), crc_crc16_ibm(seed, p, n),
            crc_sum8_with_carry(p, short_n), crc16_ccitt(p, n, seed),
            crc16_ccitt_r(p, n, seed, seed >> 16), crc16_ccitt_GDL90(p, n, seed),
            calc_crc_modbus(p, n), crc_fletcher16(p, n), hash,
            crc_crc64(words.data(), nw), parity(p[0]), crc_sum_of_bytes(p, n),
            crc_sum_of_bytes_16(p, n)
        };
        for (auto v : values) { std::cout << v << ' '; }
        std::cout << '\n';
    }
    return input.peek() == EOF ? 0 : 2;
}
