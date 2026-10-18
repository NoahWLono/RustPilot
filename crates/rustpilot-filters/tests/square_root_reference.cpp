// SPDX-License-Identifier: GPL-3.0-or-later
// Reference calls the same C++ sqrtf/sqrt operations used by pinned AP_Math.
// Test scaffolding, not copied implementation code.
#include <cfenv>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <gnu/libc-version.h>
#include <limits>
#include <xmmintrin.h>

#if !defined(__x86_64__)
#error This reference records the x86-64 sqrt NaN profile.
#endif

// Keep inputs opaque to constant folding, and test both direct and libm calls.
__attribute__((noinline)) static float root32(float value) { return ::sqrtf(value); }
__attribute__((noinline)) static double root64(double value) { return ::sqrt(value); }

static uint32_t result32(uint32_t bits) {
    float input;
    std::memcpy(&input, &bits, sizeof(input));
    const float result = root32(input);
    std::memcpy(&bits, &result, sizeof(result));
    return bits;
}

static uint64_t result64(uint64_t bits) {
    double input;
    std::memcpy(&input, &bits, sizeof(input));
    const double result = root64(input);
    std::memcpy(&bits, &result, sizeof(result));
    return bits;
}

int main() {
    static_assert(sizeof(float) == 4 && sizeof(double) == 8);
    static_assert(std::numeric_limits<float>::is_iec559);
    static_assert(std::numeric_limits<double>::is_iec559);
    if (std::fegetround() != FE_TONEAREST || (_mm_getcsr() & 0xe040) != 0) {
        std::fprintf(stderr, "requires FE_TONEAREST and MXCSR FTZ/DAZ clear\n");
        return 2;
    }
    std::printf("profile|compiler=%s|glibc=%s|round=FE_TONEAREST|mxcsr=%08x|ftz=0|daz=0\n",
                __VERSION__, gnu_get_libc_version(), _mm_getcsr());
    char kind;
    unsigned long long bits;
    while (std::scanf(" %c %llx", &kind, &bits) == 2) {
        if (kind == 'f') {
            std::printf("%08x\n", result32(static_cast<uint32_t>(bits)));
        } else if (kind == 'd') {
            std::printf("%016llx\n", static_cast<unsigned long long>(result64(bits)));
        } else if (kind == 'r' && bits == 0) {
            // All positive subnormals, followed by all significands of [1,4).
            for (uint32_t x = 1; x < 0x00800000; ++x) {
                std::printf("%08x\n", result32(x));
            }
            for (uint32_t x = 0x3f800000; x < 0x40800000; ++x) {
                std::printf("%08x\n", result32(x));
            }
        } else if (kind == 'n' && bits == 0) {
            for (uint32_t payload = 1; payload < 0x00800000; ++payload) {
                std::printf("%08x\n", result32(0x7f800000 | payload));
                std::printf("%08x\n", result32(0xff800000 | payload));
            }
        } else {
            return 3;
        }
    }
    return std::ferror(stdin) || std::ferror(stdout) ? 4 : 0;
}
