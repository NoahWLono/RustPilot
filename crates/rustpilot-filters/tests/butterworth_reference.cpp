// SPDX-License-Identifier: GPL-3.0-or-later
// Test driver: all filtering is performed by the pinned, unmodified Butter.h.
#include <cstdint>
#include <cstring>
#include <iostream>
#include <tuple>
#include "Butter.h"

static uint32_t bits(float value) {
    uint32_t result;
    std::memcpy(&result, &value, sizeof(result));
    return result;
}
static float value(uint32_t bits) {
    float result;
    std::memcpy(&result, &bits, sizeof(result));
    return result;
}
template<class C> static void print_coefficients() {
    std::cout << bits(C::A1) << ' ' << bits(C::A2) << ' ' << bits(C::GAIN) << ' ';
}
using Filters = std::tuple<
    butter100hz0_25, butter50hz0_125, butter10hz0_025,
    butter100hz0_5, butter50hz0_25, butter10hz0_05,
    butter100hz1_0, butter50hz0_5, butter10hz0_1,
    butter100hz1_5, butter50hz0_75, butter10hz0_15,
    butter100hz2_0, butter50hz1_0, butter10hz0_2,
    butter100hz3_0, butter50hz1_5, butter10hz0_3,
    butter100hz4_0, butter50hz2_0, butter10hz0_4,
    butter100hz8_0, butter50hz4_0, butter10hz0_8,
    butter50hz8_0, butter10hz1_6>;

int main() {
    print_coefficients<butter100_025_coeffs>();
    print_coefficients<butter100_05_coeffs>();
    print_coefficients<butter100_1_coeffs>();
    print_coefficients<butter100_1_5_coeffs>();
    print_coefficients<butter100_2_coeffs>();
    print_coefficients<butter100_3_coeffs>();
    print_coefficients<butter100_4_coeffs>();
    print_coefficients<butter100_8_coeffs>();
    print_coefficients<butter50_8_coeffs>();
    std::cout << '\n';
    // Each tuple member is value-initialized, which zeroes the trivial Butter2
    // history before default initialization. No uninitialized stack state.
    Filters filters{};
    char operation;
    while (std::cin >> operation) {
        if (operation == 'r') {
            // Upstream has no reset. Compare Rust reset with a new zero-state
            // upstream instance, not with a modified C++ implementation.
            filters = Filters{};
        } else if (operation == 's') {
            uint32_t input;
            if (!(std::cin >> input)) return 2;
            std::apply([input](auto&... f) {
                ((std::cout << bits(f.filter(value(input))) << ' '), ...);
            }, filters);
            std::cout << '\n';
        } else {
            return 3;
        }
    }
    return std::cin.eof() ? 0 : 4;
}
