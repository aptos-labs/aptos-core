#ifndef FNDSA_KGEN_INNER_H__
#define FNDSA_KGEN_INNER_H__

/* ==================================================================== */
/*
 * This file includes declarations used by the key pair generation code
 * only.
 */

#include "inner.h"

/* ==================================================================== */
/*
 * Computations modulo small 31-bit primes.
 *
 * We consider only primes p such that:
 *   (4/3)*2^30 < p < 2^31     (i.e. 2*p < 2^32 < 3*p)
 *   p = 1 mod 2048
 * Values modulo p are held in a 32-bit type (uint32_t) in the [0,p-1] range.
 * We define R = 2^32 mod p. The Montgomery representation of x modulo p
 * is x*R mod p (in the [0,p-1] range).
 *
 * The signed normalized representation of x is the unique integer y such
 * that y = x mod p and -p/2 < y < p/2.
 *
 * The PRIMES[] array contains the largest such primes, in descending
 * order. Six values are provided for each prime p:
 *   p     modulus
 *   p0i   -1/p mod 2^32
 *   R2    2^64 mod p
 *   g     a primitive 2048-th root of 1 modulo p (i.e. g^1024 = -1 mod p)
 *   ig    1/g mod p
 *   s     inverse mod p of the product of the previous primes
 * Values g, ig and s are in Montgomery representation. Thus:
 *   PRIMES[j].s = (2^32)/(\prod_{i<j} PRIMES[i].p) mod PRIMES[j].p
 * Value s helps in applying the CRT to convert big integers from RNS to
 * normal representation.
 *
 * Montgomery multiplication (mp_mmul()), given x and y, computes
 * (x*y)/R mod p; thus, the Montgomery multiplication of the Montgomery
 * representation of x and y yields the Montgomery representation of the
 * product x*y.
 *
 * The NTT representation of a polynomial modulo X^n+1 and modulo p is
 * the set of evaluations of that polynomial on the roots of X^n+1, which
 * are the odd powers of g.
 */

/* Given v in [-(p-1),+(p-1)], get v mod p in [0,p-1]. */
static inline uint32_t
mp_set(int32_t v, uint32_t p)
{
	return (uint32_t)v + (p & tbmask((uint32_t)v));
}

/* For x in [0,p-1], get its signed normalized value in [-p/2,+p/2]. */
static inline int32_t
mp_norm(uint32_t x, uint32_t p)
{
	x -= p & tbmask((p >> 1) - x);
	return *(int32_t *)&x;
}

/* Get R = 2^32 mod p. */
static inline uint32_t
mp_R(uint32_t p)
{
	/* Since 2*p < 2^32 < 3*p, we just return 2^32 - 2*p. */
	return -(p << 1);
}

/* Get hR = 2^31 mod p. */
static inline uint32_t
mp_hR(uint32_t p)
{
	/* Since p < 2^31 < (3/2)*p, we just return 2^31 - p. */
	return ((uint32_t)1 << 31) - p;
}

/* Addition modulo p. */
static inline uint32_t
mp_add(uint32_t a, uint32_t b, uint32_t p)
{
	uint32_t d = a + b - p;
	return d + (p & tbmask(d));
}

/* Subtraction modulo p. */
static inline uint32_t
mp_sub(uint32_t a, uint32_t b, uint32_t p)
{
	uint32_t d = a - b;
	return d + (p & tbmask(d));
}

/* Halving modulo p. */
static inline uint32_t
mp_half(uint32_t a, uint32_t p)
{
#if FNDSA_ASM_CORTEXM4
	uint32_t t;
	__asm__(
		"ubfx	%1, %0, #0, #1\n\t"
		"umlal	%0, %1, %1, %2\n\t"
		"lsr.w	%0, %0, #1"
		: "+r" (a), "=&r" (t)
		: "r" (p));
	return a;
#else
	return (a + (p & -(a & 1))) >> 1;
#endif
}

/* Montgomery multiplication: return (a*b)/R mod p.
   The function also works for out-of-range inputs provided that:
   a*b <= p + (p-1)*2^32  */
static inline uint32_t
mp_mmul(uint32_t a, uint32_t b, uint32_t p, uint32_t p0i)
{
#if FNDSA_ASM_CORTEXM4
	uint32_t d;
	__asm__(
		"umull	%0, %2, %0, %1\n\t"
		"mul	%1, %0, %4\n\t"
		"umlal	%0, %2, %1, %3\n\t"
		"sub.w	%2, %2, %3\n\t"
		"and	%0, %3, %2, asr #31\n\t"
		"add.w	%2, %2, %0"
		: "+r" (a), "+r" (b), "=&r" (d)
		: "r" (p), "r" (p0i));
	return d;
#else
	uint64_t z = (uint64_t)a * (uint64_t)b;
	uint32_t w = (uint32_t)z * p0i;
	uint32_t d = (uint32_t)((z + (uint64_t)w * (uint64_t)p) >> 32) - p;
	return d + (p & tbmask(d));
#endif
}

/* Divide x by y modulo p. If y is not invertible modulo p, then this
   function returns 0. Source values x and y MUST be in [0,p-1].
   Modulus p MUST be odd (it is not necessary that p is prime). */
#define mp_div   fndsa_mp_div
uint32_t mp_div(uint32_t x, uint32_t y, uint32_t p);

#if FNDSA_AVX2
#define mp_div_x8   fndsa_mp_div_x8
TARGET_AVX2 __m256i mp_div_x8(__m256i ynum, __m256i yden, __m256i yp);
#endif

/* Return 2^(31*e) mod p. This function assumes that e is not secret. */
static inline uint32_t
mp_Rx31(unsigned e, uint32_t p, uint32_t p0i, uint32_t R2)
{
	/* x <- 2^63 mod p = R*2^31 mod p */
	uint32_t x = mp_half(R2, p);
	uint32_t d = 1;
	for (;;) {
		if ((e & 1) != 0) {
			d = mp_mmul(d, x, p, p0i);
		}
		e >>= 1;
		if (e == 0) {
			return d;
		}
		x = mp_mmul(x, x, p, p0i);
	}
}

/* Compute the roots for NTT and inverse NTT; given g (primitive 2048-th
   root of 1 modulo p), this fills gm[] and igm[] with powers of g and 1/g:
      gm[rev(i)] = g^i mod p
      igm[rev(i)] = (1/2)*(1/g)^i mod p
   rev() is the bit-reversal function over 10 bits. Only the first n = 2^logn
   values of each array are filled.
   g and ig must be provided in Montgomery representation.
   Output values in gm[] and igm[] are in Montgomery representation. */
#define mp_mkgmigm   fndsa_mp_mkgmigm
void mp_mkgmigm(unsigned logn, uint32_t *restrict gm, uint32_t *restrict igm,
	uint32_t g, uint32_t ig, uint32_t p, uint32_t p0i);

/* Like mp_mkgmigm(), but computing only gm[]. */
#define mp_mkgm   fndsa_mp_mkgm
void mp_mkgm(unsigned logn, uint32_t *restrict gm,
	uint32_t g, uint32_t p, uint32_t p0i);

/* Like mp_mkgmigm(), but computing only igm[]. */
#define mp_mkigm   fndsa_mp_mkigm
void mp_mkigm(unsigned logn, uint32_t *restrict igm,
	uint32_t ig, uint32_t p, uint32_t p0i);

/* Compute the NTT over a polynomial. The polynomial a[] is modified
   in-place. */
#define mp_NTT   fndsa_mp_NTT
void mp_NTT(unsigned logn, uint32_t *restrict a, const uint32_t *restrict gm,
	uint32_t p, uint32_t p0i);

/* Compute the inverse NTT over a polynomial. The polynomial a[] is modified
   in-place. */
#define mp_iNTT   fndsa_mp_iNTT
void mp_iNTT(unsigned logn, uint32_t *restrict a, const uint32_t *restrict igm,
	uint32_t p, uint32_t p0i);

/*
 * Precomputed small primes. Table has 308 entries.
 */
typedef struct {
	uint32_t p;
	uint32_t p0i;
	uint32_t R2;
	uint32_t g;
	uint32_t ig;
	uint32_t s;
} small_prime;
#define PRIMES   fndsa_PRIMES
extern const small_prime PRIMES[];

#if FNDSA_AVX2
TARGET_AVX2
static inline __m256i
mp_set_x8(__m256i yv, __m256i yp)
{
	return _mm256_add_epi32(yv, _mm256_and_si256(yp,
		_mm256_srai_epi32(yv, 31)));
}

TARGET_AVX2
static inline __m256i
mp_norm_x8(__m256i yv, __m256i yp, __m256i yhp)
{
	return _mm256_sub_epi32(yv, _mm256_and_si256(yp,
		_mm256_cmpgt_epi32(yv, yhp)));
}

TARGET_AVX2
static inline __m256i
mp_add_x8(__m256i ya, __m256i yb, __m256i yp)
{
	__m256i yd = _mm256_sub_epi32(_mm256_add_epi32(ya, yb), yp);
	return _mm256_add_epi32(yd, _mm256_and_si256(yp,
		_mm256_srai_epi32(yd, 31)));
}

TARGET_AVX2
static inline __m256i
mp_sub_x8(__m256i ya, __m256i yb, __m256i yp)
{
	__m256i yd = _mm256_sub_epi32(ya, yb);
	return _mm256_add_epi32(yd, _mm256_and_si256(yp,
		_mm256_srai_epi32(yd, 31)));
}

TARGET_AVX2
static inline __m256i
mp_half_x8(__m256i ya, __m256i yp)
{
	return _mm256_srli_epi32(
		_mm256_add_epi32(ya, _mm256_and_si256(yp,
			_mm256_sub_epi32(_mm256_setzero_si256(),
			_mm256_and_si256(ya, _mm256_set1_epi32(1))))), 1);
}

/* Input:
      ya = a0 : XX : a1 : XX : a2 : XX : a3 : XX
      yb = b0 : XX : b1 : XX : b2 : XX : b3 : XX
   Output:
      mm(a0,b0) : 00 : mm(a1,b1) : 00 : mm(a2,b2) : 00 : mm(a3,b3) : 00  */
TARGET_AVX2
static inline __m256i
mp_mmul_x4(__m256i ya, __m256i yb, __m256i yp, __m256i yp0i)
{
	__m256i yd = _mm256_mul_epu32(ya, yb);
	__m256i ye = _mm256_mul_epu32(yd, yp0i);
	ye = _mm256_mul_epu32(ye, yp);
	yd = _mm256_srli_epi64(_mm256_add_epi64(yd, ye), 32);
	yd = _mm256_sub_epi32(yd, yp);
	return _mm256_add_epi32(yd, _mm256_and_si256(yp,
		_mm256_srai_epi32(yd, 31)));
}

TARGET_AVX2
static inline __m256i
mp_mmul_x8(__m256i ya, __m256i yb, __m256i yp, __m256i yp0i)
{
	/* yd0 <- a0*b0 : a2*b2 (+high lane) */
	__m256i yd0 = _mm256_mul_epu32(ya, yb);
	/* yd1 <- a1*b1 : a3*b3 (+high lane) */
	__m256i yd1 = _mm256_mul_epu32(
		_mm256_srli_epi64(ya, 32),
		_mm256_srli_epi64(yb, 32));

	__m256i ye0 = _mm256_mul_epu32(yd0, yp0i);
	__m256i ye1 = _mm256_mul_epu32(yd1, yp0i);
	ye0 = _mm256_mul_epu32(ye0, yp);
	ye1 = _mm256_mul_epu32(ye1, yp);
	yd0 = _mm256_add_epi64(yd0, ye0);
	yd1 = _mm256_add_epi64(yd1, ye1);

	/* yf0 <- lo(d0) : lo(d1) : hi(d0) : hi(d1) (+high lane) */
	__m256i yf0 = _mm256_unpacklo_epi32(yd0, yd1);
	/* yf1 <- lo(d2) : lo(d3) : hi(d2) : hi(d3) (+high lane) */
	__m256i yf1 = _mm256_unpackhi_epi32(yd0, yd1);
	/* yg <- hi(d0) : hi(d1) : hi(d2) : hi(d3) (+high lane) */
	__m256i yg = _mm256_unpackhi_epi64(yf0, yf1);
	/* Alternate version (instead of the three unpack above) but it
	   seems to be slightly slower.
	__m256i yg = _mm256_blend_epi32(_mm256_srli_epi64(yd0, 32), yd1, 0xAA);
	 */

	yg = _mm256_sub_epi32(yg, yp);
	return _mm256_add_epi32(yg, _mm256_and_si256(yp,
		_mm256_srai_epi32(yg, 31)));
}

#define avx2_mp_mkgmigm   fndsa_avx2_mp_mkgmigm
void avx2_mp_mkgmigm(unsigned logn,
	uint32_t *restrict gm, uint32_t *restrict igm,
	uint32_t g, uint32_t ig, uint32_t p, uint32_t p0i);
#define avx2_mp_mkgm   fndsa_avx2_mp_mkgm
void avx2_mp_mkgm(unsigned logn, uint32_t *restrict gm,
	uint32_t g, uint32_t p, uint32_t p0i);
#define avx2_mp_mkigm   fndsa_avx2_mp_mkigm
void avx2_mp_mkigm(unsigned logn, uint32_t *restrict igm,
	uint32_t ig, uint32_t p, uint32_t p0i);
#define avx2_mp_NTT   fndsa_avx2_mp_NTT
void avx2_mp_NTT(unsigned logn,
	uint32_t *restrict a, const uint32_t *restrict gm,
	uint32_t p, uint32_t p0i);
#define avx2_mp_iNTT   fndsa_avx2_mp_iNTT
void avx2_mp_iNTT(unsigned logn,
	uint32_t *restrict a, const uint32_t *restrict igm,
	uint32_t p, uint32_t p0i);
#endif

/* ==================================================================== */
/*
 * Custom bignum implementation.
 *
 * Big integers are represented as sequences of 32-bit integers; the
 * integer values are not necessarily consecutive in RAM (a dynamically
 * provided "stride" value is added to the current word pointer, to get
 * to the next word). The "len" parameter qualifies the number of words.
 *
 * Normal representation uses 31-bit limbs; each limb is stored in a
 * 32-bit word, with the top bit (31) always cleared. Limbs are in
 * low-to-high order. Signed integers use two's complement (hence, bit 30
 * of the last limb is the sign bit).
 *
 * RNS representation of a big integer x is the sequence of values
 * x modulo p, for the primes p defined in the PRIMES[] array.
 */

/* Multiply the provided big integer m with a small value x. The big
   integer must have stride 1. This function assumes that x < 2^31
   and that the big integer uses unsigned notation. The carry word is
   returned. */
#define zint_mul_small   fndsa_zint_mul_small
uint32_t zint_mul_small(uint32_t *m, size_t len, uint32_t x);

/* Reduce a big integer d modulo a small integer p.
   Rules:
     d is unsigned
     p is prime
     2^30 < p < 2^31
     p0i = -1/p mod 2^32
     R2 = 2^64 mod p     */
#define zint_mod_small_unsigned   fndsa_zint_mod_small_unsigned
uint32_t zint_mod_small_unsigned(const uint32_t *d, size_t len, size_t stride,
	uint32_t p, uint32_t p0i, uint32_t R2);

/* Like zint_mod_small_unsigned() except that d uses signed convention.
   Extra parameter is Rx = 2^(31*len) mod p. */
static inline uint32_t
zint_mod_small_signed(const uint32_t *d, size_t len, size_t stride,
        uint32_t p, uint32_t p0i, uint32_t R2, uint32_t Rx)
{
	if (len == 0) {
		return 0;
	}
	uint32_t z = zint_mod_small_unsigned(d, len, stride, p, p0i, R2);
	z = mp_sub(z, Rx & -(d[(len - 1) * stride] >> 30), p);
	return z;
}

/* Add s*a to d. d and a initially have length len words; the new d
   has length len+1 words. Small integer s must fit on 31 bits. d and a
   must not overlap. d uses dstride, while a has stride 1. */
#define zint_add_mul_small   fndsa_zint_add_mul_small
void zint_add_mul_small(uint32_t *restrict d, size_t len, size_t dstride,
	const uint32_t *restrict a, uint32_t s);

/* Normalize a modular integer aorund 0: if x > m/2, then x is replaced
   with x - m. Input x uses unsigned convention, output is signed. The
   two integers x and m have the same length (len words). x uses xstride,
   while m has stride 1. */
#define zint_norm_zero   fndsa_zint_norm_zero
void zint_norm_zero(uint32_t *restrict x, size_t len, size_t xstride,
	const uint32_t *restrict m);

/* Rebuild integers from their RNS representation. There are num_sets
   sets of n integers; within each set, the n integers are interleaved,
   so that words of a given integer occur every n slots in RAM (i.e.
   each integer has stride n, and the integers of a set start on
   consecutive words). Each integer has length xlen words. The sets are
   consecutive in RAM.

   If normalized_signed is non-zero then the output values are normalized
   in [-m/2,m/2] (with m being the product of all involved small prime
   moduli) and returned in signed conventions; otherwise, output values
   are in [0,m-1] and use unsigned convention.

   tmp[] must have room for xlen words. */
#define zint_rebuild_CRT   fndsa_zint_rebuild_CRT
void zint_rebuild_CRT(uint32_t *restrict xx, size_t xlen, size_t n,
	size_t num_sets, int normalize_signed, uint32_t *restrict tmp);

/* Negate a big integer conditionally: a is replaced with -a if and only
   if ctl = 0xFFFFFFFF. Control value ctl must be 0x00000000 or 0xFFFFFFFF.
   The integer has stride 1. */
#define zint_negate   fndsa_zint_negate
void zint_negate(uint32_t *a, size_t len, uint32_t ctl);

/* Compute GCD(x,y). x and y must be both odd. If the GCD is not 1, then
   this function returns 0 (failure). If the GCD is 1, then the function
   returns 1, and u and v are set to integers such that:
      0 <= u <= y
      0 <= v <= x
      x*u - y*v = 1
   x and y are unmodified. Both input value must have the same encoded
   length (len), and have stride 1. u and v also have size len each.
   tmp must have room for 4*len words. The x, y, u, v and tmp must not
   overlap in any way (x and y may overlap, but this is useless). */
#define zint_bezout   fndsa_zint_bezout
int zint_bezout(uint32_t *restrict u, uint32_t *restrict v,
	const uint32_t *restrict x, const uint32_t *restrict y,
	size_t len, uint32_t *restrict tmp);

/* Add k*(2^sc)*y to x. The result is assumed to fit in the array of
   size xlen (truncation is applied if necessary). Scale factor sc
   is provided as sch and scl such that sc = 31*sch + scl and scl
   is in [0,30].
   xlen MUST NOT be lower than ylen; however, it is allowed that xlen is
   greater than ylen.
   x and y both use signed convention and have the same stride. */
#define zint_add_scaled_mul_small   fndsa_zint_add_scaled_mul_small
void zint_add_scaled_mul_small(uint32_t *restrict x, size_t xlen,
	const uint32_t *restrict y, size_t ylen, size_t stride,
	int32_t k, uint32_t sch, uint32_t scl);

/* Subtract y*2^sc from x. This is a specialized version of
   zint_add_scaled_mul_small(), with multiplier k = -1. */
#define zint_sub_scaled   fndsa_zint_sub_scaled
void zint_sub_scaled(uint32_t *restrict x, size_t xlen,
	const uint32_t *restrict y, size_t ylen, size_t stride,
	uint32_t sch, uint32_t scl);

#if FNDSA_AVX2
#define avx2_zint_mod_small_unsigned_x8   fndsa_avx2_zint_mod_small_unsigned_x8
TARGET_AVX2 __m256i avx2_zint_mod_small_unsigned_x8(
	const uint32_t *d, size_t len, size_t stride,
	__m256i yp, __m256i yp0i, __m256i yR2);
#define avx2_zint_add_mul_small_x8   fndsa_avx2_zint_add_mul_small_x8
TARGET_AVX2 void avx2_zint_add_mul_small_x8(
	uint32_t *restrict d, size_t len, size_t dstride,
	const uint32_t *restrict a, __m256i ys);
#define avx2_zint_rebuild_CRT   fndsa_avx2_zint_rebuild_CRT
void avx2_zint_rebuild_CRT(uint32_t *restrict xx, size_t xlen, size_t n,
	size_t num_sets, int normalize_signed, uint32_t *restrict tmp);

TARGET_AVX2
static inline __m256i
avx2_zint_mod_small_signed_x8(const uint32_t *d, size_t len, size_t stride,
        __m256i yp, __m256i yp0i, __m256i yR2, __m256i yRx)
{
	if (len == 0) {
		return _mm256_setzero_si256();
	}
	__m256i yz = avx2_zint_mod_small_unsigned_x8(
		d, len, stride, yp, yp0i, yR2);
	__m256i yl = _mm256_loadu_si256((__m256i *)(d + (len - 1) * stride));
	__m256i ym = _mm256_sub_epi32(_mm256_setzero_si256(),
		_mm256_srli_epi32(yl, 30));
	yz = mp_sub_x8(yz, _mm256_and_si256(yRx, ym), yp);
	return yz;
}
#endif

/* ==================================================================== */
/*
 * Fixed-point numbers.
 *
 * For FFT and other computations with approximations, we use a fixed-point
 * format over 64 bits; the top 32 bits are the integral part, and the low
 * 32 bits are the fractional part.
 */

/* We wrap the fxr type into a struct so that any attempt at using
   arithmetic operators directly is detected. */
typedef struct {
	uint64_t v;
} fxr;

/* Macro for defining constants of type fxr. */
#define FXR(x)   { (x) }

/* Convert an integer to fxr. */
static inline fxr fxr_of(int32_t j) { return (fxr) { (uint64_t)j << 32 }; }

/* Given integer t, return t/2^32 as fxr. */
static inline fxr fxr_of_scaled32(uint64_t t) { return (fxr) { t }; }

/* Arithmetic operations on fxr. */
static inline fxr fxr_add(fxr x, fxr y) { return (fxr) { x.v + y.v }; }
static inline fxr fxr_sub(fxr x, fxr y) { return (fxr) { x.v - y.v }; }
static inline fxr fxr_double(fxr x) { return (fxr) { x.v << 1 }; }
static inline fxr fxr_neg(fxr x) { return (fxr) { -x.v }; }

static inline fxr
fxr_abs(fxr x)
{
	x.v -= (x.v << 1) & (uint64_t)(*(int64_t *)&x.v >> 63);
	return x;
}

static inline fxr
fxr_mul(fxr x, fxr y)
{
#if FNDSA_ASM_CORTEXM4
	uint32_t x0 = (uint32_t)x.v;
	uint32_t x1 = (uint32_t)(x.v >> 32);
	uint32_t y0 = (uint32_t)y.v;
	uint32_t y1 = (uint32_t)(y.v >> 32);
	uint32_t z0, z1, tt;
	__asm__(
		"and	%0, %4, %5, asr #31\n\t"
		"and	%1, %6, %3, asr #31\n\t"
		"umaal	%1, %0, %4, %6\n\t"
		"umull	%2, %0, %3, %5\n\t"
		"smlal	%0, %1, %3, %6\n\t"
		"smlal	%0, %1, %4, %5"
		: "=&r" (z0), "=&r" (z1), "=&r" (tt)
		: "r" (x0), "r" (x1), "r" (y0), "r" (y1));
	return (fxr) { (uint64_t)z0 | ((uint64_t)z1 << 32) };
#elif (defined __GNUC__ || defined __clang__) && defined __SIZEOF_INT128__
	/* If __int128 is supported then the underlying platform is
	   assumed to be 64-bit and have native support for the 64x64->128
	   multiplication. Note that there are some existing 64-bit CPUs
	   where that multiplication is not constant-time (e.g. ARM
	   Cortex A53 and A55). */
	__int128 z = (__int128)*(int64_t *)&x.v * (__int128)*(int64_t *)&y.v;
	return (fxr) { (uint64_t)(z >> 32) };
#else
	uint32_t xl = (uint32_t)x.v;
	uint32_t yl = (uint32_t)y.v;
	int32_t xh = (int32_t)(*(int64_t *)&x.v >> 32);
	int32_t yh = (int32_t)(*(int64_t *)&y.v >> 32);
	uint64_t z0 = ((uint64_t)xl * (uint64_t)yl) >> 32;
	uint64_t z1 = (uint64_t)xl * (uint64_t)yh;
	uint64_t z2 = (uint64_t)xh * (uint64_t)yl;
	uint64_t z3 = (uint64_t)((int64_t)xh * (int64_t)yh) << 32;
	return (fxr) { z0 + z1 + z2 + z3 };
#endif
}

/* Specialized squaring function. */
static inline fxr
fxr_sqr(fxr x)
{
#if FNDSA_ASM_CORTEXM4
	uint32_t x0 = (uint32_t)x.v;
	uint32_t x1 = (uint32_t)(x.v >> 32);
	uint32_t z0, z1, tt;
	__asm__(
		"and	%0, %4, %3, asr #31\n\t"
		"mov.w	%1, %0\n\t"
		"umaal	%1, %0, %4, %4\n\t"
		"umull	%2, %0, %3, %3\n\t"
		"smlal	%0, %1, %3, %4\n\t"
		"smlal	%0, %1, %4, %3"
		: "=&r" (z0), "=&r" (z1), "=&r" (tt)
		: "r" (x0), "r" (x1));
	return (fxr) { (uint64_t)z0 | ((uint64_t)z1 << 32) };
#elif (defined __GNUC__ || defined __clang__) && defined __SIZEOF_INT128__
	/* If __int128 is supported then the underlying platform is
	   assumed to be 64-bit and have native support for the 64x64->128
	   multiplication. Note that there are some existing 64-bit CPUs
	   where that multiplication is not constant-time (e.g. ARM
	   Cortex A53 and A55). */
	int64_t t = *(int64_t *)&x.v;
	__int128 z = (__int128)t * (__int128)t;
	return (fxr) { (uint64_t)(z >> 32) };
#else
	uint32_t xl = (uint32_t)x.v;
	int32_t xh = (int32_t)(*(int64_t *)&x.v >> 32);
	uint64_t z0 = ((uint64_t)xl * (uint64_t)xl) >> 32;
	uint64_t z1 = (uint64_t)xl * (uint64_t)xh;
	uint64_t z3 = (uint64_t)((int64_t)xh * (int64_t)xh) << 32;
	return (fxr) { z0 + (z1 << 1) + z3 };
#endif
}

static inline int32_t
fxr_round(fxr x)
{
	x.v += 0x80000000ul;
	return (int32_t)(*(int64_t *)&x.v >> 32);
}

/* Divide x by 2^e. */
static inline fxr
fxr_div2e(fxr x, unsigned e)
{
	x.v += ((uint64_t)1 << e) >> 1;
	return (fxr) { (uint64_t)(*(int64_t *)&x.v >> e) };
}

/* Multiply x by 2^e. */
static inline fxr
fxr_mul2e(fxr x, unsigned e)
{
	return (fxr) { x.v << e };
}

/* Division on fxr values. */
#define inner_fxr_div   fndsa_inner_fxr_div
uint64_t inner_fxr_div(uint64_t x, uint64_t y);
static inline fxr
fxr_inv(fxr x)
{
	return (fxr) { inner_fxr_div((uint64_t)1 << 32, x.v) };
}
static inline fxr
fxr_div(fxr x, fxr y)
{
	return (fxr) { inner_fxr_div(x.v, y.v) };
}

/* Comparison (lower-than). */
static inline int
fxr_lt(fxr x, fxr y)
{
	return *(int64_t *)&x.v < *(int64_t *)&y.v;
}

static const fxr fxr_zero = FXR(0);
static const fxr fxr_sqrt2 = FXR(6074001000ull);

/* Complex values use two fxr (real and imaginary parts). */
typedef struct {
	fxr re, im;
} fxc;

#define FXC(re, im)   { FXR(re), FXR(im) }

static inline fxc
fxc_add(fxc x, fxc y)
{
	return (fxc) { fxr_add(x.re, y.re), fxr_add(x.im, y.im) };
}

static inline fxc
fxc_sub(fxc x, fxc y)
{
	return (fxc) { fxr_sub(x.re, y.re), fxr_sub(x.im, y.im) };
}

static inline fxc
fxc_half(fxc x)
{
	return (fxc) { fxr_div2e(x.re, 1), fxr_div2e(x.im, 1) };
}

static inline fxc
fxc_mul(fxc x, fxc y)
{
	/* We are computing r = (a + i*b)*(c + i*d) with:
	     z0 = a*c
	     z1 = b*d
	     z2 = (a + b)*(c + d)
	     r = (z0 - z1) + i*(z2 - (z0 + z1))
	   Due to the approximate nature of fxr, this might not yield
	   exactly the same imaginary value as a*d + b*c would. */
	fxr z0 = fxr_mul(x.re, y.re);
	fxr z1 = fxr_mul(x.im, y.im);
	fxr z2 = fxr_mul(fxr_add(x.re, x.im), fxr_add(y.re, y.im));
	return (fxc) { fxr_sub(z0, z1), fxr_sub(z2, fxr_add(z0, z1)) };
}

static inline fxc
fxc_conj(fxc x)
{
	return (fxc) { x.re, fxr_neg(x.im) };
}

/*
 * Polynomials modulo X^n+1 with real coefficients are represented as
 * arrays of n elements of type fxr.
 *
 * The FFT representation of a polynomial f is the set of f(zeta^(2*i+1))
 * for zeta a root of X^n+1. Since f is a real polynomial, we only need
 * only n/2 complex values:
 *   conj(f(zeta^(2*i+1))) = f(conj(zeta)^(2*i+1))
 *                         = f(zeta^(2*n-(2*i+1)))
 * In this implementation, we split the real and imaginary parts of each
 * of the n/2 complex values: f[i] contains the real part, and f[i+n/2]
 * contains the corresponding imaginary part.
 */

/* Convert a (real) vector to its FFT representation. */
#define vect_FFT   fndsa_vect_FFT
void vect_FFT(unsigned logn, fxr *f);

/* Convert back from FFT representation into a real vector. */
#define vect_iFFT   fndsa_vect_iFFT
void vect_iFFT(unsigned logn, fxr *f);

/* Set a vector d to the value of the small polynomial f. */
#define vect_set   fndsa_vect_set
void vect_set(unsigned logn, fxr *d, const int8_t *f);

/* Add vector b to vector a. This works in both real and FFT representations.
   Vectors a and b MUST NOT overlap. */
#define vect_add   fndsa_vect_add
void vect_add(unsigned logn, fxr *restrict a, const fxr *restrict b);

/* Multiply vector a by the real constant c. This works in both real
   and FFT representations. */
#define vect_mul_realconst   fndsa_vect_mul_realconst
void vect_mul_realconst(unsigned logn, fxr *a, fxr c);

/* Multiply a vector by 2^e. This works in both real and FFT representations. */
#define vect_mul2e   fndsa_vect_mul2e
void vect_mul2e(unsigned logn, fxr *a, unsigned e);

/* Multiply vector a by vector b. The vectors must be in FFT representation.
   Vectors a and b MUST NOT overlap. */
#define vect_mul_fft   fndsa_vect_mul_fft
void vect_mul_fft(unsigned logn, fxr *restrict a, const fxr *restrict b);

/* Convert a vector into its adjoint (in FFT representation). */
#define vect_adj_fft   fndsa_vect_adj_fft
void vect_adj_fft(unsigned logn, fxr *a);

/* Multiply vector a by self-adjoint vector b. The vectors must be in FFT
   representation. Since the FFT representation of a self-adjoint vector
   contains only real numbers, the second half of b contains only zeros and
   is not accessed by this function. Vectors a and b MUST NOT overlap. */
#define vect_mul_selfadj_fft   fndsa_vect_mul_selfadj_fft
void vect_mul_selfadj_fft(unsigned logn,
	fxr *restrict a, const fxr *restrict b);

/* Divide vector a by self-adjoint vector b. The vectors must be in FFT
   representation. Since the FFT representation of an auto-adjoint vector
   contains only real numbers, the second half of b contains only zeros and
   is not accessed by this function. Vectors a and b MUST NOT overlap. */
#define vect_div_selfadj_fft   fndsa_vect_div_selfadj_fft
void vect_div_selfadj_fft(unsigned logn,
	fxr *restrict a, const fxr *restrict b);

/* Compute d = a*adj(a) + b*adj(b). Polynomials are in FFT representation.
   Since d is self-adjoint, only its first half is set; the second half
   is _implicitly_ zero (this function does not access the second half of d).
   Vectors a, b and d MUST NOT overlap. */
#define vect_norm_fft   fndsa_vect_norm_fft
void vect_norm_fft(unsigned logn, fxr *restrict d,
	const fxr *restrict a, const fxr *restrict b);

/* Compute d = (2^e)/(a*adj(a) + b*adj(b)). Polynomials are in FFT
   representation. Since d is self-adjoint, only its first half is set; the
   second half is _implicitly_ zero (this function does not access the
   second half of d). Vectors a, b and d MUST NOT overlap. */
#define vect_invnorm_fft   fndsa_vect_invnorm_fft
void vect_invnorm_fft(unsigned logn, fxr *restrict d,
	const fxr *restrict a, const fxr *restrict b, unsigned e);

/* Compute d = (2^e)*adj(a)/(a*adj(a)), written back into a[]. Polynomials
   are in FFT representation. */
#define vect_inv_mul2e_fft   fndsa_vect_inv_mul2e_fft
void vect_inv_mul2e_fft(unsigned logn, fxr *a, unsigned e);

#if FNDSA_AVX2
TARGET_AVX2
static inline __m256i
fxr_mul_x4(__m256i ya, __m256i yb)
{
	__m256i ya_hi = _mm256_srli_epi64(ya, 32);
	__m256i yb_hi = _mm256_srli_epi64(yb, 32);
	__m256i y1 = _mm256_mul_epu32(ya, yb);
	__m256i y2 = _mm256_mul_epu32(ya, yb_hi);
	__m256i y3 = _mm256_mul_epu32(ya_hi, yb);
	__m256i y4 = _mm256_mul_epu32(ya_hi, yb_hi);
	y1 = _mm256_srli_epi64(y1, 32);
	y4 = _mm256_slli_epi64(y4, 32);
	__m256i y5 = _mm256_add_epi64(
		_mm256_add_epi64(y1, y2),
		_mm256_add_epi64(y3, y4));
	__m256i yna = _mm256_srai_epi32(ya, 31);
	__m256i ynb = _mm256_srai_epi32(yb, 31);
	return _mm256_sub_epi64(y5,
		_mm256_add_epi64(
			_mm256_and_si256(_mm256_slli_epi64(yb, 32), yna),
			_mm256_and_si256(_mm256_slli_epi64(ya, 32), ynb)));
}

TARGET_AVX2
static inline __m256i
fxr_sqr_x4(__m256i ya)
{
	__m256i ya_hi = _mm256_srli_epi64(ya, 32);
	__m256i y1 = _mm256_mul_epu32(ya, ya);
	__m256i y2 = _mm256_mul_epu32(ya, ya_hi);
	__m256i y3 = _mm256_mul_epu32(ya_hi, ya_hi);
	y1 = _mm256_srli_epi64(y1, 32);
	y2 = _mm256_add_epi64(y2, y2);
	y3 = _mm256_slli_epi64(y3, 32);
	__m256i y4 = _mm256_add_epi64(_mm256_add_epi64(y1, y2), y3);
	return _mm256_sub_epi64(y4,
		_mm256_and_si256(_mm256_slli_epi64(ya, 33),
		_mm256_srai_epi32(ya, 31)));
}

TARGET_AVX2
static inline __m256i
fxr_half_x4(__m256i ya)
{
	const __m256i y1 = _mm256_set1_epi64x(1);
	const __m256i yh = _mm256_set1_epi64x((uint64_t)1 << 63);
	ya = _mm256_add_epi64(ya, y1);
	return _mm256_or_si256(
		_mm256_srli_epi64(ya, 1),
		_mm256_and_si256(ya, yh));
}

#define avx2_fxr_div_x4   fndsa_avx2_fxr_div_x4
TARGET_AVX2 __m256i avx2_fxr_div_x4(__m256i yn, __m256i yd);

TARGET_AVX2
static inline void
fxr_div_x4_1(fxr *n0, fxr *n1, fxr *n2, fxr *n3, fxr d)
{
	__m256i yn = _mm256_setr_epi64x(n0->v, n1->v, n2->v, n3->v);
	__m256i yd = _mm256_set1_epi64x(d.v);
	union {
		__m256i y;
		uint64_t q[4];
	} z;
	z.y = avx2_fxr_div_x4(yn, yd);
	n0->v = z.q[0];
	n1->v = z.q[1];
	n2->v = z.q[2];
	n3->v = z.q[3];
}

TARGET_AVX2
static inline void
fxc_mul_x4(__m256i *yd_re, __m256i *yd_im,
        __m256i ya_re, __m256i ya_im, __m256i yb_re, __m256i yb_im)
{
	__m256i y0 = fxr_mul_x4(ya_re, yb_re);
	__m256i y1 = fxr_mul_x4(ya_im, yb_im);
	__m256i y2 = fxr_mul_x4(
		_mm256_add_epi64(ya_re, ya_im),
		_mm256_add_epi64(yb_re, yb_im));
	*yd_re = _mm256_sub_epi64(y0, y1);
	*yd_im = _mm256_sub_epi64(y2, _mm256_add_epi64(y0, y1));
}

#define avx2_vect_FFT   fndsa_avx2_vect_FFT
void avx2_vect_FFT(unsigned logn, fxr *f);
#define avx2_vect_iFFT   fndsa_avx2_vect_iFFT
void avx2_vect_iFFT(unsigned logn, fxr *f);
#define avx2_vect_set   fndsa_avx2_vect_set
void avx2_vect_set(unsigned logn, fxr *d, const int8_t *f);
#define avx2_vect_add   fndsa_avx2_vect_add
void avx2_vect_add(unsigned logn, fxr *restrict a, const fxr *restrict b);
#define avx2_vect_mul_realconst   fndsa_avx2_vect_mul_realconst
void avx2_vect_mul_realconst(unsigned logn, fxr *a, fxr c);
#define avx2_vect_mul2e   fndsa_avx2_vect_mul2e
void avx2_vect_mul2e(unsigned logn, fxr *a, unsigned e);
#define avx2_vect_mul_fft   fndsa_avx2_vect_mul_fft
void avx2_vect_mul_fft(unsigned logn, fxr *restrict a, const fxr *restrict b);
#define avx2_vect_adj_fft   fndsa_avx2_vect_adj_fft
void avx2_vect_adj_fft(unsigned logn, fxr *a);
#define avx2_vect_mul_selfadj_fft   fndsa_avx2_vect_mul_selfadj_fft
void avx2_vect_mul_selfadj_fft(unsigned logn,
	fxr *restrict a, const fxr *restrict b);
#define avx2_vect_div_selfadj_fft   fndsa_avx2_vect_div_selfadj_fft
void avx2_vect_div_selfadj_fft(unsigned logn,
	fxr *restrict a, const fxr *restrict b);
#define avx2_vect_norm_fft   fndsa_avx2_vect_norm_fft
void avx2_vect_norm_fft(unsigned logn, fxr *restrict d,
	const fxr *restrict a, const fxr *restrict b);
#define avx2_vect_invnorm_fft   fndsa_avx2_vect_invnorm_fft
void avx2_vect_invnorm_fft(unsigned logn, fxr *restrict d,
	const fxr *restrict a, const fxr *restrict b, unsigned e);
#define avx2_vect_inv_mul2e_fft   fndsa_avx2_vect_inv_mul2e_fft
void avx2_vect_inv_mul2e_fft(unsigned logn, fxr *a, unsigned e);
#endif

/* ==================================================================== */
/*
 * Polynomials with integer coefficients.
 *
 * Polynomials use an interleaved in-memory representation:
 *
 *   There are n = 2^logn coefficients (degrees 0 to n-1).
 *   Each coefficient has len words and has stride n.
 *   The n coefficients starts on consecutive words.
 *
 * A polynomial may use plain representation (integers may use signed or
 * unsigned convention) or RNS (the words of an integer are really the
 * integer value reduced modulo the first len primes in PRIMES).
 * Moreover, when using RNS, each row (i.e. n consecutive words in RAM
 * that encode the polynomial modulo PRIMES[i].p) may be in NTT
 * representation.
 */

/* Load a one-byte polynomial with reduction modulo p. */
#define poly_mp_set_small   fndsa_poly_mp_set_small
void poly_mp_set_small(unsigned logn, uint32_t *restrict d,
	const int8_t *restrict f, uint32_t p);

/* Convert a polynomial in one-word normal representation (signed) into RNS
   modulo the single prime p. */
#define poly_mp_set   fndsa_poly_mp_set
void poly_mp_set(unsigned logn, uint32_t *f, uint32_t p);

/* Convert a polynomial in RNS (modulo a single prime p) into one-word
   normal representation (signed). */
#define poly_mp_norm   fndsa_poly_mp_norm
void poly_mp_norm(unsigned logn, uint32_t *f, uint32_t p);

/* Convert a polynomial to small integers. Source values are supposed
   to be normalized (signed). Returned value is 0 if any of the
   coefficients exceeds the provided limit (in absolute value); on
   success, 1 is returned.
  
   In case of failure, the function returns earlier; this does not
   break constant-time discipline as long as a failure implies that the
   (f,g) polynomials are discarded. */
#define poly_big_to_small   fndsa_poly_big_to_small
int poly_big_to_small(unsigned logn, int8_t *restrict d,
	const uint32_t *restrict s, int lim);

/* Get the maximum bit length of all coefficients of a polynomial. Each
   coefficient has size flen words.
  
   The bit length of a big integer is defined to be the length of the
   minimal binary representation, using two's complement for negative
   values, and excluding the sign bit. This definition implies that
   if x = 2^k, then x has bit length k but -x has bit length k-1. For
   non powers of two, x and -x have the same bit length.
  
   This function is constant-time with regard to coefficient values and
   the returned bit length. */
#define poly_max_bitlength   fndsa_poly_max_bitlength
uint32_t poly_max_bitlength(unsigned logn, const uint32_t *f, size_t flen);

/* Compute q = x / 31 and r = x % 31 for an unsigned integer x. This
   macro is constant-time and works for values x up to 63487 (inclusive). */
#define DIVREM31(q, r, x)  { \
	        uint32_t divrem31_q, divrem31_x; \
	        divrem31_x = (x); \
	        divrem31_q = (uint32_t)(divrem31_x * (uint32_t)67651) >> 21; \
	        (q) = divrem31_q; \
	        (r) = divrem31_x - 31 * divrem31_q; \
	} while (0)

/* Convert a polynomial to fixed-point approximations, with scaling.
   For each coefficient x, the computed approximation is x/2^sc.
   This function assumes that |x| < 2^(30+sc). The length of each
   coefficient must be less than 2^24 words.
  
   This function is constant-time with regard to the coefficient values
   and to the scaling factor. */
#define poly_big_to_fixed   fndsa_poly_big_to_fixed
void poly_big_to_fixed(unsigned logn, fxr *restrict d,
	const uint32_t *restrict f, size_t len, uint32_t sc);

/* Subtract k*f from F, where F, f and k are polynomials modulo X^n+1.
   Coefficients of polynomial k are small integers (signed values in the
   -2^31..+2^31 range) scaled by 2^sc.
  
   This function implements the basic quadratic multiplication algorithm,
   which is efficient in space (no extra buffer needed) but slow at
   high degree. */
#define poly_sub_scaled   fndsa_poly_sub_scaled
void poly_sub_scaled(unsigned logn,
	uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc);

/* Subtract k*f from F. Coefficients of polynomial k are small integers
   (signed values in the -2^31..+2^31 range) scaled by 2^sc. Polynomial f
   MUST be in RNS+NTT over flen+1 words (even though f itself would fit on
   flen words); polynomial F MUST be in plain representation. */
#define poly_sub_scaled_ntt   fndsa_poly_sub_scaled_ntt
void
poly_sub_scaled_ntt(unsigned logn, uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc, uint32_t *restrict tmp);

/* depth = 1
   logn = logn_top - depth
   Inputs:
      F       polynomial of degree 2^logn, plain integer representation (FGlen)
      FGlen   size of each coefficient of F (must be 1 or 2)
      f       polynomial of degree 2^logn_top, small coefficients
      k       polynomial of degree 2^logn (plain, 32-bit)
      sc      scaling logarithm (public value)
      tmp     temporary with room at least max(FGlen, 2^logn_top) words
   Operation:
      F <- F - (2^sc)*k*ft
   with ft being the degree-n polynomial corresponding to f.
   It is assumed that the result fits.
  
   WARNING: polynomial k is consumed in the process.
  
   This function uses 3*n words in tmp[]. */
#define poly_sub_kf_scaled_depth1   fndsa_poly_sub_kf_scaled_depth1
void poly_sub_kf_scaled_depth1(unsigned logn_top,
	uint32_t *restrict F, size_t FGlen,
	uint32_t *restrict k, uint32_t sc,
	const int8_t *restrict f,
	uint32_t *restrict tmp);

/* Compute the squared norm of a small polynomial. */
#define poly_sqnorm   fndsa_poly_sqnorm
uint32_t poly_sqnorm(unsigned logn, const int8_t *f);

#if FNDSA_AVX2
#define avx2_poly_mp_set_small   fndsa_avx2_poly_mp_set_small
void avx2_poly_mp_set_small(unsigned logn, uint32_t *restrict d,
	const int8_t *restrict f, uint32_t p);
#define avx2_poly_mp_set   fndsa_avx2_poly_mp_set
void avx2_poly_mp_set(unsigned logn, uint32_t *f, uint32_t p);
#define avx2_poly_mp_norm   fndsa_avx2_poly_mp_norm
void avx2_poly_mp_norm(unsigned logn, uint32_t *f, uint32_t p);
#define avx2_poly_sub_scaled   fndsa_avx2_poly_sub_scaled
void avx2_poly_sub_scaled(unsigned logn,
	uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc);
#define avx2_poly_sub_scaled_ntt   fndsa_avx2_poly_sub_scaled_ntt
void
avx2_poly_sub_scaled_ntt(unsigned logn, uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc, uint32_t *restrict tmp);
#define avx2_poly_sub_kf_scaled_depth1   fndsa_avx2_poly_sub_kf_scaled_depth1
void avx2_poly_sub_kf_scaled_depth1(unsigned logn_top,
	uint32_t *restrict F, size_t FGlen,
	uint32_t *restrict k, uint32_t sc,
	const int8_t *restrict f,
	uint32_t *restrict tmp);
#define avx2_poly_sqnorm   fndsa_avx2_poly_sqnorm
uint32_t avx2_poly_sqnorm(unsigned logn, const int8_t *f);
#endif

/* ==================================================================== */
/*
 * NTRU equation solving.
 *
 * Given small polynomials f and g, solving the NTRU equation means finding
 * small polynomials F and G such that f*G - g*F = q (modulo X^n+1).
 *
 * The implementation requires that f and g have odd parity. It may find
 * a solution only if the resultants of f and g, respectively, with X^n+1
 * are prime to each other. Even when a solution mathematically exists, the
 * implementation may fail to find it. In general, when a solution exists,
 * there are several, which are not trivially derived from each other; it
 * is unspecified which solution is returned (however, this code is
 * deterministic and will always return the same solution for a given (f,g)
 * input).
 */

/* Solve the NTRU equation for the provided (f,g). The (F,G) solution,
   if found, is returned at the start of the tmp[] array, as two
   consecutive int8_t[] values. Returned value is 1 on success, 0 on error.

   tmp[] must have room for 6*n words. */
#define solve_NTRU   fndsa_solve_NTRU
int solve_NTRU(unsigned logn,
        const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp);

/* Check that a given (f,g) has an acceptable orthogonolized norm.
   tmp[] must have room for 2.5*n fxr values */
#define check_ortho_norm   fndsa_check_ortho_norm
int check_ortho_norm(unsigned logn,
	const int8_t *f, const int8_t *g, fxr *tmp);

#if FNDSA_AVX2
#define avx2_solve_NTRU   fndsa_avx2_solve_NTRU
int avx2_solve_NTRU(unsigned logn,
        const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp);
#define avx2_check_ortho_norm   fndsa_avx2_check_ortho_norm
int avx2_check_ortho_norm(unsigned logn,
	const int8_t *f, const int8_t *g, fxr *tmp);
#endif

/* ==================================================================== */
/*
 * (f,g) sampling (Gaussian distribution).
 */

/* Sample f (or g) from the provided SHAKE-based PRNG. This function
   ensures that the sampled polynomial has odd parity. */
#define sample_f   fndsa_sample_f
void sample_f(unsigned logn, shake_context *pc, int8_t *f);

/* ==================================================================== */

#endif
