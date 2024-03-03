#ifndef MISC_H
#define MISC_H

#ifdef USE_OPENMP
#include <omp.h>
#endif
#include <cstdint>

uint32_t log2 (uint32_t value);

#ifdef _OPENMP

/**
 * This object is used to temporarily change the max number of omp threads.
 * When the object is destructed, the max threads is set to it's original value.
 */
class ThreadLimit {
public:
    ThreadLimit(uint32_t maxThreads):
        prev_max_threads(omp_get_max_threads())
    {
        omp_set_num_threads(maxThreads);
    }

    ~ThreadLimit() noexcept
    {
        omp_set_num_threads(prev_max_threads);
    }

private:
    uint32_t prev_max_threads;
};

#endif // _OPENMP

#endif // MISC_H
