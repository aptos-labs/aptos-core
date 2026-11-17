#ifndef FNDSA_SIGN_INNER_H__
#define FNDSA_SIGN_INNER_H__

/* ==================================================================== */
/*
 * This file includes declarations used by the signature generation code
 * only.
 */

#include "inner.h"

/* ==================================================================== */
/*
 * Fixed-point values.
 *
 * Each fixed-point value is represented over 96 bits (12 bytes).
 * For representation x, mathematical value is:
 *    signed(x[2]) + x[1]/2^32 + x[2]/2^64
 * (top word x[2] is interpreted as a signed value)
 */
typedef uint32_t rx[3];

/* d <- a */
static inline void
rx_copy(rx d, const rx a)
{
	d[0] = a[0];
	d[1] = a[1];
	d[2] = a[2];
}

/* If ctl == 0xFFFFFFFF: d <- a
   If ctl == 0x00000000: d is unchanged
   ctl MUST be either 0xFFFFFFFF of 0x00000000 */
static inline void
rx_copy_cond(rx d, const rx a, uint32_t ctl)
{
	d[0] ^= ctl & (a[0] ^ d[0]);
	d[1] ^= ctl & (a[1] ^ d[1]);
	d[2] ^= ctl & (a[2] ^ d[2]);
}

/* d <- a + b */
static inline void
rx_add(rx d, const rx a, const rx b)
{
#if defined __x86_64__ || defined _M_X64
	uint64_t a0 = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t b0 = (uint64_t)b[0] | ((uint64_t)b[1] << 32);
	unsigned long long z0;
	unsigned int z1;
	unsigned char c = _addcarry_u64(0, a0, b0, &z0);
	(void)_addcarry_u32(c, a[2], b[2], &z1);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)(z0 >> 32);
	d[2] = z1;
#else
	uint64_t z0 = (uint64_t)a[0] + (uint64_t)b[0];
	uint64_t z1 = (uint64_t)a[1] + (uint64_t)b[1] + (z0 >> 32);
	uint32_t z2 = a[2] + b[2] + (uint32_t)(z1 >> 32);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)z1;
	d[2] = z2;
#endif
}

/* d <- a - b */
static inline void
rx_sub(rx d, const rx a, const rx b)
{
#if defined __x86_64__ || defined _M_X64
	uint64_t a0 = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t b0 = (uint64_t)b[0] | ((uint64_t)b[1] << 32);
	unsigned long long z0;
	unsigned int z1;
	unsigned char c = _subborrow_u64(0, a0, b0, &z0);
	(void)_subborrow_u32(c, a[2], b[2], &z1);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)(z0 >> 32);
	d[2] = z1;
#else
	uint64_t z0 = (uint64_t)a[0] - (uint64_t)b[0];
	uint64_t z1 = (uint64_t)a[1] - (uint64_t)b[1] - (z0 >> 63);
	uint32_t z2 = a[2] - b[2] - (uint32_t)(z1 >> 63);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)z1;
	d[2] = z2;
#endif
}

/* d <- a - b, borrow is returned:
   If unsigned(a) < unsigned(b), then 0xFFFFFFFF is returned, otherwise
   0x00000000 is returned. This works on the *unsigned* interpretation
   of the source values a and b. */
static inline uint32_t
rx_sub_borrow(rx d, const rx a, const rx b)
{
#if defined __x86_64__ || defined _M_X64
	uint64_t a0 = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t b0 = (uint64_t)b[0] | ((uint64_t)b[1] << 32);
	unsigned long long z0;
	unsigned int z1;
	unsigned char c = _subborrow_u64(0, a0, b0, &z0);
	c = _subborrow_u32(c, a[2], b[2], &z1);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)(z0 >> 32);
	d[2] = z1;
	return -(uint32_t)c;
#else
	uint64_t z0 = (uint64_t)a[0] - (uint64_t)b[0];
	uint64_t z1 = (uint64_t)a[1] - (uint64_t)b[1] - (z0 >> 63);
	uint64_t z2 = (uint64_t)a[2] - (uint64_t)b[2] - (z1 >> 63);
	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)z1;
	d[2] = (uint32_t)z2;
	return (uint32_t)(z2 >> 32);
#endif
}

/* d <- -a */
static inline void
rx_neg(rx d, const rx a)
{
	static const rx zero = { 0, 0, 0 };
	rx_sub(d, zero, a);
}

/* d <- a*b
   Rounding toward -infinity is applied. */
#if FNDSA_ASM_CORTEXM4
#define rx_mul   fndsa_rx_mul
void rx_mul(rx d, const rx a, const rx b);
#else
static inline void
rx_mul(rx d, const rx a, const rx b)
{
#if (defined __GNUC__ || defined __clang__) && defined __SIZEOF_INT128__
	uint64_t al = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t bl = (uint64_t)b[0] | ((uint64_t)b[1] << 32);
	int32_t ah = *(int32_t *)&a[2];
	int32_t bh = *(int32_t *)&b[2];

	unsigned __int128 t0 = (unsigned __int128)al * (unsigned __int128)bl;
	unsigned __int128 t1 = (unsigned __int128)((__int128)al * (__int128)bh);
	unsigned __int128 t2 = (unsigned __int128)((__int128)ah * (__int128)bl);
	t0 = (t0 + 0x8000000000000000) >> 64;
	unsigned __int128 z = t0 + t1 + t2;

	d[0] = (uint32_t)z;
	d[1] = (uint32_t)((uint64_t)z >> 32);
	d[2] = (uint32_t)(z >> 64) + (uint32_t)ah * (uint32_t)bh;
#elif defined _M_X64
	uint64_t al = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t bl = (uint64_t)b[0] | ((uint64_t)b[1] << 32);
	int32_t ah = *(int32_t *)&a[2];
	int32_t bh = *(int32_t *)&b[2];

	unsigned __int64 t0l, t0h;
	t0l = _umul128(al, bl, &t0h);
	unsigned __int64 t1l, t1h;
	t1l = _umul128(al, (uint64_t)bh, &t1h);
	unsigned __int64 t2l, t2h;
	t2l = _umul128((uint64_t)ah, bl, &t2h);
	t1h -= (uint64_t)((int64_t)bh >> 32) & al;
	t2h -= (uint64_t)((int64_t)ah >> 32) & bl;
	t0h += t0l >> 63;
	unsigned long long z0, z1;
	unsigned char c = _addcarry_u64(0, t0h, t1l, &z0);
	(void)_addcarry_u64(c, 0, t1h, &z1);
	c = _addcarry_u64(0, t2l, z0, &z0);
	(void)_addcarry_u64(c, t2h, z1, &z1);

	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)(z0 >> 32);
	d[2] = (uint32_t)z1 + (uint32_t)ah * (uint32_t)bh;
#else
	/* We make a 3x3 unsigned multiplication, but the result is
	   truncated: if u = a*b:
	     - we do not care about u[0]
	     - u[1] is used only for rounding
	     - u[5] is not needed either */
	uint64_t w = (uint64_t)a[0] * (uint64_t)b[0];
	w = (uint64_t)a[0] * (uint64_t)b[1] + (w >> 32);
	uint32_t u1 = w;
	w = (uint64_t)a[0] * (uint64_t)b[2] + (w >> 32);
	uint32_t u2 = w;
	uint32_t u3 = w >> 32;

	w = (uint64_t)a[1] * (uint64_t)b[0] + u1;
	u1 = w;
	w = (uint64_t)a[1] * (uint64_t)b[1] + (w >> 32) + u2;
	u2 = w;
	w = (uint64_t)a[1] * (uint64_t)b[2] + (w >> 32) + u3;
	u3 = w;
	uint64_t u4 = w >> 32;

	w = (uint64_t)a[2] * (uint64_t)b[0] + u2;
	u2 = w;
	w = (uint64_t)a[2] * (uint64_t)b[1] + (w >> 32) + u3;
	u3 = w;
	u4 = a[2] * b[2] + (uint32_t)(w >> 32) + u4;

	/* Sign adjustments:
	     if a < 0 then subtract b*2^32
	     if b < 0 then subtract a*2^32 */
	uint32_t ma = (uint32_t)(*(int32_t *)&a[2] >> 31);
	uint32_t mb = (uint32_t)(*(int32_t *)&b[2] >> 31);
	w = (uint64_t)u3 - (uint64_t)(ma & b[0]);
	u3 = w;
	u4 = u4 - (ma & b[1]) + (w >> 32);
	w = (uint64_t)u3 - (uint64_t)(mb & a[0]);
	u3 = w;
	u4 = u4 - (mb & a[1]) + (w >> 32);

	/* Rounding: add 2^31 to u1, propagate carry. */
	w = (uint64_t)u1 + 0x80000000;
	w = (uint64_t)u2 + (w >> 32);
	u2 = w;
	w = (uint64_t)u3 + (w >> 32);
	u3 = w;
	u4 = u4 + (uint32_t)(w >> 32);

	d[0] = u2;
	d[1] = u3;
	d[2] = u4;
#endif
}
#endif

/* d <- a*a
   Rounding toward -infinity is applied. */
static inline void
rx_sqr(rx d, const rx a)
{
	rx_mul(d, a, a);
}

/* d <- a*b
   Operand b is in [0,1], passed as an integer (scaled by 2^64).
   Rounding toward -infinity is applied. */
#if FNDSA_ASM_CORTEXM4
#define rx_mul_ufrac   fndsa_rx_mul_ufrac
void rx_mul_ufrac(rx d, const rx a, uint64_t b);
#else
static inline void
rx_mul_ufrac(rx d, const rx a, uint64_t b)
{
#if (defined __GNUC__ || defined __clang__) && defined __SIZEOF_INT128__
	uint64_t al = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	int32_t ah = *(int32_t *)&a[2];

	unsigned __int128 t0 = (unsigned __int128)al * (unsigned __int128)b;
	unsigned __int128 t2 = (unsigned __int128)((__int128)ah * (__int128)b);
	t0 = (t0 + 0x8000000000000000) >> 64;
	unsigned __int128 z = t0 + t2;

	d[0] = (uint32_t)z;
	d[1] = (uint32_t)((uint64_t)z >> 32);
	d[2] = (uint32_t)(z >> 64);
#elif defined _M_X64
	uint64_t al = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	int32_t ah = *(int32_t *)&a[2];

	unsigned __int64 t0l, t0h;
	t0l = _umul128(al, b, &t0h);
	unsigned __int64 t2l, t2h;
	t2l = _umul128((uint64_t)ah, b, &t2h);
	t2h -= (uint64_t)((int64_t)ah >> 32) & b;
	t0h += t0l >> 63;
	unsigned long long z0, z1;
	unsigned char c = _addcarry_u64(0, t0h, t2l, &z0);
	(void)_addcarry_u64(c, 0, t2h, &z1);

	d[0] = (uint32_t)z0;
	d[1] = (uint32_t)(z0 >> 32);
	d[2] = (uint32_t)z1;
#else
	/* We make a 3x2 unsigned multiplication, but the result is
	   truncated: if u = a*b:
	     - we do not care about u[0]
	     - u[1] is used only for rounding
	     - u[5] is not needed either */
	uint32_t b0 = (uint32_t)b;
	uint32_t b1 = (uint32_t)(b >> 32);
	uint64_t w = (uint64_t)a[0] * (uint64_t)b0;
	w = (uint64_t)a[0] * (uint64_t)b1 + (w >> 32);
	uint32_t u1 = (uint32_t)w;
	uint32_t u2 = (uint32_t)(w >> 32);

	w = (uint64_t)a[1] * (uint64_t)b0 + u1;
	u1 = w;
	w = (uint64_t)a[1] * (uint64_t)b1 + (w >> 32) + u2;
	u2 = w;
	uint32_t u3 = (uint32_t)(w >> 32);

	w = (uint64_t)a[2] * (uint64_t)b0 + u2;
	u2 = w;
	w = (uint64_t)a[2] * (uint64_t)b1 + (w >> 32) + u3;
	u3 = w;
	uint32_t u4 = (uint32_t)(w >> 32);

	/* Sign adjustments: if a < 0 then subtract b*2^32 */
	uint32_t ma = (uint32_t)(*(int32_t *)&a[2] >> 31);
	w = (uint64_t)u3 - (uint64_t)(ma & b0);
	u3 = w;
	u4 = u4 - (ma & b1) + (w >> 32);

	/* Rounding: add 2^31 to u1, propagate carry. */
	w = (uint64_t)u1 + 0x80000000;
	w = (uint64_t)u2 + (w >> 32);
	u2 = w;
	w = (uint64_t)u3 + (w >> 32);
	u3 = w;
	u4 = u4 + (uint32_t)(w >> 32);

	d[0] = u2;
	d[1] = u3;
	d[2] = u4;
#endif
}
#endif

/* return a*b
   The two operands and the returned value, all in [0,1], are represented
   by integers, with scaling by 2^64.
   Rounding toward -infinity is applied. */
static inline uint64_t
rx_mul_ufrac_ufrac(uint64_t a, uint64_t b)
{
#if (defined __GNUC__ || defined __clang__) && defined __SIZEOF_INT128__
	unsigned __int128 z = (unsigned __int128)a * (unsigned __int128)b;
	z = (z + 0x8000000000000000) >> 64;
	return (uint64_t)z;
#elif defined _M_X64
	unsigned __int64 z0, z1;
	z0 = _umul128(a, b, &z1);
	z1 += z0 >> 63;
	return (uint64_t)z1;
#else
	/* We make a 2x2 unsigned multiplication, but the result is
	   truncated: if u = a*b:
	     - we do not care about u[0]
	     - u[1] is used only for rounding */
	uint32_t a0 = (uint32_t)a;
	uint32_t a1 = (uint32_t)(a >> 32);
	uint32_t b0 = (uint32_t)b;
	uint32_t b1 = (uint32_t)(b >> 32);
	uint64_t w = (uint64_t)a0 * (uint64_t)b0;
	w = (uint64_t)a0 * (uint64_t)b1 + (w >> 32);
	uint32_t u1 = (uint32_t)w;
	uint32_t u2 = (uint32_t)(w >> 32);
	w = (uint64_t)a1 * (uint64_t)b0 + u1;
	u1 = w;
	w = (uint64_t)a1 * (uint64_t)b1 + (w >> 32) + u2;
	u2 = w;
	uint32_t u3 = (uint32_t)(w >> 32);

	/* Rounding: add 2^31 to u1, propagate carry. */
	w = (uint64_t)u1 + 0x80000000;
	w = (uint64_t)u2 + (w >> 32);
	u2 = w;
	u3 += (uint32_t)(w >> 32);

	return (uint64_t)u2 | ((uint64_t)u3 << 32);
#endif
}

/* d <- a/b
   Special conditions:
     - operands must be such that there is no overflow
     - divisor must be positive
   Output is approximate but should be within 2^(-51) of exact result. */
#define rx_div_spec   fndsa_rx_div_spec
void rx_div_spec(rx d, const rx a, const rx b);

/* d <- 1/b
   Same rules as rx_div_spec(), with a = 1. */
static inline void
rx_inv_spec(rx d, const rx b)
{
	static const rx RX_ONE = { 0, 0, 1 };
	rx_div_spec(d, RX_ONE, b);
}

/* Square root.
   Special conditions:
       6327 < a < 16823
   Output is approximate but should be within 2^(-49) of exact result. */
#define rx_sqrt_spec   fndsa_rx_sqrt_spec
void rx_sqrt_spec(rx d, const rx a);

/* d <- x  (conversion integer to fixed-point) */
static inline void
rx_of32(rx d, int32_t x)
{
	d[0] = 0;
	d[1] = 0;
	d[2] = (uint32_t)x;
}

/* d <- x/2^64 */
static inline void
rx_of_ufrac(rx d, uint64_t x)
{
	d[0] = (uint32_t)x;
	d[1] = (uint32_t)(x >> 32);
	d[2] = 0;
}

/* Return floor(a) (rounding toward -infinity). */
static inline int32_t
rx_floor(const rx a)
{
	return *(int32_t *)&a[2];
}

/* Return trunc(a) (rounding toward zero). */
static inline int32_t
rx_trunc(const rx a)
{
	/* For non-negative values, return a[2].
	   For negative values, return a[2] + 1, unless a[0] and a[1]
	   are both zero, in which case a[2] is returned. */
	uint32_t m = (uint32_t)(*(int32_t *)&a[2] >> 31);
	uint64_t w = (uint64_t)(a[0] | a[1]) + (uint64_t)m;
	return *(int32_t *)&a[2] + (int32_t)(w >> 32);
}

/* Return (a - floor(a))*2^64 (fractional part, scaled up by 2^64). */
static inline uint64_t
rx_ufrac(const rx a)
{
	return (uint64_t)a[0] | ((uint64_t)a[1] << 32);
}

/* Return round(a) (nearest integer, ties broken toward +infinity). */
static inline int32_t
rx_rint(const rx a)
{
	return *(int32_t *)&a[2] + (int32_t)(a[1] >> 31);
}

/* d <- a/2
   Output is rounded toward -infinity. */
static inline void
rx_half(rx d, const rx a)
{
#if FNDSA_64
	uint64_t a0 = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t d0 = (a0 >> 1) | ((uint64_t)a[2] << 63);
	uint32_t d2 = (uint32_t)(*(int32_t *)&a[2] >> 1);
	d[0] = (uint32_t)d0;
	d[1] = (uint32_t)(d0 >> 32);
	d[2] = d2;
#else
	uint32_t d0 = (a[0] >> 1) | (a[1] << 31);
	uint32_t d1 = (a[1] >> 1) | (a[2] << 31);
	uint32_t d2 = (uint32_t)(*(int32_t *)&a[2] >> 1);
	d[0] = d0;
	d[1] = d1;
	d[2] = d2;
#endif
}

/* d <- a/2^e
   e MUST be in [1,31].
   Output is rounded toward -infinity. */
static inline void
rx_div2e(rx d, const rx a, unsigned e)
{
#if FNDSA_64
	uint64_t a0 = (uint64_t)a[0] | ((uint64_t)a[1] << 32);
	uint64_t d0 = (a0 >> e) | ((uint64_t)a[2] << (64 - e));
	uint32_t d2 = (uint32_t)(*(int32_t *)&a[2] >> e);
	d[0] = (uint32_t)d0;
	d[1] = (uint32_t)(d0 >> 32);
	d[2] = d2;
#else
	uint32_t d0 = (a[0] >> e) | (a[1] << (32 - e));
	uint32_t d1 = (a[1] >> e) | (a[2] << (32 - e));
	uint32_t d2 = (uint32_t)(*(int32_t *)&a[2] >> e);
	d[0] = d0;
	d[1] = d1;
	d[2] = d2;
#endif
}

/* d <- a*2^e
   e MUST be in [1,31].
   Value MUST NOT overflow. */
static inline void
rx_mul2e(rx d, const rx a, unsigned e)
{
	uint32_t d0 = a[0] << e;
	uint32_t d1 = (a[0] >> (32 - e)) | (a[1] << e);
	uint32_t d2 = (a[1] >> (32 - e)) | (a[2] << e);
	d[0] = d0;
	d[1] = d1;
	d[2] = d2;
}

/* d <- a*2^32
   Value MUST NOT overflow. */
static inline void
rx_mul2p32(rx d, const rx a)
{
	d[2] = a[1];
	d[1] = a[0];
	d[0] = 0;
}

/* Complex multiplication. Note that we use here a three-multiplications
   method:
     t0 <- a_re*b_re
     t1 <- a_im*b_im
     t2 <- (a_re + a_im)*(b_re + b_im)
     d <- (t0 - t1) + i*(t2 - t0 - t1)  */
#define RXC_MUL(d_re, d_im, a_re, a_im, b_re, b_im)   do { \
		rx rxct_m0, rxct_m1, rxct_m2, rxct_m3; \
		rx_add(rxct_m0, a_re, a_im); \
		rx_add(rxct_m1, b_re, b_im); \
		rx_mul(rxct_m0, rxct_m0, rxct_m1); \
		rx_mul(rxct_m2, a_re, b_re); \
		rx_mul(rxct_m3, a_im, b_im); \
		rx_sub(rxct_m0, rxct_m0, rxct_m2); \
		rx_sub(d_re, rxct_m2, rxct_m3); \
		rx_sub(d_im, rxct_m0, rxct_m3); \
	} while (0)
/* Alternate version with four multiplications.
     t0 <- a_re*b_re
     t1 <- a_im*b_im
     t2 <- a_re*b_im
     t3 <- a_im*b_re
   d <- (t0 - t1) + i*(t2 + t3)

#define RXC_MUL(d_re, d_im, a_re, a_im, b_re, b_im)   do { \
		rx rxct_m0, rxct_m1, rxct_m2, rxct_m3; \
		rx_mul(rxct_m0, a_re, b_re); \
		rx_mul(rxct_m1, a_im, b_im); \
		rx_mul(rxct_m2, a_re, b_im); \
		rx_mul(rxct_m3, a_im, b_re); \
		rx_sub(d_re, rxct_m0, rxct_m1); \
		rx_add(d_im, rxct_m2, rxct_m3); \
	} while (0)
*/

/* Right-shift a 64-bit unsigned value by a possibly secret shift count.
   The shift count is between 0 and 63 (inclusive). On some 32-bit
   architectures (especially old 32-bit PowerPC), support of 64-bit
   shifts involves conditional jumps, i.e. leaking through timing
   measurements whether the shift was 32-bit or 64-bit, which is why
   this function is defined.

   On ARM Cortex-M4, compilers apply a branchless sequence of 8 instructions
   which leverages the fact that the shift opcodes actually use an 8-bit
   count (and not 5-bit). */
static inline uint64_t
rx_ursh(uint64_t x, int n)
{
#if FNDSA_64 || FNDSA_ASM_CORTEXM4
	return x >> n;
#else
	x ^= (x ^ (x >> 32)) & -(uint64_t)(n >> 5);
	return x >> (n & 31);
#endif
}

/* ==================================================================== */
/*
 * Fixed-point polynomials.
 *
 * Polynomials are modulo X^n+1 and have real coefficients. When converted
 * to FFT format, they have n/2 complex coefficients, with the real and
 * imaginary parts separated (real in f[i] and imaginary in f[i+n/2]).
 *
 * Unless specified explicitly, the functions below can work with
 * polynomials in both real and FFT representations (if using several
 * operands, then they must all be in the same representation).
 */

/* Convert a self-adjoint polynomial with integer coefficients (n/2
   elements of type int16_t) into its FFT representation (n/2 real
   numbers of type rx).
   The source (a) and destination (d) may overlap. It is most efficient
   if the source and destination start at the same address. */
#define xpoly_FFT_int16_selfadj   fndsa_xpoly_FFT_int16_selfadj
void xpoly_FFT_int16_selfadj(unsigned logn, rx *d, const int16_t *a);

/* Convert a self-adjoint polynomial with integer coefficients (n/2
   elements of type int32_t) into its FFT representation (n/2 real
   numbers of type rx).
   The source (a) and destination (d) may overlap. It is most efficient
   if the source and destination ends are on the same address. */
#define xpoly_FFT_int32_selfadj   fndsa_xpoly_FFT_int32_selfadj
void xpoly_FFT_int32_selfadj(unsigned logn, rx *d, const int32_t *a);

/* Convert a polynomial with integer coefficients (n elements of type
   int32_t) into its FFT representation (n elements of type rx).
   The source (a) and destination (d) may overlap. It is most efficient
   if the source and destination ends are on the same address. */
#define xpoly_FFT_int32   fndsa_xpoly_FFT_int32
void xpoly_FFT_int32(unsigned logn, rx *d, const int32_t *a);

/* Add polynomial b to polynomial a. */
#define xpoly_add   fndsa_xpoly_add
void xpoly_add(unsigned logn, rx *a, const rx *b);

/* Subtract polynomial b from polynomial a. */
#define xpoly_sub   fndsa_xpoly_sub
void xpoly_sub(unsigned logn, rx *a, const rx *b);

/* Negate polynomial a (in-place). */
#define xpoly_neg   fndsa_xpoly_neg
void xpoly_neg(unsigned logn, rx *a);

/* Multiply polynomial a with polynomial b (FFT representation only). */
#define xpoly_mul_fft   fndsa_xpoly_mul_fft
void xpoly_mul_fft(unsigned logn, rx *a, const rx *b);

/* Divide polynomial a by self-adjoint polynomial b (FFT representation only,
   polynomial b may be half-length). All FFT coefficients of b must be
   positive (this is necessarily the case when b = f*adj(f) + g*adj(g)
   for two real polynomials f and g). */
#define xpoly_div_selfadj_spec_fft   fndsa_xpoly_div_selfadj_spec_fft
void xpoly_div_selfadj_spec_fft(unsigned logn, rx *a, const rx *b);

/* Multiply polynomial a by polynomial (X+1)*b, result in d. Polynomials
   are in FFT representation. Polynomial a and d are generic (n elements).
   Polynomial b is self-adjoint and represents (X+1)*b, which is 1/X-adjoint.
   d[] and a[] may be the same array; no other overlap allowed.
   Degree 2^logn MUST NOT be larger than 512. */
#define xpoly_mul_iXadj_fft   fndsa_xpoly_mul_iXadj_fft
void xpoly_mul_iXadj_fft(unsigned logn, rx *d, const rx *a, const rx *b);

/* Perform an LDL decomposition.
   Self-adjoint polynomials a0 and b1 are provided, with b1 = (X+1)*a1,
   and (a0, a1) being the split of a source self-adjoint polynomial a.
   This function computes polynomials:
      L10 = adj(a1)/a0
      d = a0 - a1*adj(a1)/a0  (self-adjoint)
      e = L10/(X+1)  (self-adjoint)
   Polynomials a0, b1, d and e use n/2 elements each. */
#define xpoly_LDL_fft   fndsa_xpoly_LDL_fft
void xpoly_LDL_fft(unsigned logn, rx *d, rx *e,
	const rx *a0, const rx *b1);

/* Split operation on a polynomial: for input polynomial f,
   half-size polynomials f0 and f1 (modulo X^(n/2)+1) are such that
   f = f0(x^2) + x*f1(x^2). Polynomial f0 is written over the first
   half of polynomial f; the other half of f is consumed. All polynomials
   are in FFT representation. */
#define xpoly_split_fft   fndsa_xpoly_split_fft
void xpoly_split_fft(unsigned logn, rx *f, rx *f1);

/* Split a self-adjoint polynomial; the second output is X-adjoint and is
   represented by the self-adjoint polynomial b1 = a1*(X+1). Polynomial a
   uses n/2 elements, and a0 and b1 use n/4 elements each. All polynomials
   are in FFT representation. */
#define xpoly_split_selfadj_fft   fndsa_xpoly_split_selfadj_fft
void xpoly_split_selfadj_fft(unsigned logn, rx *a0, rx *b1, const rx *a);

/* Merge operation on polynomials: for input half-size polynomials f0
   and f1 (modulo X^(n/2)+1), compute f = f0(x^2) + x*f1(x^2). All
   polynomials are in FFT representation. */
#define xpoly_merge_fft   fndsa_xpoly_merge_fft
void xpoly_merge_fft(unsigned logn, rx *f, const rx *f0, const rx *f1);

/* Array of constants for some FFT operations; this is public so that the
   constants can be used from the inverse FFT function located in
   test_fndsa.c. */
#define x_SIN   fndsa_x_SIN
extern const uint64_t x_SIN[];

/* ==================================================================== */
/*
 * Gaussian sampling.
 *
 * The sampler follows a Gaussian distribution whose centre and standard
 * deviations are not integral, dynamically obtained, and secret. The
 * sampler state includes a PRNG, from which random bytes are obtained
 * to sample value with a given bimodal Gaussian used as source for
 * rejection sampling of the target distribution.
 */

typedef struct {
	shake_context pc;    /* context for sampler PRNG */
	int16_t *dz;         /* extra input / dest for sampled integers */
	unsigned off;        /* current offset (reversed) in dz */
	unsigned logn;       /* top-level degree */
} sampler_state;

/* Flags for ffsamp_fft_left() and ffsamp_fft_right():
     FLAG_INIT0   if set, then t == 0 (implicit)
     FLAG_NOOUT   if set, then output t - z is not produced  */
#define FLAG_INIT0   1
#define FLAG_NOOUT   2

/* Sample the next small integer. Parameters are:
      ss       sampler state
      mu       distribution centre
      isigma   inverse of the distribution standard deviation  */
#define x_sampler_next   fndsa_x_sampler_next
int32_t x_sampler_next(sampler_state *ss, const rx mu, const rx isigma);

/* Apply Fast Fourier sampling (alt code):
      ss      sampler state (initialized)
      flags   init0 and noout flags
      tmp     parameters and free space (total: 16*n bytes):
          t      (12*n bytes) (implicit if init0 != 0)
          a      (6*n bytes, self-adjoint)
          free   (6*n bytes)
   On output, t - z is returned in tmp (12*n bytes) */
#define x_ffsamp_fft_left   fndsa_x_ffsamp_fft_left
void x_ffsamp_fft_left(sampler_state *ss, unsigned flags, rx *tmp);

/* Same as ffsamp_fft_left(), but for the right sub-tree. Self-adjoint
   polynomial a is f*adj(f) + g*adj(g), i.e. the source polynomial for
   the left sub-tree, which this function turns into the right-subtree
   through symplecticity. */
#define x_ffsamp_fft_right   fndsa_x_ffsamp_fft_right
void x_ffsamp_fft_right(sampler_state *ss, unsigned flags, rx *tmp);

/* ==================================================================== */
/*
 * Internal signing function.
 */

/* Make a 40-byte seed (into seedbuf[]) from the provided seed. If seed
   is NULL, then the system RNG is used (if available). Moreover, the
   private key (sign_key_fg) and message representative (mu) are used
   for "hedging" (tolerating a system RNG of poor quality). Returned value
   is 1 on success, 0 on error; an error may be reported if the system RNG
   is used and it reports a failure.

   If sign_key is NULL, then this function expects seedbuf[] to already
   contain SHAKE256(f||g)[40].

   impl_id shall be a pointer to an 8-byte fixed identifier that qualifies
   the exact processing of values performed by the implementation. If an
   implementation variant does not yield provided exactly the same values
   to the Gaussian sampler (for given key, message and seed), then this
   implementation identifier MUST be changed to avoid information leaks
   through occasional divergences.

   This is defined as a separate, non-inlined function so that the space
   taken by the SHAKE256 context is reused. */
#define make_seedbuf   fndsa_make_seedbuf
int make_seedbuf(uint8_t *seedbuf, const uint8_t *sign_key,
        const uint8_t *mu, const void *seed, size_t seed_len,
	const void *impl_id);

#define SEEDBUF_LEN   40

/* Internal API for supporting algorithm variants.

      make_nonce   Generate the signature nonce, given the (already flipped)
                   SHAKE context as random source. The output signature
                   buffer is provided for storage.

      make_c       Produce the polynomial c. The output signature buffer
                   is provided since it will contain the nonce.

      sig_type     signature type (standard, key recovery, message recovery)
*/
typedef struct fndsa_variant_ fndsa_variant;
struct fndsa_variant_ {
	void (*make_nonce)(fndsa_variant *fv,
		unsigned logn, shake_context *sc, uint8_t *sig);
	void (*make_c)(fndsa_variant *fv,
		unsigned logn, uint8_t *sig, uint16_t *c);
	unsigned sig_type;
};

/* Internal signing function. The sign_key_fgF pointer designates the
   encoded f, g and F from the private key (the leading byte is skipped).
   The message representative (mu) and the random seed to work on (seed,
   size seed_len bytes) are provided, as well as the output buffer (sig)
   and the temporary area (tmp). The caller has verified that the
   sig and tmp buffers are large enough, and that tmp is 32-byte aligned.

   Returned value is the signature size (in bytes), or 0 on error.
   An error may conceptually happen if the private key is incorrect/invalid.

   tmp size: 28*n bytes (32-byte aligned). */
#define sign_core   fndsa_sign_core
size_t sign_core(unsigned logn,
	const uint8_t *sign_key_fgF, fndsa_variant *fv,
	const uint8_t *seed, size_t seed_len, uint8_t *sig, void *tmp);

/* ==================================================================== */
/*
 * Computations modulo q' = 61441.
 *
 * Polynomials are held in arrays of integers. The following
 * representations are used:
 *   name       type     range
 *  --------------------------------
 *   small     int8_t    [-127,+127]
 *   signed   uint16_t   [-30720,+30720] (int16_t cast to uint16_t)
 *   mod      uint16_t   (internal)
 *   ntt      uint16_t   (internal)
 *
 * The internal convention may change depending on the implementation.
 * For most architectures, internal representation uses [0,q'] (i.e. zero
 * has two possible representations).
 */

#define MQ_MOD          12289
#define MQ2_MOD         18433
#define MQ2_IQ              3      /*  1/q mod q' */

#define mq2poly_small_to_mod           fndsa_mq2poly_small_to_mod
#define mq2poly_signed_to_mod          fndsa_mq2poly_signed_to_mod
#define mq2poly_unsigned_to_mod        fndsa_mq2poly_unsigned_to_mod
#define mq2poly_mod_to_signed          fndsa_mq2poly_mod_to_signed
#define mq2poly_NTT                    fndsa_mq2poly_NTT
#define mq2poly_iNTT                   fndsa_mq2poly_iNTT
#define mq2poly_mul_ntt                fndsa_mq2poly_mul_ntt
#define mq2poly_muladj_ntt             fndsa_mq2poly_muladj_ntt
#define mq2poly_muladj_x2_ntt          fndsa_mq2poly_muladj_x2_ntt
#define mq2poly_sub                    fndsa_mq2poly_sub
#define mq2poly_add                    fndsa_mq2poly_add
#define mq2poly_mulconst               fndsa_mq2poly_mulconst
#define mq2poly_addmulconst            fndsa_mq2poly_addmulconst
#define mq2_GM                         fndsa_mq2_GM
#define mq2_iGM                        fndsa_mq2_iGM
#define mq2mqpoly_adjust_save          fndsa_mq2mqpoly_adjust_save
#define mq2mqpoly_adjust               fndsa_mq2mqpoly_adjust
#define mq2mqpoly_CRT                  fndsa_mq2mqpoly_CRT

/* Convert a polynomial from small (f) to mod (d). */
void mq2poly_small_to_mod(unsigned logn, const int8_t *f, uint16_t *d);

/* Convert a polynomial from signed to mod (in-place). */
void mq2poly_signed_to_mod(unsigned logn, uint16_t *d);

/* Convert a polynomial from unsigned to mod (in-place). */
#if FNDSA_ASM_CORTEXM4
static inline void
mq2poly_unsigned_to_mod(unsigned logn, uint16_t *d)
{
	/* For integers modulo q', we use the "relaxed" format with
	   values in [0, q'], so the unsigned values in [0, q] are
	   already fine. */
	(void)logn;
	(void)d;
}
#else
void mq2poly_unsigned_to_mod(unsigned logn, uint16_t *d);
#endif

/* Convert a polynomial from mod to signed (in-place). */
void mq2poly_mod_to_signed(unsigned logn, uint16_t *d);

/* Convert a polynomial from mod to ntt (in-place). */
void mq2poly_NTT(unsigned logn, uint16_t *d);

/* Convert a polynomial from ntt to mod (in-place). */
void mq2poly_iNTT(unsigned logn, uint16_t *d);

/* Multiply polynomial a by polynomial b (both in ntt representation). */
void mq2poly_mul_ntt(unsigned logn, uint16_t *a, const uint16_t *b);

/* a <- a*adj(b) (ntt representation.) */
void mq2poly_muladj_ntt(unsigned logn, uint16_t *a, const uint16_t *b);

/* a <- a*adj(a) + b*adj(b) (ntt representation). */
void mq2poly_muladj_x2_ntt(unsigned logn, uint16_t *a, const uint16_t *b);

/* Subtract polynomial b from polynomial a (both must be in int
   representation, or both must be in ntt representation). */
void mq2poly_sub(unsigned logn, uint16_t *a, const uint16_t *b);

/* Add polynomial b to polynomial a (both must be in int
   representation, or both must be in ntt representation). */
void mq2poly_add(unsigned logn, uint16_t *a, const uint16_t *b);

/* For all i = 0 to n-1, replace a[i] with a[i]*m (mod q'). */
void mq2poly_mulconst(unsigned logn, uint16_t *a, unsigned m);

/* For all i = 0 to n-1, replace a[i] with (a[i] + s)*m (mod q'). */
void mq2poly_addmulconst(unsigned logn, uint16_t *a, unsigned m, unsigned s);

/* Given polynomials a and b, representing the same polynomial modulo q'
   and modulo q, respectively (both normalized), adjust the value of a
   so that it matches b. This function assumes that the underlying polynomial
   has coefficients in [-q' + 1, q' - 1]. Adjustment bits are saved into
   adj[] (which must be large enough to contain n bits). */
void mq2mqpoly_adjust_save(unsigned logn,
	int16_t *a, const int16_t *b, uint8_t *adj);

/* Reduced variant of mq2mqpoly_adjust_save(), assuming that adjustment bits
   have already been set in adj[]. */
void mq2mqpoly_adjust(unsigned logn, int16_t *a, const uint8_t *adj);

/* Given a mod q and a mod q' (both normalized), compute a mod q*q' into d
   (normalized) */
void mq2mqpoly_CRT(unsigned logn, int32_t *d,
	const int16_t *aq, const int16_t *aq2);

/* Tables of constants for NTT and inverse NTT modulo q. */
extern const uint16_t mq2_GM[];
extern const uint16_t mq2_iGM[];

/* ==================================================================== */
/* Expanded key support. */

/* Expand the private key.
   Key degree is 2^logn; valid logn values depend on the 'weak' flag:
      weak != 0:  2 <= logn <= 8
      weak == 0:  9 <= logn <= 10
   Returned value is the actual pointer to the expanded key (aligned
   as necessary). NULL is returned on error. Error conditions include:
      signing key has an invalid format
      signing key has an invalid length
      degree of signing key is out-of-range
      degree of signing key does not match the "weak" parameter
      norm L2 of (f,g) is too large
      output buffer is too small */
#define expand_skey   fndsa_expand_skey
void *expand_skey(int weak,
	const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_max_len);

/* Internal signing function, using an expanded key.
   ekbuf points to the data immediately following the one-byte header
   of the expanded key.
   The message representative (mu) and the random seed to work on (seed,
   size seed_len bytes) are provided, as well as the output buffer (sig)
   and the temporary area (tmp). The caller has verified that the
   sig and tmp buffers are large enough, and that tmp is 32-byte aligned.

   Returned value is the signature size (in bytes), or 0 on error.
   An error may conceptually happen if the private key is incorrect/invalid.

   tmp size: 34*n bytes (32-byte aligned). */
#define sign_core_expkey   fndsa_sign_core_expkey
size_t sign_core_expkey(unsigned logn, const void *ekbuf, const uint8_t *mu,
	const uint8_t *seed, size_t seed_len, uint8_t *sig, void *tmp);

/* ffSampling() on the left sub-tree.
     ss        sampler state
     xnodes    sub-tree root pointer
     xleaves   pointer to the leaves array
     tmp       input buffer (contains t0') and temporary area
   tmp should have size 24*n bytes. */
#define x_ffsamp_expkey_fft_left   fndsa_x_ffsamp_expkey_fft_left
void x_ffsamp_expkey_fft_left(sampler_state *ss,
	const rx *xnodes, const rx *xleaves, rx *tmp);

/* ffSampling() on the right sub-tree.
     ss        sampler state
     xnodes    sub-tree root pointer
     xleaves   pointer to the leaves array
     tmp       output buffer (receives t1 - z1) and temporary area
   tmp should have size 24*n bytes. */
#define x_ffsamp_expkey_fft_right   fndsa_x_ffsamp_expkey_fft_right
void x_ffsamp_expkey_fft_right(sampler_state *ss,
	const rx *xnodes, const rx *xleaves, rx *tmp);

/* ==================================================================== */
/* Test / data gathering facilities.
   This is meant only for research/statistical purposes, and must not
   be defined in production builds. */
#ifndef FNDSA_STATS
#define FNDSA_STATS   0
#endif

#if FNDSA_STATS
void fndsa_stats_start_sign(unsigned logn, const uint8_t *sign_key,
	const uint8_t *mu, const uint8_t *seedbuf);
void fndsa_stats_start_iteration(unsigned counter, const uint8_t *nonce);
void fndsa_stats_hash_to_point(unsigned logn, const uint16_t *c);
void fndsa_stats_sample(const rx t, const rx isigma, int32_t z);
#endif

/* ==================================================================== */

#endif
