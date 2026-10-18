// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cmath>
#include <cstdint>
#include <cstring>
#ifndef __glibc_likely
#define __glibc_likely(x) (x)
#endif
#ifndef __glibc_unlikely
#define __glibc_unlikely(x) (x)
#endif
static uint32_t asuint(float x) { uint32_t bits; std::memcpy(&bits, &x, 4); return bits; }
static uint64_t asuint64(double x) { uint64_t bits; std::memcpy(&bits, &x, 8); return bits; }
// The driver enforces FE_TONEAREST; under that mode nearbyint rounds ties to even.
static double roundeven_finite(double x) { return std::nearbyint(x); }
// Output expression matches glibc; C errno side effects are outside this harness.
static float __math_invalidf(float x) { return (x - x) / (x - x); }
