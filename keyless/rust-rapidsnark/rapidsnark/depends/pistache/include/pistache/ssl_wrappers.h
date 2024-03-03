/*
 * SPDX-FileCopyrightText: 2020 hyperxor
 *
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#ifdef PISTACHE_USE_SSL
#include <openssl/ssl.h>
#endif

#include <memory>

namespace Pistache::ssl
{

    struct SSLCtxDeleter
    {
        void operator()([[maybe_unused]] void* ptr)
        {
#ifdef PISTACHE_USE_SSL
            SSL_CTX_free(reinterpret_cast<SSL_CTX*>(ptr));

            // EVP_cleanup call is not related to cleaning SSL_CTX, just global cleanup routine.
            // TODO: Think about removing EVP_cleanup call at all
            // It was deprecated in openssl 1.1.0 version (see
            // https://www.openssl.org/news/changelog.txt):
            // "Make various cleanup routines no-ops and mark them as deprecated."
            EVP_cleanup();
#endif
        }
    };

    struct SSLBioDeleter
    {
        void operator()([[maybe_unused]] void* ptr)
        {
#ifdef PISTACHE_USE_SSL
            BIO_free(reinterpret_cast<BIO*>(ptr));
#endif
        }
    };

    using SSLCtxPtr = std::unique_ptr<void, SSLCtxDeleter>;
    using SSLBioPtr = std::unique_ptr<void, SSLBioDeleter>;

#ifdef PISTACHE_USE_SSL
    inline SSL_CTX* GetSSLContext(ssl::SSLCtxPtr& ctx)
    {
        return reinterpret_cast<SSL_CTX*>(ctx.get());
    }
    inline BIO* GetSSLBio(ssl::SSLBioPtr& ctx)
    {
        return reinterpret_cast<BIO*>(ctx.get());
    }
#endif

} // namespace Pistache::ssl
