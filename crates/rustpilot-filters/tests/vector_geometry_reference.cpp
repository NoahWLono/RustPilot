// SPDX-License-Identifier: GPL-3.0-or-later
// Actual pinned vector declarations; Vector2 bodies and arithmetic operators
// are compiled separately. Vector3 methods retain their header-defined context.
#define HAL_DEBUG_BUILD 1
#include "AP_Math/AP_Math.h"
#include <cstring>
#include <iostream>
#include <type_traits>
#include <cfenv>
#include <xmmintrin.h>
#include <gnu/libc-version.h>
template<class T> T value(uint64_t b) { T v; std::memcpy(&v,&b,sizeof(v)); return v; }
template<class T> uint64_t bits(T v) { uint64_t b=0; std::memcpy(&b,&v,sizeof(v)); return b; }
template<class T, template<class> class V> void run(char op,uint64_t x,uint64_t y,uint64_t z,uint64_t u,uint64_t v,uint64_t w) {
    V<T> a,b;
    a.x=value<T>(x); a.y=value<T>(y); b.x=value<T>(u); b.y=value<T>(v);
    if constexpr(std::is_same_v<V<T>,Vector3<T>>) { a.z=value<T>(z); b.z=value<T>(w); }
    switch(op) {
    case 'p': a.project(b); break;
    case 'P': a=a.projected(b); break;
    case 'r': a.reflect(b); break;
    case 'n': a=V<T>::perpendicular(a,b); break;
    case 's': a.project(a); break;
    case 't': a.reflect(a); break;
    }
    std::cout<<bits(a.x)<<' '<<bits(a.y)<<' ';
    if constexpr(std::is_same_v<V<T>,Vector3<T>>) std::cout<<bits(a.z);
    else std::cout<<0;
    std::cout<<'\n';
}
int main() {
    const unsigned mxcsr=_mm_getcsr();
    std::cout<<"vector geometry reference GCC "<<__VERSION__<<", glibc "<<gnu_get_libc_version()<<", rounding "<<std::fegetround()<<", MXCSR "<<mxcsr<<", EKF "<<HAL_WITH_EKF_DOUBLE<<", ALLOW "<<AP_MATH_ALLOW_DOUBLE_FUNCTIONS<<'\n';
    if(std::fegetround()!=FE_TONEAREST || (mxcsr & 0xe040u)) return 2;
    unsigned storage,n; char op; uint64_t x,y,z,u,v,w;
    while(std::cin>>storage>>n>>op>>x>>y>>z>>u>>v>>w) {
        if(storage==32) { if(n==2) run<float,Vector2>(op,x,y,z,u,v,w); else run<float,Vector3>(op,x,y,z,u,v,w); }
        else { if(n==2) run<double,Vector2>(op,x,y,z,u,v,w); else run<double,Vector3>(op,x,y,z,u,v,w); }
    }
}
