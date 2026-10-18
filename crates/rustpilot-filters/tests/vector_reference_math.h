// SPDX-License-Identifier: GPL-3.0-or-later
// Build shim: actual pinned vector declarations and separately extracted
// arithmetic bodies are compiled. No vector algorithm is mocked here.
#pragma once
#include <cstdint>
#include <cfloat>
#include <cmath>
#define CLASS_NO_COPY(name) name(const name &) = delete; name &operator=(const name &) = delete
#define WARN_IF_UNUSED
#undef M_PI
#define M_PI (3.141592653589793238462643383279502884)
#define M_2PI (M_PI * 2)
inline bool is_negative(float x) { return x <= -FLT_EPSILON; }
#define INTERNAL_ERROR(...) ((void)0)
#include "vector2.h"
#include "vector3.h"
float calc_lowpass_alpha_dt(float, float);
