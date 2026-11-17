/*
 * Access to the operating system's RNG.
 */

#include "inner.h"

/* On Linux (glibc-2.25+), FreeBSD 12+ and OpenBSD, we can use
   getentropy(). */
#ifndef FNDSA_RNG_GETENTROPY
#if (defined __linux__ && defined __GLIBC__ \
	&& (__GLIBC__ > 2 || (__GLIBC__ == 2 && __GLIBC_MINOR__ >= 25))) \
	|| (defined __FreeBSD__ && __FreeBSD__ >= 12) \
	|| defined __OpenBSD__
#define FNDSA_RNG_GETENTROPY   1
#else
#define FNDSA_RNG_GETENTROPY   0
#endif
#endif

/* On most Unix-like systems, we can try to open /dev/urandom. */
#ifndef FNDSA_RNG_URANDOM
#if defined _AIX \
	|| defined __ANDROID__ \
	|| defined __FreeBSD__ \
	|| defined __NetBSD__ \
	|| defined __OpenBSD__ \
	|| defined __DragonFly__ \
	|| defined __linux__ \
	|| (defined __sun && (defined __SVR4 || defined __svr4__)) \
	|| (defined __APPLE__ && defined __MACH__)
#define FNDSA_RNG_URANDOM   1
#else
#define FNDSA_RNG_URANDOM   0
#endif
#endif

/* Windows has its own system call. */
#ifndef FNDSA_RNG_WIN32
#if defined _WIN32 || defined _WIN64
#define FNDSA_RNG_WIN32   1
#else
#define FNDSA_RNG_WIN32   0
#endif
#endif

#if FNDSA_RNG_GETENTROPY
#include <unistd.h>
#endif
#if FNDSA_RNG_URANDOM
#include <sys/types.h>
#if !FNDSA_RNG_GETENTROPY
#include <unistd.h>
#endif
#include <fcntl.h>
#include <errno.h>
#endif
#if FNDSA_RNG_WIN32
#include <windows.h>
#define SystemFunction036   NTAPI SystemFunction036
#include <NTSecAPI.h>
#undef SystemFunction036
#pragma comment(lib, "advapi32")
#endif

/* see inner.h */
int
sysrng(void *dst, size_t len)
{
	(void)dst;
	if (len == 0) {
		return 1;
	}
#if FNDSA_RNG_GETENTROPY
	if (getentropy(dst, len) == 0) {
		return 1;
	}
#endif
#if FNDSA_RNG_URANDOM
	int f = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
	if (f >= 0) {
		while (len > 0) {
			ssize_t rlen = read(f, dst, len);
			if (rlen < 0) {
				if (errno == EINTR) {
					continue;
				}
				break;
			}
			dst = (uint8_t *)dst + rlen;
			len -= (size_t)rlen;
		}
		close(f);
		if (len == 0) {
			return 1;
		}
	}
#endif
#if FNDSA_RNG_WIN32
	if (RtlGenRandom(dst, len)) {
		return 1;
	}
#endif
	return 0;
}
