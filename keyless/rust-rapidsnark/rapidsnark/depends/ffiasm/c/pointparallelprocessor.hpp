#ifndef POINT_PARALLEL_PROCESSOR_H
#define POINT_PARALLEL_PROCESSOR_H

#include <vector>
#include <thread>
#include <mutex>
#include <condition_variable>
#include "growablearray_mt.hpp"

#define NOPS_CHUNK (uint64_t)(1LL<<13)
#define MAX_LEVELS 1024
template <typename Curve>
class PointParallelProcessor {

    Curve &curve;
    enum Function { ADD, ADD_MIXED, ADD_AFFINE };
    struct Op {
        Function fn;
        void *r;
        void *a;
        void *b;
        Op(Function _fn, void * _r,void *_a, void *_b) : fn(_fn), r(_r), a(_a), b(_b) {}; 
        Op() {};
    };

public:
    enum Source { ZERO=0, BASE=1, HEAP=2};

//    #pragma pack(push, 1)
    struct Point {
        Source source;
        uint16_t level;
        void *p;
    };
//    #pragma pack(pop)

private:
    typename Curve::PointAffine *bases;
    GrowableArrayMT<typename Curve::Point> *heap;
    GrowableArrayMT<Op> **ops;
    u_int32_t nLevels;

    bool terminated;
    uint32_t nThreads;
    uint32_t currentLevel;
    typename GrowableArrayMT<Op>::Iterator itExecuting;
    uint64_t pendingThreads;

    std::vector<std::thread> threads;
    std::mutex cv_mutex;
    std::condition_variable cv;

    void addOp(uint32_t idThread, uint32_t level, Function fn, Point r, Point a, Point b);
    Point allocHeapPoint(uint32_t idThread, uint32_t level);
    void *getPointPointer(Point p);

    void childThread(uint32_t th);
    void innerProcess(uint32_t level, typename GrowableArrayMT<Op>::Iterator start, typename GrowableArrayMT<Op>::Iterator end);


public:

    PointParallelProcessor(Curve &_curve, uint32_t _nThreads, typename Curve::PointAffine *_bases)  : curve(_curve) {
        bases = _bases;
        nThreads = _nThreads;
        terminated = false;
        nLevels = 0;
        ops = new GrowableArrayMT<Op> *[MAX_LEVELS];
        for (uint32_t i=0; i<MAX_LEVELS; i++) {
            ops[i] = new GrowableArrayMT<Op>(nThreads);
        }
        heap = new GrowableArrayMT<typename Curve::Point>(nThreads);
    }

    ~PointParallelProcessor() {
        for (uint32_t i=0; i<MAX_LEVELS; i++) {
            delete ops[i];
        }
        delete[] ops;
        delete heap;
    }

    Point add(uint32_t idThread, Point a, Point b);

    void calculate();

    void extractResult(typename Curve::Point &r, Point &v);
    inline Point basePoint(uint32_t idx) { 
        Point p;
        p.source = BASE;
        p.level = 0;
        p.p = &bases[idx];
        return p; 
    };
    inline Point zero() { 
        Point p;
        p.source = ZERO;
        p.level = 0;
        return p; 
    };
};

#include "pointparallelprocessor.cpp"

#endif // POINT_PARALLEL_PROCESSOR_H