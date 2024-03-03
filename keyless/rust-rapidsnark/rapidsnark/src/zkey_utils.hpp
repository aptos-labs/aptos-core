#ifndef ZKEY_UTILS_H
#define ZKEY_UTILS_H

#include <gmp.h>
#include <memory>

#include "binfile_utils.hpp"

namespace ZKeyUtils {

    class Header {


    public:
        u_int32_t n8q;
        mpz_t qPrime;
        u_int32_t n8r;
        mpz_t rPrime;

        u_int32_t nVars;
        u_int32_t nPublic;
        u_int32_t domainSize;
        u_int64_t nCoefs;

        void *vk_alpha1;
        void *vk_beta1;
        void *vk_beta2;
        void *vk_gamma2;
        void *vk_delta1;
        void *vk_delta2;

        Header();
        ~Header();
    };

    std::unique_ptr<Header> loadHeader(BinFileUtils::BinFile *f);
}

#endif // ZKEY_UTILS_H
