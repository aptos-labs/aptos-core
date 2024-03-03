#ifndef FFT_H
#define FFT_H

template <typename Field>
class FFT {
    Field f;
    typedef typename Field::Element Element;

    u_int32_t s;
    Element nqr;
    Element *roots;
    Element *powTwoInv;
    u_int32_t nThreads;

    void reversePermutationInnerLoop(Element *a, u_int64_t from, u_int64_t to, u_int32_t domainPow);
    void reversePermutation(Element *a, u_int64_t n);
    void fftInnerLoop(Element *a, u_int64_t from, u_int64_t to, u_int32_t s);
    void finalInverseInner(Element *a, u_int64_t from, u_int64_t to, u_int32_t domainPow);

public:

    FFT(u_int64_t maxDomainSize, u_int32_t _nThreads = 0);
    ~FFT();
    void fft(Element *a, u_int64_t n );
    void ifft(Element *a, u_int64_t n );

    u_int32_t log2(u_int64_t n);
    inline Element &root(u_int32_t domainPow, u_int64_t idx) { return roots[ idx << (s-domainPow)]; }

    void printVector(Element *a, u_int64_t n );

};

#include "fft.cpp"

#endif // FFT_H
