/*
 * SPDX-FileCopyrightText: 2019 Louis Solofrizzo
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* utils.cc
   Louis Solofrizzo 2019-10-17

   Utilities for pistache
*/

#include <pistache/peer.h>
#include <unistd.h>

#ifdef PISTACHE_USE_SSL

ssize_t SSL_sendfile(SSL* out, int in, off_t* offset, size_t count)
{
    unsigned char buffer[4096] = { 0 };
    ssize_t ret;
    ssize_t written;
    size_t to_read;

    if (in == -1)
        return -1;

    to_read = sizeof(buffer) > count ? count : sizeof(buffer);

    if (offset != NULL)
        ret = pread(in, buffer, to_read, *offset);
    else
        ret = read(in, buffer, to_read);

    if (ret == -1)
        return -1;

    written = SSL_write(out, buffer, static_cast<int>(ret));
    if (offset != NULL)
        *offset += written;

    return written;
}

#endif /* PISTACHE_USE_SSL */
