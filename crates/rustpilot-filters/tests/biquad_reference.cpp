// SPDX-License-Identifier: GPL-3.0-or-later
#define HAL_DEBUG_BUILD 1
#include <cstring>
#include <cfenv>
#include <stdexcept>
#include <iostream>
#include <type_traits>
#if defined(__x86_64__) || defined(__i386__)
#include <xmmintrin.h>
#endif
#include "AP_Math/AP_Math.h"
#include "LowPassFilter2p.h"

static float value(uint32_t bits) { float result; std::memcpy(&result, &bits, sizeof(result)); return result; }
static uint32_t bits(float input) { uint32_t result; std::memcpy(&result, &input, sizeof(result)); return result; }

template<typename T> struct Inspectable : LowPassFilter2p<T> {
    using LowPassFilter2p<T>::_params;
};
// Static storage defines initialised=false, as in upstream global firmware objects.
static Inspectable<float> scalar;
static Inspectable<int> integer;
static Inspectable<long> long_integer;
static Inspectable<Vector2f> vector2;
static Inspectable<Vector3f> vector3;
template<typename T> struct Direct {
    DigitalBiquadFilter<T> filter;
    typename DigitalBiquadFilter<T>::biquad_params params{10, 100, 0, 0, 0, 0, 0};
};
static Direct<float> direct_scalar;
static Direct<int> direct_integer;
static Direct<long> direct_long;
static Direct<Vector2f> direct_vector2;
static Direct<Vector3f> direct_vector3;
static DigitalBiquadFilter<float>::biquad_params standalone{1, 2, 3, 4, 5, 6, 7};

static void output(float v) { std::cout << bits(v); }
static void output(int v) { std::cout << static_cast<uint64_t>(v); }
static void output(long v) { std::cout << static_cast<uint64_t>(v); }
static void output(const Vector2f& v) { std::cout << bits(v.x) << ' ' << bits(v.y); }
static void output(const Vector3f& v) { std::cout << bits(v.x) << ' ' << bits(v.y) << ' ' << bits(v.z); }
template<typename P> static void parameters(const P& p) {
    std::cout << bits(p.cutoff_freq) << ' ' << bits(p.sample_freq) << ' ' << bits(p.a1) << ' ' << bits(p.a2) << ' ' << bits(p.b0) << ' ' << bits(p.b1) << ' ' << bits(p.b2) << ' ';
}
template<typename T> static void step(Inspectable<T>& model, char op, const T& sample, float rate, float cutoff) {
    T result{};
    if (op == 'A') result = model.apply(sample);
    if (op == 'S') model.reset(sample);
    if (op == 'R') model.reset();
    if (op == 'C') model.set_cutoff_frequency(rate, cutoff);
    parameters(model._params);
    output(result);
    std::cout << '\n';
}
template<typename T> static void direct_step(Direct<T>& model, char op, const T& sample, int64_t x, int64_t y, int64_t z, float rate, float cutoff) {
    T result{};
    if (op == 'A') result = model.filter.apply(sample, model.params);
    if (op == 'S') model.filter.reset(sample, model.params);
    if (op == 'R') model.filter.reset();
    if (op == 'C') DigitalBiquadFilter<T>::compute_params(rate, cutoff, model.params);
    if (op == 'B') model.params = {10, 100, value(static_cast<uint32_t>(x)), value(static_cast<uint32_t>(y)), value(static_cast<uint32_t>(z)), rate, cutoff};
    parameters(model.params);
    output(result);
    std::cout << '\n';
}
int main() {
    static_assert(sizeof(int) == 4 && sizeof(long) == 8, "reference integer aliases require the recorded LP64 ABI");
    if (std::fegetround() != FE_TONEAREST) throw std::runtime_error("reference requires FE_TONEAREST");
#if defined(__x86_64__) || defined(__i386__)
    if ((_mm_getcsr() & 0xe040u) != 0) throw std::runtime_error("reference requires MXCSR RNE, FTZ=0, DAZ=0");
#endif
    volatile float cosine_argument = static_cast<float>(M_PI / 4.0f);
    float (*volatile cosine_call)(float) = ::cosf;
    if (bits(cosine_call(cosine_argument)) != 0x3f3504f3u || bits(cosf(M_PI/4.0f)) != 0x3f3504f3u) throw std::runtime_error("fixed cosine bit pattern differs");
    unsigned kind;
    char op;
    int64_t x, y, z;
    uint32_t rate, cutoff;
    while (std::cin >> kind >> op >> x >> y >> z >> rate >> cutoff) {
        switch (kind) {
        case 0: step(scalar, op, value(static_cast<uint32_t>(x)), value(rate), value(cutoff)); break;
        case 1: step(integer, op, static_cast<int>(x), value(rate), value(cutoff)); break;
        case 2: step(long_integer, op, static_cast<long>(x), value(rate), value(cutoff)); break;
        case 3: step(vector2, op, Vector2f{value(static_cast<uint32_t>(x)), value(static_cast<uint32_t>(y))}, value(rate), value(cutoff)); break;
        case 4: step(vector3, op, Vector3f{value(static_cast<uint32_t>(x)), value(static_cast<uint32_t>(y)), value(static_cast<uint32_t>(z))}, value(rate), value(cutoff)); break;
        case 5: DigitalBiquadFilter<float>::compute_params(value(rate), value(cutoff), standalone); parameters(standalone); std::cout << 0 << '\n'; break;
        case 6: direct_step(direct_scalar, op, value(static_cast<uint32_t>(x)), x, y, z, value(rate), value(cutoff)); break;
        case 7: direct_step(direct_integer, op, static_cast<int>(x), x, y, z, value(rate), value(cutoff)); break;
        case 8: direct_step(direct_long, op, static_cast<long>(x), x, y, z, value(rate), value(cutoff)); break;
        case 9: direct_step(direct_vector2, op, Vector2f{value(static_cast<uint32_t>(x)), value(static_cast<uint32_t>(y))}, x, y, z, value(rate), value(cutoff)); break;
        case 10: direct_step(direct_vector3, op, Vector3f{value(static_cast<uint32_t>(x)), value(static_cast<uint32_t>(y)), value(static_cast<uint32_t>(z))}, x, y, z, value(rate), value(cutoff)); break;
        default: throw std::runtime_error("invalid model");
        }
    }
}
