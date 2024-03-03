#ifdef USE_OPENMP
#include <omp.h>
#endif
#include <memory.h>
#include "misc.hpp"
#include "multiexp.hpp"
/*
template <typename Curve>
void ParallelMultiexp<Curve>::initAccs() {
    #pragma omp parallel for
    for (int i=0; i<nThreads; i++) {
        memset((void *)&(accs[i*accsPerChunk]), 0, accsPerChunk*sizeof(PaddedPoint));
    }
}
*/

template <typename Curve>
void ParallelMultiexp<Curve>::initAccs() {
    #pragma omp parallel for
    for (uint64_t i=0; i<nThreads*accsPerChunk; i++) {
        g.copy(accs[i].p, g.zero());
    }
}

template <typename Curve>
uint64_t ParallelMultiexp<Curve>::getChunk(uint64_t scalarIdx, uint64_t chunkIdx) {
    uint64_t bitStart = chunkIdx*bitsPerChunk;
    uint64_t byteStart = bitStart/8;
    uint64_t efectiveBitsPerChunk = bitsPerChunk;
    if (byteStart > scalarSize-8) byteStart = scalarSize - 8;
    if (bitStart + bitsPerChunk > scalarSize*8) efectiveBitsPerChunk = scalarSize*8 - bitStart;
    uint64_t shift = bitStart - byteStart*8;
    uint64_t v = *(uint64_t *)(scalars + scalarIdx*scalarSize + byteStart);
    v = v >> shift;
    v = v & ( (1 << efectiveBitsPerChunk) - 1);
    return uint64_t(v);
}

// go over all the numbers (windowed numbered) in the window/chunk and add them to their corresponding index
template <typename Curve>
void ParallelMultiexp<Curve>::processChunk(uint64_t idChunk) {
    #pragma omp parallel for
    for(uint64_t i=0; i<n; i++) {
        if (g.isZero(bases[i])) continue;
        uint64_t chunkValue = getChunk(i, idChunk);
#ifdef _OPENMP
        int idThread = omp_get_thread_num();
#else
        int idThread = 0;
#endif
//        if(chunkValue==0) continue;
        if (chunkValue) {
            g.add(accs[idThread*accsPerChunk+chunkValue].p, accs[idThread*accsPerChunk+chunkValue].p, bases[i]);
        }
    }
}

template <typename Curve>
void ParallelMultiexp<Curve>::processChunk(uint64_t idChunk, uint64_t nX, uint64_t size[]) {
    #pragma omp parallel for
    for(uint64_t i=0; i<n; i++) {
        uint mod = i % nX;
        uint len = size[mod] - 1;
        if (i < 0 || i > len * nX + mod) continue;
        if (g.isZero(bases[i])) continue;
#ifdef _OPENMP
        int idThread = omp_get_thread_num();
#else
        int idThread = 0;
#endif
        uint64_t chunkValue = getChunk(i, idChunk);

        if (chunkValue) {
            g.add(accs[idThread*accsPerChunk+chunkValue].p, accs[idThread*accsPerChunk+chunkValue].p, bases[i]);
        }
    }
}

// This function takes all chunks and accumulate them to the first chunk's indexes
template <typename Curve>
void ParallelMultiexp<Curve>::packThreads() {
    #pragma omp parallel for
    for(uint64_t i=0; i<accsPerChunk; i++) {
        for(uint64_t j=1; j<nThreads; j++) {
            if (!g.isZero(accs[j*accsPerChunk + i].p)) {
                g.add(accs[i].p, accs[i].p, accs[j*accsPerChunk + i].p);
                g.copy(accs[j*accsPerChunk + i].p, g.zero());
            }
        }
    }
}

template <typename Curve>
void ParallelMultiexp<Curve>::reduce(typename Curve::Point &res, uint64_t nBits) {
    if (nBits==1) {
        g.copy(res, accs[1].p);
        g.copy(accs[1].p, g.zero());
        return;
    }
    uint64_t ndiv2 = 1 << (nBits-1);


    PaddedPoint *sall = new PaddedPoint[nThreads];
    memset(sall, 0, sizeof(PaddedPoint)*nThreads);

    #pragma omp parallel for
    for (uint64_t i = 1; i<ndiv2; i++) {
#ifdef _OPENMP
        int idThread = omp_get_thread_num();
#else
        int idThread = 0;
#endif
        if (!g.isZero(accs[ndiv2 + i].p)) {
            g.add(accs[i].p, accs[i].p, accs[ndiv2 + i].p);
            g.add(sall[idThread].p, sall[idThread].p, accs[ndiv2 + i].p);
            g.copy(accs[ndiv2 + i].p, g.zero());
        }
    }
    for (u_int32_t i=0; i<nThreads; i++) {
        g.add(accs[ndiv2].p, accs[ndiv2].p, sall[i].p);
    }

    typename Curve::Point p1;
    reduce(p1, nBits-1);

    for (u_int32_t i=0; i<nBits-1; i++) g.dbl(accs[ndiv2].p, accs[ndiv2].p);
    g.add(res, p1, accs[ndiv2].p);
    g.copy(accs[ndiv2].p, g.zero());
    delete[] sall;
}

template <typename Curve>
void ParallelMultiexp<Curve>::multiexp(typename Curve::Point &r, typename Curve::PointAffine *_bases, uint8_t* _scalars, uint64_t _scalarSize, uint64_t _n, uint64_t _nThreads) {
#ifdef _OPENMP
    nThreads = _nThreads==0 ? omp_get_max_threads() : _nThreads;
    ThreadLimit threadLimit (nThreads);
#else
    nThreads = 1;
#endif
    bases = _bases;
    scalars = _scalars;
    scalarSize = _scalarSize;
    n = _n;

    if (n==0) {
        g.copy(r, g.zero());
        return;
    }
    if (n==1) {
        g.mulByScalar(r, bases[0], scalars, scalarSize);
        return;
    }

    bitsPerChunk = log2((uint32_t)(n / PME2_PACK_FACTOR));

    if (bitsPerChunk > PME2_MAX_CHUNK_SIZE_BITS) bitsPerChunk = PME2_MAX_CHUNK_SIZE_BITS;
    if (bitsPerChunk < PME2_MIN_CHUNK_SIZE_BITS) bitsPerChunk = PME2_MIN_CHUNK_SIZE_BITS;
    nChunks = ((scalarSize*8 - 1 ) / bitsPerChunk)+1;
    accsPerChunk = 1 << bitsPerChunk;  // In the chunks last bit is always zero.

    typename Curve::Point *chunkResults = new typename Curve::Point[nChunks];
    accs = new PaddedPoint[nThreads*accsPerChunk];
    // std::cout << "InitTrees " << "\n";
    initAccs();

    for (uint64_t i=0; i<nChunks; i++) {
        // std::cout << "process chunks " << i << "\n"; 

        processChunk(i);
        // std::cout << "pack " << i << "\n";
        packThreads();
        // std::cout << "reduce " << i << "\n";
        reduce(chunkResults[i], bitsPerChunk);
    }

    delete[] accs;

    g.copy(r, chunkResults[nChunks-1]);
    for  (int j=nChunks-2; j>=0; j--) {
        for (uint64_t k=0; k<bitsPerChunk; k++) g.dbl(r,r);
        g.add(r, r, chunkResults[j]);
    }

    delete[] chunkResults;
}

template <typename Curve>
void ParallelMultiexp<Curve>::multiexp(typename Curve::Point &r,
                                        typename Curve::PointAffine *_bases,
                                        uint8_t* _scalars, uint64_t _scalarSize,
                                       uint64_t _n,
                                      uint64_t nx,
                                      uint64_t x[],
                                        uint64_t _nThreads) {
#ifdef _OPENMP
    nThreads = _nThreads==0 ? omp_get_max_threads() : _nThreads;
    ThreadLimit threadLimit (nThreads);
#else
    nThreads = 1;
#endif

    bases = _bases;
    scalars = _scalars;
    scalarSize = _scalarSize;
    n = _n;

    if (n == 0) {
        g.copy(r, g.zero());
        return;
    }
    if (n == 1) {
        g.mulByScalar(r, bases[0], scalars, scalarSize);
        return;
    }
    bitsPerChunk = log2((uint32_t)(n / PME2_PACK_FACTOR));
    if (bitsPerChunk > PME2_MAX_CHUNK_SIZE_BITS) bitsPerChunk = PME2_MAX_CHUNK_SIZE_BITS;
    if (bitsPerChunk < PME2_MIN_CHUNK_SIZE_BITS) bitsPerChunk = PME2_MIN_CHUNK_SIZE_BITS;
    nChunks = ((scalarSize * 8 - 1) / bitsPerChunk) + 1;
    accsPerChunk = 1 << bitsPerChunk;  // In the chunks last bit is always zero.

    typename Curve::Point *chunkResults = new typename Curve::Point[nChunks];
    accs = new PaddedPoint[nThreads * accsPerChunk];
    // std::cout << "InitTrees " << "\n";
    initAccs();
    for (uint64_t i = 0; i < nChunks; i++) {
        // std::cout << "process chunks " << i << "\n";
        processChunk(i, nx, x);
        // std::cout << "pack " << i << "\n";
        packThreads();
        // std::cout << "reduce " << i << "\n";
        reduce(chunkResults[i], bitsPerChunk);
    }

    delete[] accs;

    g.copy(r, chunkResults[nChunks - 1]);
    for (int j = nChunks - 2; j >= 0; j--) {
        for (uint64_t k = 0; k < bitsPerChunk; k++) g.dbl(r, r);
        g.add(r, r, chunkResults[j]);
    }

    delete[] chunkResults;
}
