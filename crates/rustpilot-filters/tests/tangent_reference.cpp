// SPDX-License-Identifier: GPL-3.0-or-later
// The included unmodified reference is separately licensed under MIT.
#include <cfenv>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dlfcn.h>
#include <gnu/libc-version.h>
#include <xmmintrin.h>
#include <initializer_list>
#define __tanf rustpilot_pinned_tanf
#include "glibc_tanf.c"
#undef __tanf

static float from_bits(uint32_t bits) { float x; std::memcpy(&x, &bits, 4); return x; }

int main(int argc, char **argv) {
    if (std::fegetround() != FE_TONEAREST || (_mm_getcsr() & 0xe040) != 0) {
        std::fprintf(stderr, "requires FE_TONEAREST and MXCSR FTZ/DAZ clear\n");
        return 2;
    }
    std::fprintf(stderr, "reference glibc1848099f063e99d4ffecbd7667766d54862398b9; FE_TONEAREST; MXCSR=%08x\n", _mm_getcsr());
    float (*host_tanf)(float) = nullptr;
    const bool host = std::getenv("RUSTPILOT_TEST_HOST_GLIBC") != nullptr;
    if (host) {
        if (std::strcmp(std::getenv("RUSTPILOT_TEST_HOST_GLIBC"), "1") != 0 || std::strcmp(gnu_get_libc_version(), "2.44") != 0) {
            std::fprintf(stderr, "host comparison requires RUSTPILOT_TEST_HOST_GLIBC=1 and glibc2.44\n");
            return 3;
        }
        auto library = dlopen("libm.so.6", RTLD_NOW);
        host_tanf = reinterpret_cast<float (*)(float)>(dlsym(library, "tanf"));
        if (!host_tanf) return 4;
        Dl_info info{};
        if (!dladdr(reinterpret_cast<void *>(host_tanf), &info)) return 5;
        std::fprintf(stderr, "additional host comparison requested: glibc%s %s\n", gnu_get_libc_version(), info.dli_fname);
    } else {
        std::fprintf(stderr, "host libm comparison not requested; comparing pinned source only\n");
    }
    uint64_t count = 0;
    const bool tiny = argc == 2 && std::strcmp(argv[1], "--tiny") == 0;
    if (tiny) {
        for (uint32_t sign : {0u, 0x80000000u}) {
            for (uint32_t mantissa = 0; mantissa < (1u << 23); ++mantissa) {
                uint32_t bits = sign | (114u << 23) | mantissa;
                float x = from_bits(bits);
                if (asuint(rustpilot_pinned_tanf(x)) != bits || (host && asuint(host_tanf(x)) != bits)) {
                    std::fprintf(stderr, "tiny mismatch %08x\n", bits);
                    return 6;
                }
                ++count;
            }
        }
    } else {
        uint32_t bits;
        while (std::fread(&bits, sizeof(bits), 1, stdin) == 1) {
            float x = from_bits(bits);
            uint32_t expected = asuint(rustpilot_pinned_tanf(x));
            if (host && asuint(host_tanf(x)) != expected) {
                std::fprintf(stderr, "host/source mismatch input=%08x source=%08x host=%08x\n", bits, expected, asuint(host_tanf(x)));
                return 7;
            }
            if (std::fwrite(&expected, sizeof(expected), 1, stdout) != 1) return 8;
            ++count;
        }
        if (std::ferror(stdin)) return 9;
    }
    std::fprintf(stderr, "pinned source values=%llu; host libm values=%llu\n", static_cast<unsigned long long>(count), static_cast<unsigned long long>(host ? count : 0));
}
