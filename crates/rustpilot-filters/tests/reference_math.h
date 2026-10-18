// SPDX-License-Identifier: GPL-3.0-or-later
// Minimal AP_Math replacement for scalar reference compilation only.
#pragma once
#include <cfloat>
#include <cmath>
#define CLASS_NO_COPY(name) name(const name &) = delete; name &operator=(const name &) = delete
#undef M_PI
#define M_PI (3.141592653589793238462643383279502884)
#define M_2PI (M_PI * 2)
inline bool is_negative(float x) { return x <= -FLT_EPSILON; }
inline bool is_zero(float x) { return std::fabs(x) < FLT_EPSILON; }
#define INTERNAL_ERROR(...) ((void)0)
float calc_lowpass_alpha_dt(float, float);
// Placeholder vectors satisfy upstream explicit template instantiations.
// Their behavior is irrelevant to scalar tests and is NOT vector validation.
template<int N> struct ReferenceVector {
    float v = 0;
    ReferenceVector operator-(const ReferenceVector &x) const { return {v-x.v}; }
    ReferenceVector operator*(float x) const { return {v*x}; }
    ReferenceVector &operator+=(const ReferenceVector &x) { v += x.v; return *this; }
};
using Vector2f = ReferenceVector<2>;
using Vector3f = ReferenceVector<3>;
