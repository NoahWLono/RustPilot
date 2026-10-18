// SPDX-License-Identifier: GPL-3.0-or-later
// Test driver for unmodified, pinned ArduPilot headers. No algorithm shims.
#include "FilterWithBuffer.h"
#include "AverageFilter.h"
#include <cmath>
#include <cstring>
#include <iostream>
#include <string>
#include <type_traits>

template<class T> T decode(int64_t raw) {
    if constexpr (std::is_same_v<T,float>) {
        uint32_t bits = static_cast<uint32_t>(raw);
        T value;
        std::memcpy(&value, &bits, sizeof(value));
        return value;
    } else {
        return static_cast<T>(raw);
    }
}
template<class T> void emit(T value) {
    if constexpr (std::is_same_v<T,float>) {
        if (std::isnan(value)) { std::cout << "nan "; return; }
        uint32_t bits;
        std::memcpy(&bits, &value, sizeof(bits));
        std::cout << bits << ' ';
    } else if constexpr (std::is_same_v<T,double>) {
        if (std::isnan(value)) { std::cout << "nan "; return; }
        uint64_t bits;
        std::memcpy(&bits, &value, sizeof(bits));
        std::cout << bits << ' ';
    } else {
        std::cout << static_cast<int64_t>(value) << ' ';
    }
}

template<class T, uint8_t N> void buffer_case(size_t length) {
    FilterWithBuffer<T,N> filter;
    for (size_t i=0; i<length; ++i) {
        char operation; int64_t raw;
        std::cin >> operation >> raw;
        T result{};
        if (operation == 'R') { filter.reset(); } else { result=filter.apply(decode<T>(raw)); }
        emit(result);
        for (unsigned slot=0; slot<N; ++slot) { emit(filter.get_sample(slot)); }
        std::cout << '\n';
    }
}
template<class T, class U, uint8_t N> void average_case(size_t length) {
    AverageFilter<T,U,N> filter;
    for (size_t i=0; i<length; ++i) {
        char operation; int64_t raw;
        std::cin >> operation >> raw;
        T result{};
        if (operation == 'R') { filter.reset(); } else { result=filter.apply(decode<T>(raw)); }
        emit(result);
        for (unsigned slot=0; slot<N; ++slot) { emit(filter.get_sample(slot)); }
        std::cout << '\n';
    }
}
template<class T, class U, uint8_t N> void integral_case(size_t length) {
    AverageIntegralFilter<T,U,N> filter;
    for (size_t i=0; i<length; ++i) {
        char operation; int64_t raw;
        std::cin >> operation >> raw;
        T result{};
        if (operation == 'R') { filter.reset(); } else { result=filter.apply(decode<T>(raw)); }
        emit(result); emit(filter.getf()); emit(filter.getd());
        for (unsigned slot=0; slot<N; ++slot) { emit(filter.get_sample(slot)); }
        std::cout << '\n';
    }
}

#define BUFFER_ONE(TAG,T,N) if (name=="b_" TAG "_" #N) { buffer_case<T,N>(length); continue; }
#define BUFFER_FAMILY(TAG,T) BUFFER_ONE(TAG,T,2) BUFFER_ONE(TAG,T,3) BUFFER_ONE(TAG,T,4) BUFFER_ONE(TAG,T,5) BUFFER_ONE(TAG,T,6) BUFFER_ONE(TAG,T,7)
#define AVERAGE_ONE(TAG,T,U,N) if (name=="a_" TAG "_" #N) { average_case<T,U,N>(length); continue; }
#define AVERAGE_FAMILY(TAG,T,U) AVERAGE_ONE(TAG,T,U,2) AVERAGE_ONE(TAG,T,U,3) AVERAGE_ONE(TAG,T,U,4) AVERAGE_ONE(TAG,T,U,5)
#define INTEGRAL_ONE(TAG,T,U,N) if (name=="i_" TAG "_" #N) { integral_case<T,U,N>(length); continue; }
int main() {
    std::string name;
    size_t length;
    while (std::cin >> name >> length) {
        BUFFER_FAMILY("i16",int16_t)
        BUFFER_FAMILY("u16",uint16_t)
        BUFFER_FAMILY("i32",int32_t)
        BUFFER_FAMILY("u32",uint32_t)
        BUFFER_ONE("i32",int32_t,1)
        BUFFER_ONE("i32",int32_t,127)
        AVERAGE_FAMILY("i8i16",int8_t,int16_t)
        AVERAGE_FAMILY("u8u16",uint8_t,uint16_t)
        AVERAGE_FAMILY("i16i32",int16_t,int32_t)
        AVERAGE_FAMILY("u16u32",uint16_t,uint32_t)
        AVERAGE_FAMILY("i32f32",int32_t,float)
        AVERAGE_FAMILY("u32f32",uint32_t,float)
        AVERAGE_ONE("f32f32",float,float,5)
        AVERAGE_ONE("i32i32",int32_t,int32_t,10)
        AVERAGE_ONE("u32u32",uint32_t,uint32_t,5)
        AVERAGE_ONE("u8u8",uint8_t,uint8_t,3)
        AVERAGE_ONE("u16u16",uint16_t,uint16_t,3)
        AVERAGE_ONE("i8i8",int8_t,int8_t,3)
        AVERAGE_ONE("i16i16",int16_t,int16_t,3)
        AVERAGE_ONE("i8i32",int8_t,int32_t,3)
        AVERAGE_ONE("u8u32",uint8_t,uint32_t,3)
        INTEGRAL_ONE("i8i8",int8_t,int8_t,3)
        INTEGRAL_ONE("i8i16",int8_t,int16_t,3)
        INTEGRAL_ONE("i8i32",int8_t,int32_t,3)
        INTEGRAL_ONE("u8u8",uint8_t,uint8_t,3)
        INTEGRAL_ONE("u8u16",uint8_t,uint16_t,3)
        INTEGRAL_ONE("u8u32",uint8_t,uint32_t,3)
        INTEGRAL_ONE("i16i16",int16_t,int16_t,3)
        INTEGRAL_ONE("i16i32",int16_t,int32_t,3)
        INTEGRAL_ONE("u16u16",uint16_t,uint16_t,3)
        INTEGRAL_ONE("u16u32",uint16_t,uint32_t,3)
        INTEGRAL_ONE("i32i32",int32_t,int32_t,10)
        INTEGRAL_ONE("u32u32",uint32_t,uint32_t,5)
        std::cerr << "unknown case " << name << '\n';
        return 1;
    }
}
