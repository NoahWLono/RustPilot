// SPDX-License-Identifier: GPL-3.0-or-later
// Test driver only. LowPassFilter implementations compile from upstream source.
#include <LowPassFilter.h>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <string>
static float decode(uint32_t bits) { float f; std::memcpy(&f, &bits, 4); return f; }
static void emit(float f) { uint32_t bits; std::memcpy(&bits, &f, 4); std::cout << bits << '\n'; }
class Digital : public DigitalLPF<float> {
public: float apply(float s, float a) { return _apply(s,a); }
};
int main() {
    Digital d;
    LowPassFilter<float> v(2.0f);
    LowPassFilterConstDt<float> c(50.0f,2.0f);
    // Constructors do not initialize all automatic-storage state upstream.
    d.reset(); v.reset(); c.reset();
    std::string op;
    uint32_t a,b;
    while (std::cin >> op) {
        if (op == "a") { std::cin >> a >> b; emit(calc_lowpass_alpha_dt(decode(a),decode(b))); }
        else if (op == "d") { std::cin >> a >> b; emit(d.apply(decode(a),decode(b))); }
        else if (op == "dr") { d.reset(); }
        else if (op == "ds") { std::cin >> a; d.reset(decode(a)); }
        else if (op == "v") { std::cin >> a >> b; emit(v.apply(decode(a),decode(b))); }
        else if (op == "vr") { v.reset(); }
        else if (op == "vs") { std::cin >> a; v.reset(decode(a)); }
        else if (op == "vc") { std::cin >> a; v.set_cutoff_frequency(decode(a)); }
        else if (op == "c") { std::cin >> a; emit(c.apply(decode(a))); }
        else if (op == "cr") { c.reset(); }
        else if (op == "cs") { std::cin >> a; c.reset(decode(a)); }
        else if (op == "cc") { std::cin >> a >> b; c.set_cutoff_frequency(decode(a),decode(b)); }
        else return 2;
    }
}
