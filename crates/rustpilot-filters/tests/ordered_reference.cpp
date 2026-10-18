// SPDX-License-Identifier: GPL-3.0-or-later
// Test I/O only: all filter algorithms below come from verified upstream files.
#include <cassert>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <string>
#include <type_traits>
#include "ModeFilter.cpp"
#include "DerivativeFilter.cpp"

template<class T> T read_sample() {
    if constexpr (std::is_same<T, float>::value) {
        uint32_t bits; std::cin >> bits;
        float value; std::memcpy(&value, &bits, sizeof(value)); return value;
    } else {
        int64_t value; std::cin >> value; return static_cast<T>(value);
    }
}

template<class T> void emit(T value) {
    if constexpr (std::is_same<T, float>::value) {
        uint32_t bits; std::memcpy(&bits, &value, sizeof(bits)); std::cout << bits << ' ';
    } else { std::cout << static_cast<int64_t>(value) << ' '; }
}

template<class T, uint8_t N> void run_mode(uint8_t rank) {
    // Static storage guarantees zero initialization before the upstream ctor.
    // Each invocation of this executable handles one filter instance.
    static ModeFilter<T, N> filter(rank);
    std::cout << static_cast<unsigned>(filter.get_filter_size()) << '\n';
    std::string operation;
    while (std::cin >> operation) {
        if (operation == "a") {
            T sample = read_sample<T>();
            const T output = filter.apply(sample);
            if constexpr (std::is_same<T,float>::value) {
                assert((std::isnan(output) && std::isnan(filter.get())) || output == filter.get());
            } else { assert(output == filter.get()); }
        } else if (operation == "r") { filter.reset(); }
        else if (operation != "q") { std::abort(); }
        emit(filter.get());
        for (unsigned i = 0; i < N; ++i) { emit(filter.get_sample(i)); }
        std::cout << '\n';
    }
}

template<class T> void select_mode(unsigned size, uint8_t rank) {
    switch (size) {
    case 1: run_mode<T,1>(rank); break;
    case 3: run_mode<T,3>(rank); break;
    case 4: run_mode<T,4>(rank); break;
    case 5: run_mode<T,5>(rank); break;
    case 6: run_mode<T,6>(rank); break;
    case 7: run_mode<T,7>(rank); break;
    case 127: run_mode<T,127>(rank); break;
    default: std::abort();
    }
}

template<uint8_t N> void run_derivative() {
    static DerivativeFilter<float,N> filter;
    std::cout << static_cast<unsigned>(filter.get_filter_size()) << '\n';
    std::string operation;
    while (std::cin >> operation) {
        if (operation == "u") {
            float value = read_sample<float>(); uint32_t timestamp; std::cin >> timestamp;
            filter.update(value, timestamp);
        } else if (operation == "a") {
            float value = read_sample<float>(); emit(filter.apply(value));
        } else if (operation == "r") { filter.reset(); }
        else if (operation != "q") { std::abort(); }
        emit(filter.slope()); emit(filter.slope());
        for (unsigned i = 0; i < N; ++i) { emit(filter.get_sample(i)); }
        std::cout << '\n';
    }
}

int main() {
    std::string family; std::cin >> family;
    if (family == "mode") {
        std::string type; unsigned size, rank; std::cin >> type >> size >> rank;
        if (type == "i8") select_mode<int8_t>(size, rank);
        else if (type == "u8") select_mode<uint8_t>(size, rank);
        else if (type == "i16") select_mode<int16_t>(size, rank);
        else if (type == "u16") select_mode<uint16_t>(size, rank);
        else if (type == "f32") select_mode<float>(size, rank);
        else std::abort();
    } else if (family == "derivative") {
        unsigned size; std::cin >> size;
        switch(size) {
        case 5: run_derivative<5>(); break;
        case 7: run_derivative<7>(); break;
        case 9: run_derivative<9>(); break;
        case 11: run_derivative<11>(); break;
        default: std::abort();
        }
    } else std::abort();
}
