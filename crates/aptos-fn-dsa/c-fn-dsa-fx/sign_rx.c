/*
 * Operations on fixed-point values.
 */

#include "sign_inner.h"

#if defined __clang__ && defined __hash_builtin
#if __has_builtin(__builtin_clz)
#define clz32_nonzero(x)   __builtin_clz(x)
#endif
#if __has_builtin(__builtin_clzll)
#define clz64_nonzero(x)   __builtin_clzll(x)
#endif
#elif defined __GNUC__ && __GNUC__ >= 4
#define clz32_nonzero(x)   __builtin_clz(x)
#define clz64_nonzero(x)   __builtin_clzll(x)
#elif defined _MSC_VER && _MSC_VER >= 1500
static inline int
clz32_nonzero_inner(unsigned int n)
{
	unsigned long i;
	(void)_BitScanReverse(&i, n);
	return 31 - i;
}
#define clz32_nonzero(x)   clz32_nonzero_inner(x)
#endif

#ifndef clz32_nonzero
static inline int
clz32_nonzero(uint32_t x)
{
	uint32_t m = ((x >> 16) - 1) >> 16;
	uint32_t s = m & 16;
	x = (x & m) | (x >> 16);
	m = ((x >> 8) - 1) >> 8;
	s |= m & 8;
	x = (x & m) | (x >> 8);
	m = ((x >> 4) - 1) >> 4;
	s |= m & 4;
	x = (x & m) | (x >> 4);
	m = ((x >> 2) - 1) >> 2;
	s |= m & 2;
	x = (x & m) | (x >> 2);
	m = ((x >> 1) - 1) >> 1;
	s |= m & 1;
	return s;
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see sign_inner.h */
void
rx_div_spec(rx d, const rx a, const rx b)
{
	/* Normalize dividend and divisor so that 1/2 <= b < 1.
	   Since the division cannot overflow, we know that |a| < b*2^31,
	   so the normalization cannot make a overflow.
	   TODO: optimize with platform's opcodes for bit counting */
	rx ta, tb, ua, ub;
	uint32_t ctl;
	rx_copy(ua, a);
	rx_copy(ub, b);

	uint32_t iz = (uint32_t)(ub[2] - 1) >> 31;  /* iz == 1 if b[2] == 0 */
	uint32_t e = 32 - clz32_nonzero(ub[2] | iz);
	rx_div2e(ta, ua, e);
	rx_div2e(tb, ub, e);
	ctl = iz - 1;
	rx_copy_cond(ua, ta, ctl);
	rx_copy_cond(ub, tb, ctl);

	/* Now b < 1. If b < 1/2, we need to left-shift it. */
	rx_mul2p32(ta, ua);
	rx_mul2p32(tb, ub);
	ctl = ((ub[1] | -ub[1]) >> 31) - 1;
	rx_copy_cond(ua, ta, ctl);
	rx_copy_cond(ub, tb, ctl);
	/* b >= 2^(-32) */
	iz = ub[1] >> 31;
	e = clz32_nonzero(ub[1]) | iz;
	rx_mul2e(ta, ua, e);
	rx_mul2e(tb, ub, e);
	ctl = iz - 1;
	rx_copy_cond(ua, ta, ctl);
	rx_copy_cond(ub, tb, ctl);

	/* a and b have been normalized, so that 1/2 <= b < 1.
	   We now compute 1/b with Newton-Raphson. */
	static const rx RX_32_17 = { 3789677026, 3789677025, 1 };
	static const rx RX_48_17 = { 3537031891, 3537031890, 2 };
	static const rx RX_ONE = { 0, 0, 1 };
	uint64_t bf = rx_ufrac(ub);
	rx x;
	rx_mul_ufrac(x, RX_32_17, bf);
	rx_sub(x, RX_48_17, x);
	for (int i = 0; i < 5; i ++) {
		rx y;
		rx_mul_ufrac(y, x, bf);
		rx_sub(y, RX_ONE, y);
		rx_mul(y, y, x);
		rx_add(x, x, y);
	}

	/* x contains 1/b, we just multiply by a */
	rx_mul(d, ua, x);
}
#endif

#if 0
/* see sign_inner.h */
void
rx_div_spec(rx d, const rx a, const rx b)
{
	/* Get dividend sign (sa) and absolute value (v, 4 words). */
	uint32_t v[4];
	uint32_t sa = (uint32_t)(*(int32_t *)&a[2] >> 31);
	uint64_t w = (a[0] ^ sa) + (sa & 1);
	v[0] = (uint32_t)w;
	w = (a[1] ^ sa) + (w >> 32);
	v[1] = (uint32_t)w;
	w = (a[2] ^ sa) + (w >> 32);
	v[2] = (uint32_t)w;
	v[3] = (uint32_t)(w >> 32);

	/* Obtain the quotient bit by bit, repeatedly subtracting the
	   shifted divisor when possible. */
	uint64_t u[3];
	u[0] = (uint64_t)b[0] << 32;
	u[1] = (uint64_t)b[1] << 32;
	u[2] = (uint64_t)b[2] << 32;
	uint32_t q[3];

	for (int j = 2; j >= 0; j --) {
		uint32_t qw = 0;
		for (int i = 31; i >= 0; i --) {
			u[0] >>= 1;
			u[1] >>= 1;
			u[2] >>= 1;
			w = v[0] - u[0];
			uint32_t r0 = (uint32_t)w;
			w = v[1] - u[1] + (uint64_t)(*(int64_t *)&w >> 32);
			uint32_t r1 = (uint32_t)w;
			w = v[2] - u[2] + (uint64_t)(*(int64_t *)&w >> 32);
			uint32_t r2 = (uint32_t)w;
			w = v[3] + (uint64_t)(*(int64_t *)&w >> 32);
			uint32_t r3 = (uint32_t)w;

			uint32_t m = (uint32_t)(w >> 63) - 1;
			v[0] ^= m & (r0 ^ v[0]);
			v[1] ^= m & (r1 ^ v[1]);
			v[2] ^= m & (r2 ^ v[2]);
			v[3] ^= m & (r3 ^ v[3]);
			qw = (qw << 1) - m;
		}
		q[j] = qw;
		u[0] <<= 32;
		u[1] <<= 32;
		u[2] <<= 32;
		v[3] = v[2];
		v[2] = v[1];
		v[1] = v[0];
		v[0] = 0;
	}

	/* Apply sign back. */
	w = (q[0] ^ sa) + (sa & 1);
	d[0] = (uint32_t)w;
	w = (q[1] ^ sa) + (w >> 32);
	d[1] = (uint32_t)w;
	d[2] = (q[2] ^ sa) + (uint32_t)(w >> 32);

	/* Note: this returns the quotient rounded toward 0. */
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see sign_inner.h */
void
rx_sqrt_spec(rx d, const rx a)
{
	/* Inputs are un-normalized leaf values. We know that:
	      sigma_min <= sigma/sqrt(a) <= sigma_max
	   which implies in practice that:
	      6327 < a < 16823
	   We use a Newton-Raphson approximation of 1/sqrt(a/2^12).
	   We start at x = 0.64 (since a is supposed to be "around 10000").
	   Then, we repeatedly write:
	      x <- x*(3/2 - (a/2)*x^2)
	   x converges on 1/sqrt(a/2^12), hence we only have to return
	   a*(x/2^6) to get a good approximation of sqrt(a).
	   8 iterations are deemed enough. */
	static const rx RX_THREE = { 0, 0, 3 };
	rx b;
	b[0] = (a[0] >> 12) | (a[1] << 20);
	b[1] = (a[1] >> 12) | (a[2] << 20);
	b[2] = a[2] >> 12;
	uint64_t xf = 11805916207174113034u;
	for (int i = 0; i < 8; i ++) {
		uint64_t yf = rx_mul_ufrac_ufrac(xf, xf);
		rx y;
		rx_mul_ufrac(y, b, yf);
		rx_sub(y, RX_THREE, y);
		rx_half(y, y);
		rx_mul_ufrac(y, y, xf);
		xf = rx_ufrac(y);
	}
	rx_mul_ufrac(d, a, xf >> 6);
}
#endif
