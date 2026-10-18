// SPDX-License-Identifier: GPL-3.0-or-later
// Driver context for actual pinned vector methods; algorithms live in a
// separate translation unit. Header-defined methods retain their real inlining.
#define HAL_DEBUG_BUILD 1
#include "AP_Math/AP_Math.h"
#include <cstdint>
#include <cstring>
#include <iostream>
#include <type_traits>
#include <cfenv>
#include <xmmintrin.h>
#include <gnu/libc-version.h>
template<class T> T value(uint64_t b) { T v; std::memcpy(&v,&b,sizeof(v)); return v; }
template<class T> uint64_t bits(T v) { uint64_t b=0; std::memcpy(&b,&v,sizeof(v)); return b; }
template<class V> void emit(const V& a, bool flag=false) { std::cout<<bits(a.x)<<' '<<bits(a.y)<<" 0 "<<flag<<'\n'; }
template<class T> void emit(const Vector3<T>& a, bool flag=false) { std::cout<<bits(a.x)<<' '<<bits(a.y)<<' '<<bits(a.z)<<' '<<flag<<'\n'; }
template<class T> void scalar(T v) { std::cout<<bits(v)<<" 0 0 0\n"; }
template<class T, template<class> class V> void run(char op, uint64_t x,uint64_t y,uint64_t z,uint64_t u,uint64_t v,uint64_t w,uint64_t s) {
    V<T> a,b;
    a.x=value<T>(x); a.y=value<T>(y); b.x=value<T>(u); b.y=value<T>(v);
    if constexpr (std::is_same_v<V<T>,Vector3<T>>) { a.z=value<T>(z); b.z=value<T>(w); }
    const T arg=value<T>(s);
    switch(op) {
    case '/': emit(a/arg); break;
    case '=': a/=arg; emit(a); break;
    case '*': scalar(a*b); break;
    case 'o': scalar(a.dot(b)); break;
    case 's': scalar(a.length_squared()); break;
    case 'l': scalar(a.length()); break;
    case 'm': a.normalize(); emit(a); break;
    case 'n': emit(a.normalized()); break;
    case 'k': {
        bool limited;
        if constexpr(std::is_same_v<V<T>,Vector3<T>>) limited=a.limit_length_xy(arg);
        else limited=a.limit_length(arg);
        emit(a,limited); break;
    }
    case 'd': if constexpr(std::is_same_v<V<T>,Vector3<T>>) scalar(a.distance_squared(b)); break;
    case 'f': emit(a.tofloat()); break;
    case 'g': emit(a.todouble()); break;
    }
}
int main() {
    const unsigned mxcsr=_mm_getcsr();
    std::cout << "vector magnitude reference GCC " << __VERSION__ << ", glibc " << gnu_get_libc_version() << ", rounding " << std::fegetround() << ", MXCSR " << mxcsr << ", EKF " << HAL_WITH_EKF_DOUBLE << ", ALLOW " << AP_MATH_ALLOW_DOUBLE_FUNCTIONS << "\n";
    if(std::fegetround()!=FE_TONEAREST || (mxcsr & ((1u<<15)|(1u<<6)))) return 2;
    unsigned storage,n; char op; uint64_t x,y,z,u,v,w,s;
    while(std::cin>>storage>>n>>op>>x>>y>>z>>u>>v>>w>>s) {
        if(storage==32) { if(n==2) run<float,Vector2>(op,x,y,z,u,v,w,s); else run<float,Vector3>(op,x,y,z,u,v,w,s); }
        else { if(n==2) run<double,Vector2>(op,x,y,z,u,v,w,s); else run<double,Vector3>(op,x,y,z,u,v,w,s); }
    }
}
