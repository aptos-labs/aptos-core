/*
 * SPDX-FileCopyrightText: 2019 Louis Solofrizzo
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* utils.h
   Louis Solofrizzo 2019-10-17

   Utilities for pistache
*/

#pragma once

#ifdef PISTACHE_USE_SSL

#include <openssl/ssl.h>

/*!
 * \brief sendfile(2) like utility for OpenSSL contexts
 *
 * \param[out] out The SSL context
 * \param[in] in File descriptor to stream
 * \param[out] offset See below.
 * \param[in] count Number of bytes to write
 *
 * Unlike the system call, this function will bufferise data in user-space,
 * thus making it blocking and memory hungry.
 *
 * If offset is  not NULL, then it points to a variable holding the file offset
 * from which SSL_sendfile() will start reading data from in. When
 * SSL_sendfile() returns, this  variable will be set to the offset of the byte
 * following the last byte that was read.
 *
 * \note This function exists in OpenSSL3[1]. It uses KTLS features which are
 * far more superior that this function. We'll need to do the switch when
 * OpenSSL3 becomes the SSL mainline.
 *
 * \return The number of bytes written to the SSL context
 *
 * [1] https://www.openssl.org/docs/manmaster/man3/SSL_sendfile.html
 */
ssize_t SSL_sendfile(SSL* out, int in, off_t* offset, size_t count);

#endif /* PISTACHE_USE_SSL */
