// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
using u128 = __uint128_t;
static u128 u128_from_u64(uint64_t x) { return x; }
static u128 u128_mul(u128 a, u128 b) { return a * b; }
static u128 u128_add(u128 a, u128 b) { return a + b; }
static u128 u128_rshift(u128 x, int shift) { return x >> shift; }
static uint64_t u128_high(u128 x) { return x >> 64; }
static uint64_t u128_low(u128 x) { return x; }
