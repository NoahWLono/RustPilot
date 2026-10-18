// SPDX-License-Identifier: GPL-3.0-or-later
// The actual pinned headers supply inline methods. The out-of-line vector
// methods and AP_Math equality helper are in separate translation units.
#define HAL_DEBUG_BUILD 1
#include "AP_Math/AP_Math.h"
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <type_traits>
#include <cfenv>
#include <xmmintrin.h>
#include <gnu/libc-version.h>

template<class T> T value(uint64_t b) { T v; std::memcpy(&v,&b,sizeof(v)); return v; }
template<class T> uint64_t bits(T v) { uint64_t b=0; std::memcpy(&b,&v,sizeof(v)); return b; }
template<class T> void emit(const Vector2<T>& v) { std::cout<<' '<<bits(v.x)<<' '<<bits(v.y); }
template<class T> void emit(const Vector3<T>& v) { std::cout<<' '<<bits(v.x)<<' '<<bits(v.y)<<' '<<bits(v.z); }

template<class T, template<class> class V> void run(uint64_t x,uint64_t y,uint64_t z,uint64_t u,uint64_t v,uint64_t w) {
    using Vec=V<T>;
    constexpr bool three=std::is_same_v<Vec,Vector3<T>>;
    constexpr unsigned n=three ? 3 : 2;
    // These assertions establish the measured host layout only. Pointer
    // arithmetic across scalar members remains outside portable ISO C++.
    static_assert(std::is_standard_layout_v<Vec>);
    static_assert(sizeof(Vec)==n*sizeof(T));
    static_assert(offsetof(Vec,x)==0);
    static_assert(offsetof(Vec,y)==sizeof(T));
    if constexpr(three) { static_assert(offsetof(Vec,z)==2*sizeof(T)); }
    Vec a,b;
    a.x=value<T>(x); a.y=value<T>(y); b.x=value<T>(u); b.y=value<T>(v);
    if constexpr(three) { a.z=value<T>(z); b.z=value<T>(w); }
    std::cout<<a.is_nan()<<' '<<a.is_inf()<<' '<<a.is_zero()<<' '<<(a==b)<<' '<<(a!=b);
    const Vec& ca=a;
    for(unsigned i=0;i<n;i++) std::cout<<' '<<bits(ca[i]);
    for(unsigned i=0;i<n;i++) std::cout<<' '<<bits(a[i]);
    emit(a);
    const T replacements[3]={value<T>(u),value<T>(v),value<T>(w)};
    for(unsigned i=0;i<n;i++) { T& slot=a[i]; slot=replacements[i]; emit(a); }
    a.zero(); emit(a);
    const Vec zero; emit(zero);
    if constexpr(three) { emit(Vector3<T>(Vector2<T>(value<T>(x),value<T>(y)),value<T>(z))); }
    std::cout<<'\n';
}

int main() {
    const unsigned mxcsr=_mm_getcsr();
    std::cout<<"vector predicate reference GCC "<<__VERSION__<<", glibc "<<gnu_get_libc_version()<<", rounding "<<std::fegetround()<<", MXCSR "<<mxcsr<<", EKF "<<HAL_WITH_EKF_DOUBLE<<", ALLOW "<<AP_MATH_ALLOW_DOUBLE_FUNCTIONS<<"\n";
    if(std::fegetround()!=FE_TONEAREST || (mxcsr & 0xe040u)) return 2;
    unsigned storage,n; uint64_t x,y,z,u,v,w;
    while(std::cin>>storage>>n>>x>>y>>z>>u>>v>>w) {
        if(storage==32) { if(n==2) run<float,Vector2>(x,y,z,u,v,w); else run<float,Vector3>(x,y,z,u,v,w); }
        else { if(n==2) run<double,Vector2>(x,y,z,u,v,w); else run<double,Vector3>(x,y,z,u,v,w); }
    }
}
