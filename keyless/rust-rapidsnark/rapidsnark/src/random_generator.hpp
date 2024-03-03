#ifndef RANDOM_GENERATOR_H
#define RANDOM_GENERATOR_H

#ifdef USE_SODIUM

#include <sodium.h>

#else

#include <random>

inline void
randombytes_buf(void * const buf, const size_t size)
{
    std::random_device engine;
    std::uniform_int_distribution<uint8_t> distr;

    uint8_t *buffer = static_cast<uint8_t*>(buf);

    for(size_t i = 0; i < size; i++) {
        buffer[i] = distr(engine);
    }
}

#endif //USE_SODIUM

#endif // RANDOM_GENERATOR_H
