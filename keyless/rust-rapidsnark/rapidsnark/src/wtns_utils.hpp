#ifndef WTNS_UTILS
#define WTNS_UTILS

#include <gmp.h>

#include "binfile_utils.hpp"

namespace WtnsUtils {

    class Header {
    public:
        u_int32_t n8;
        mpz_t prime;

        u_int32_t nVars;

        Header();
        ~Header();
    };

    std::unique_ptr<Header> loadHeader(BinFileUtils::BinFile *f);

}

#endif // ZKEY_UTILS_H