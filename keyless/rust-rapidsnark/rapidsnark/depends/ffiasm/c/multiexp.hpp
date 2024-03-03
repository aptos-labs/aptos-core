#ifndef PAR_MULTIEXP2
#define PAR_MULTIEXP2

#define PME2_PACK_FACTOR 2
#define PME2_MAX_CHUNK_SIZE_BITS 16
#define PME2_MIN_CHUNK_SIZE_BITS 2

#include <cstdint>

template <typename Curve>
class ParallelMultiexp {

    struct PaddedPoint {
        typename Curve::Point p;
//        uint8_t padding[32];
    };

    typename Curve::PointAffine *bases;
    uint8_t* scalars;
    uint64_t scalarSize;
    uint64_t n;
    uint64_t nThreads;
    uint64_t bitsPerChunk;
    uint64_t accsPerChunk;
    uint64_t nChunks;
    Curve &g;
    PaddedPoint *accs;

    void initAccs();

    uint64_t getChunk(uint64_t scalarIdx, uint64_t chunkIdx);
    void processChunk(uint64_t idxChunk);
    void processChunk(uint64_t idxChunk, uint64_t nx, uint64_t x[]);
    void packThreads();
    void reduce(typename Curve::Point &res, uint64_t nBits);

public:
    ParallelMultiexp(Curve &_g): g(_g) {}
    void multiexp(typename Curve::Point &r, typename Curve::PointAffine *_bases, uint8_t* _scalars, uint64_t _scalarSize, uint64_t _n, uint64_t _nThreads=0);
    void multiexp(typename Curve::Point &r,
                  typename Curve::PointAffine *_bases,
                  uint8_t* _scalars,
                  uint64_t _scalarSize,
                  uint64_t _n,
                  uint64_t nx,
                  uint64_t x[],
                  uint64_t _nThreads=0);

};

#include "multiexp.cpp"

#endif // PAR_MULTIEXP2
