#include <thread>
#include <vector>
#ifdef USE_OPENMP
#include <omp.h>
#endif

using namespace std;

// The function we want to execute on the new thread.

template <typename Field>
u_int32_t FFT<Field>::log2(u_int64_t n) {
    assert(n!=0);
    u_int32_t res=0;
    while (n!=1) {
        n >>= 1;
        res ++;
    }
    return res;
}

static inline u_int64_t BR(u_int64_t x, u_int64_t domainPow)
{
    x = (x >> 16) | (x << 16);
    x = ((x & 0xFF00FF00) >> 8) | ((x & 0x00FF00FF) << 8);
    x = ((x & 0xF0F0F0F0) >> 4) | ((x & 0x0F0F0F0F) << 4);
    x = ((x & 0xCCCCCCCC) >> 2) | ((x & 0x33333333) << 2);
    return (((x & 0xAAAAAAAA) >> 1) | ((x & 0x55555555) << 1)) >> (32-domainPow);
}

#define ROOT(s,j) (rootsOfUnit[(1<<(s))+(j)])

template <typename Field>
FFT<Field>::FFT(u_int64_t maxDomainSize, uint32_t _nThreads) {
#ifdef _OPENMP
    nThreads = _nThreads==0 ? omp_get_max_threads() : _nThreads;
#else
    nThreads = 1;
#endif
    f = Field::field;

    u_int32_t domainPow = log2(maxDomainSize);

    mpz_t m_qm1d2;
    mpz_t m_q;
    mpz_t m_nqr;
    mpz_t m_aux;
    mpz_init(m_qm1d2);
    mpz_init(m_q);
    mpz_init(m_nqr);
    mpz_init(m_aux);

    f.toMpz(m_aux, f.negOne());     

    mpz_add_ui(m_q, m_aux, 1);
    mpz_fdiv_q_2exp(m_qm1d2, m_aux, 1);

    mpz_set_ui(m_nqr, 2);
    mpz_powm(m_aux, m_nqr, m_qm1d2, m_q);
    while (mpz_cmp_ui(m_aux, 1) == 0) {
        mpz_add_ui(m_nqr, m_nqr, 1);
        mpz_powm(m_aux, m_nqr, m_qm1d2, m_q);
    }

    f.fromMpz(nqr, m_nqr);

    // std::cout << "nqr: " << f.toString(nqr) << std::endl;

    s = 1;
    mpz_set(m_aux, m_qm1d2);
    while ((!mpz_tstbit(m_aux, 0))&&(s<domainPow)) {
        mpz_fdiv_q_2exp(m_aux, m_aux, 1);
        s++;
    }

    if (s<domainPow) {
        throw std::range_error("Domain size too big for the curve");
    }

    uint64_t nRoots = 1LL << s;

    roots = new Element[nRoots];
    powTwoInv = new Element[s+1];

    f.copy(roots[0], f.one());
    f.copy(powTwoInv[0], f.one());
    if (nRoots>1) {
        mpz_powm(m_aux, m_nqr, m_aux, m_q);
        f.fromMpz(roots[1], m_aux);

        mpz_set_ui(m_aux, 2);
        mpz_invert(m_aux, m_aux, m_q);
        f.fromMpz(powTwoInv[1], m_aux);
    }
    #pragma omp parallel
    {
#ifdef _OPENMP
        int idThread = omp_get_thread_num();
        int nThreads = omp_get_num_threads();
#else
        int idThread = 0;
        int nThreads = 1;
#endif
        uint64_t increment = nRoots / nThreads;
        uint64_t start = idThread==0 ? 2 : idThread * increment;
        uint64_t end   = idThread==nThreads-1 ? nRoots : (idThread+1) * increment;
        if (end>start) {
            f.exp(roots[start], roots[1], (uint8_t *)(&start), sizeof(start));
        }
        for (uint64_t i=start+1; i<end; i++) {
            f.mul(roots[i], roots[i-1], roots[1]);
        }
    }
    Element aux;
    f.mul(aux, roots[nRoots-1], roots[1] );
    assert(f.eq(aux, f.one()));

    for (uint64_t i=2; i<=s; i++) {
        f.mul(powTwoInv[i], powTwoInv[i-1], powTwoInv[1]);
    }

    mpz_clear(m_qm1d2);
    mpz_clear(m_q);
    mpz_clear(m_nqr);
    mpz_clear(m_aux);
}

template <typename Field>
FFT<Field>::~FFT() {
    delete[] roots;
    delete[] powTwoInv;
}

/*
template <typename Field>
void FFT<Field>::reversePermutationInnerLoop(Element *a, u_int64_t from, u_int64_t to, u_int32_t domainPow) {
    Element tmp;
    for (u_int64_t i=from; i<to; i++) {
        u_int64_t r = BR(i, domainPow);
        if (i>r) {
            f.copy(tmp, a[i]);
            f.copy(a[i], a[r]);
            f.copy(a[r], tmp);
        }
    }
}


template <typename Field>
void FFT<Field>::reversePermutation(Element *a, u_int64_t n) {
    int domainPow = log2(n);
    std::vector<std::thread> threads(nThreads-1);
    u_int64_t increment = n / nThreads;
    if (increment) {
        for (u_int64_t i=0; i<nThreads-1; i++) {
            threads[i] = std::thread (&FFT<Field>::reversePermutationInnerLoop, this, a, i*increment, (i+1)*increment, domainPow);
        }
    }
    reversePermutationInnerLoop(a, (nThreads-1)*increment, n, domainPow);
    if (increment) {
        for (u_int32_t i=0; i<nThreads-1; i++) {
            if (threads[i].joinable()) threads[i].join();
        }
    }
}
*/


template <typename Field>
void FFT<Field>::reversePermutation(Element *a, u_int64_t n) {
    int domainPow = log2(n);
    #pragma omp parallel for
    for (u_int64_t i=0; i<n; i++) {
        Element tmp;
        u_int64_t r = BR(i, domainPow);
        if (i>r) {
            f.copy(tmp, a[i]);
            f.copy(a[i], a[r]);
            f.copy(a[r], tmp);
        }
    }
}


template <typename Field>
void FFT<Field>::fft(Element *a, u_int64_t n) {
    reversePermutation(a, n);
    u_int64_t domainPow =log2(n);
    assert(((u_int64_t)1 << domainPow) == n);
    for (u_int32_t s=1; s<=domainPow; s++) {
        u_int64_t m = 1 << s;
        u_int64_t mdiv2 = m >> 1;
        #pragma omp parallel for
        for (u_int64_t i=0; i< (n>>1); i++) {
            Element t;
            Element u;
            u_int64_t k=(i/mdiv2)*m;
            u_int64_t j=i%mdiv2;

            f.mul(t, root(s, j), a[k+j+mdiv2]);
            f.copy(u,a[k+j]);
            f.add(a[k+j], t, u);
            f.sub(a[k+j+mdiv2], u, t);
        }
    }
}

template <typename Field>
void FFT<Field>::ifft(Element *a, u_int64_t n ) {
    fft(a, n);
    u_int64_t domainPow =log2(n);
    u_int64_t nDiv2= n >> 1; 
    #pragma omp parallel for
    for (u_int64_t i=1; i<nDiv2; i++) {
        Element tmp;
        u_int64_t r = n-i;
        f.copy(tmp, a[i]);
        f.mul(a[i], a[r], powTwoInv[domainPow]);
        f.mul(a[r], tmp, powTwoInv[domainPow]);
    } 
    f.mul(a[0], a[0], powTwoInv[domainPow]);
    f.mul(a[n >> 1], a[n >> 1], powTwoInv[domainPow]);
}



template <typename Field>
void FFT<Field>::printVector(Element *a, u_int64_t n ) {
    cout << "[" << endl;
    for (u_int64_t i=0; i<n; i++) {
        cout << f.toString(a[i]) << endl;
    }
    cout << "]" << endl;
}

