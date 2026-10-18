// SPDX-License-Identifier: GPL-3.0-or-later
#define HAL_DEBUG_BUILD 1
#include <cstring>
#include <cfenv>
#include <stdexcept>
#if defined(__x86_64__) || defined(__i386__)
#include <xmmintrin.h>
#endif
#include <iostream>
#include "AP_Math/AP_Math.h"
#include "LowPassFilter.h"

static float value(uint32_t bits) { float result; std::memcpy(&result, &bits, sizeof(result)); return result; }
static uint32_t bits(float input) { uint32_t result; std::memcpy(&result, &input, sizeof(result)); return result; }
template<typename T> struct Digital : DigitalLPF<T> { using DigitalLPF<T>::_apply; };
template<typename T> struct Models {
    Digital<T> digital;
    LowPassFilter<T> variable{0.0f};
    LowPassFilterConstDt<T> fixed{0.0f, 0.0f};
};
// Static storage initializes the upstream otherwise-uninitialized bool.
static Models<Vector2f> vector2;
static Models<Vector3f> vector3;
template<typename T> static T step(Models<T>& model, unsigned kind, char op, const T& sample, float arg, float rate, float& cutoff) {
    if (kind == 3) {
        cutoff = 0.0f;
        if (op == '+') return sample + model.digital.get();
        if (op == '-') return sample - model.digital.get();
        if (op == '*') return sample * arg;
        if (op == '=') { T result = sample; result += model.digital.get(); return result; }
        throw std::runtime_error("unknown vector operation");
    }
    if (kind == 0) {
        if (op == 'A') model.digital._apply(sample, arg);
        if (op == 'R') model.digital.reset();
        if (op == 'S') model.digital.reset(sample);
        cutoff = 0.0f;
        return model.digital.get();
    }
    if (kind == 1) {
        if (op == 'A') model.variable.apply(sample, arg);
        if (op == 'R') model.variable.reset();
        if (op == 'S') model.variable.reset(sample);
        if (op == 'C') model.variable.set_cutoff_frequency(arg);
        cutoff = model.variable.get_cutoff_freq();
        return model.variable.get();
    }
    if (op == 'A') model.fixed.apply(sample);
    if (op == 'R') model.fixed.reset();
    if (op == 'S') model.fixed.reset(sample);
    if (op == 'C') model.fixed.set_cutoff_frequency(rate, arg);
    cutoff = model.fixed.get_cutoff_freq();
    return model.fixed.get();
}
int main() {
    if (std::fegetround() != FE_TONEAREST) throw std::runtime_error("reference requires FE_TONEAREST");
#if defined(__x86_64__) || defined(__i386__)
    if ((_mm_getcsr() & 0xe040u) != 0) throw std::runtime_error("reference requires MXCSR round-to-nearest, FTZ=0, DAZ=0");
#endif
    unsigned dimension, kind;
    char op;
    uint32_t x, y, z, arg, rate;
    while (std::cin >> dimension >> kind >> op >> x >> y >> z >> arg >> rate) {
        float cutoff;
        if (dimension == 2) {
            auto result = step(vector2, kind, op, Vector2f{value(x), value(y)}, value(arg), value(rate), cutoff);
            std::cout << bits(result.x) << ' ' << bits(result.y) << ' ' << bits(cutoff) << '\n';
        } else {
            auto result = step(vector3, kind, op, Vector3f{value(x), value(y), value(z)}, value(arg), value(rate), cutoff);
            std::cout << bits(result.x) << ' ' << bits(result.y) << ' ' << bits(result.z) << ' ' << bits(cutoff) << '\n';
        }
    }
}
