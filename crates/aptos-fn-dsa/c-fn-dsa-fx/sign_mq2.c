/*
 * Computations modulo q' = 18433.
 * This is used to support private key conversion from official FN-DSA
 * to the alternate format.
 */

#include "sign_inner.h"

/* Modulus q' = 18433 */
#define Q2         18433

#if Q2 != MQ2_MOD
#error Mismatch on secondary modulus
#endif

/* -1/q' mod 2^32 */
#define Q1I   3955247103

/* 2^64 mod q' */
#define R2           806

/* The other modulus (q) */
#define Q          12289

/* 1/q mod q' (Montgomery representation) */
#define Q2_IQ_MR   13692

/* Q1I split into low and high parts (16 bits each), converted to
   signed 16-bit representation. */
#define Q1Ilo      18431
#define Q1Ihi      -5184

#if FNDSA_ASM_CORTEXM4
/*
 * On the ARM Cortex-M4, we use a relaxed internal representation with
 * values in [0,q']; this makes the ext-to-int operation trivial (internal
 * representation is a superset of external representation) and allows
 * some speed-ups in the NTT.
 */

static inline uint32_t
mq2_add(uint32_t x, uint32_t y)
{
	x += y - Q2;
	x += Q2 & (x >> 16);
	return x;
}

static inline uint32_t
mq2_sub(uint32_t x, uint32_t y)
{
	x -= y;
	x += Q2 & (x >> 16);
	return x;
}

static inline uint32_t
mq2_half(uint32_t x)
{
	x += Q2 & -(x & 1);
	return x >> 1;
}

static inline uint32_t
mq2_mred(uint32_t x)
{
	uint32_t t;
	__asm__("mul %1, %0, %3\n\tumaal %3, %0, %1, %2"
		: "+r" (x), "=&r" (t)
		: "r" (Q2), "r" (Q1I));
	(void)t;
	return x;
}

#else

static inline uint32_t
mq2_add(uint32_t x, uint32_t y)
{
	/* -(x+y) in [-q',+q'-2] */
	x = Q2 - (x + y);
	/* normalize to [0,+q'-1] */
	x += Q2 & (x >> 16);
	/* negate to get x+y in [1,q'] */
	return Q2 - x;
}

#if FNDSA_SSE2
TARGET_SSE2
static inline __m128i
mq2_add_x8(__m128i x, __m128i y)
{
	__m128i qq = _mm_set1_epi16(Q2);
	__m128i a = _mm_sub_epi16(qq, _mm_add_epi16(x, y));
	__m128i b = _mm_add_epi16(a, _mm_and_si128(qq, _mm_srai_epi16(a, 15)));
	return _mm_sub_epi16(qq, b);
}
#elif FNDSA_NEON
TARGET_NEON
static inline int16x8_t
mq2_add_x8(int16x8_t x, int16x8_t y)
{
	int16x8_t qq = vdupq_n_s16(Q2);
	int16x8_t a = vsubq_s16(qq, vaddq_s16(x, y));
	int16x8_t b = vaddq_s16(a, vandq_s16(qq, vshrq_n_s16(a, 15)));
	return vsubq_s16(qq, b);
}
#endif

static inline uint32_t
mq2_sub(uint32_t x, uint32_t y)
{
	/* -(x-y) in [-(q'-1),+q'-1] */
	y -= x;
	/* normalize to [0,+q'-1] */
	y += Q2 & (y >> 16);
	/* negate to get x-y in [1,q'] */
	return Q2 - y;
}

#if FNDSA_SSE2
TARGET_SSE2
static inline __m128i
mq2_sub_x8(__m128i x, __m128i y)
{
	__m128i qq = _mm_set1_epi16(Q2);
	__m128i a = _mm_sub_epi16(y, x);
	__m128i b = _mm_add_epi16(a, _mm_and_si128(qq, _mm_srai_epi16(a, 15)));
	return _mm_sub_epi16(qq, b);
}
#elif FNDSA_NEON
TARGET_NEON
static inline int16x8_t
mq2_sub_x8(int16x8_t x, int16x8_t y)
{
	int16x8_t qq = vdupq_n_s16(Q2);
	int16x8_t a = vsubq_s16(y, x);
	int16x8_t b = vaddq_s16(a, vandq_s16(qq, vshrq_n_s16(a, 15)));
	return vsubq_s16(qq, b);
}
#endif

static inline uint32_t
mq2_half(uint32_t x)
{
	x += Q2 & -(x & 1);
	return x >> 1;
}

#if FNDSA_SSE2
TARGET_SSE2
static inline __m128i
mq2_half_x8(__m128i x)
{
	__m128i qq = _mm_set1_epi16(Q2);
	__m128i y = _mm_and_si128(x, _mm_set1_epi16(1));
	y = _mm_sub_epi16(_mm_setzero_si128(), y);
	y = _mm_and_si128(y, qq);
	return _mm_srli_epi16(_mm_add_epi16(x, y), 1);
}
#elif FNDSA_NEON
TARGET_NEON
static inline int16x8_t
mq2_half_x8(int16x8_t x)
{
	int16x8_t qq = vdupq_n_s16(Q2);
	int16x8_t y = vshrq_n_s16(vshlq_n_s16(x, 15), 15);
	y = vandq_s16(y, qq);
	return vreinterpretq_s16_u16(
		vshrq_n_u16(vreinterpretq_u16_s16(vaddq_s16(x, y)), 1));
}
#endif

static inline uint32_t
mq2_mred(uint32_t x)
{
	x *= Q1I;
	x = (x >> 16) * Q2;
	return (x >> 16) + 1;
}

#if FNDSA_SSE2

TARGET_SSE2
static inline __m128i
mq2_mred_x8(__m128i lo, __m128i hi)
{
	__m128i qx8 = _mm_set1_epi16(Q2);
	__m128i q1ilox8 = _mm_set1_epi16(Q1Ilo);
	__m128i q1ihix8 = _mm_set1_epi16(Q1Ihi);

	/* x <- (x * Q1I) >> 16
	   32-bit input is split into its low and high halves. Q1I is
	   itself a 32-bit constant. Product is computed modulo 2^32,
	   and we are interested only in its high 16 bits. */
	__m128i x = _mm_add_epi16(
		_mm_add_epi16(
			_mm_mulhi_epu16(lo, q1ilox8),
			_mm_mullo_epi16(lo, q1ihix8)),
		_mm_mullo_epi16(hi, q1ilox8));

	/* x <- (x * q') >> 16
	   x and q' both fit on 16 bits each. */
	x = _mm_mulhi_epu16(x, qx8);

	/* Result is x + 1. */
	return _mm_add_epi16(x, _mm_set1_epi16(1));
}

TARGET_SSE2
static inline __m128i
mq2_mmul_x8(__m128i x, __m128i y)
{
	return mq2_mred_x8(_mm_mullo_epi16(x, y), _mm_mulhi_epu16(x, y));
}

#elif FNDSA_NEON

TARGET_NEON
static inline int16x8_t
mq2_mmul_x8(int16x8_t a, int16x8_t b)
{
	/* Compute 32-bit products. */
	uint32x4_t c0 = vreinterpretq_u32_s32(
		vmull_s16(vget_low_s16(a), vget_low_s16(b)));
	uint32x4_t c1 = vreinterpretq_u32_s32(
		vmull_s16(vget_high_s16(a), vget_high_s16(b)));

	/* x <- (x * Q1I) >> 16, reassembled into a single word. */
	uint32x4_t q1ix8 = vdupq_n_u32(Q1I);
	c0 = vmulq_u32(c0, q1ix8);
	c1 = vmulq_u32(c1, q1ix8);
	uint16x8_t d = vuzp2q_u16(
		vreinterpretq_u16_u32(c0),
		vreinterpretq_u16_u32(c1));

	/* x <- (x * q') >> 16
	   x and q' both fit on 16 bits each. */
	uint16x4_t qx8 = vdup_n_u16(Q2);
	c0 = vmull_u16(vget_low_u16(d), qx8);
	c1 = vmull_u16(vget_high_u16(d), qx8);
	d = vuzp2q_u16(vreinterpretq_u16_u32(c0), vreinterpretq_u16_u32(c1));

	return vreinterpretq_s16_u16(vaddq_u16(d, vdupq_n_u16(1)));
}

#endif

#endif

static inline uint32_t
mq2_mmul(uint32_t x, uint32_t y) {
	return mq2_mred(x * y);
}

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_small_to_mod(unsigned logn, const int8_t *f, uint16_t *d)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 4) {
		const __m128i *fp = (const __m128i *)f;
		__m128i *dp = (__m128i *)d;
		__m128i qq = _mm_set1_epi16(Q2);
		for (size_t i = 0; i < (1u << (logn - 4)); i ++) {
			__m128i x = _mm_loadu_si128(fp + i);
			x = _mm_sub_epi8(_mm_setzero_si128(), x);
			__m128i x0 = _mm_srai_epi16(
				_mm_unpacklo_epi8(_mm_setzero_si128(), x), 8);
			__m128i x1 = _mm_srai_epi16(
				_mm_unpackhi_epi8(_mm_setzero_si128(), x), 8);
			x0 = _mm_add_epi16(x0,
				_mm_and_si128(_mm_srai_epi16(x0, 15), qq));
			x1 = _mm_add_epi16(x1,
				_mm_and_si128(_mm_srai_epi16(x1, 15), qq));
			x0 = _mm_sub_epi16(qq, x0);
			x1 = _mm_sub_epi16(qq, x1);
			_mm_storeu_si128(dp + (i << 1) + 0, x0);
			_mm_storeu_si128(dp + (i << 1) + 1, x1);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 4) {
		int16x8_t zero = vdupq_n_s16(0);
		int16x8_t qq = vdupq_n_s16(Q2);
		for (size_t i = 0; i < n; i += 16) {
			int8x16_t x = vld1q_s8(f + i);
			int16x8_t x0 = vmovl_s8(vget_low_s8(x));
			int16x8_t x1 = vmovl_s8(vget_high_s8(x));
			x0 = vaddq_s16(x0, vandq_s16(qq, vcleq_s16(x0, zero)));
			x1 = vaddq_s16(x1, vandq_s16(qq, vcleq_s16(x1, zero)));
			vst1q_s16((int16_t *)d + i, x0);
			vst1q_s16((int16_t *)d + i + 8, x1);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = -(uint32_t)f[i];
		d[i] = (uint16_t)(Q2 - (x + (Q2 & (x >> 16))));
	}
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_signed_to_mod(unsigned logn, uint16_t *d)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i *dp = (__m128i *)d;
		__m128i qq = _mm_set1_epi16(Q2);
		for (size_t i = 0; i < (1u << (logn - 3)); i ++) {
			__m128i y = _mm_loadu_si128(dp + i);
			y = _mm_sub_epi16(_mm_setzero_si128(), y);
			y = _mm_add_epi16(y,
				_mm_and_si128(qq, _mm_srai_epi16(y, 15)));
			y = _mm_sub_epi16(qq, y);
			_mm_storeu_si128(dp + i, y);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t zero = vdupq_n_s16(0);
		int16x8_t qq = vdupq_n_s16(Q2);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t x = vld1q_s16((int16_t *)d + i);
			x = vaddq_s16(x, vandq_s16(qq, vcleq_s16(x, zero)));
			vst1q_s16((int16_t *)d + i, x);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = -(uint32_t)*(int16_t *)&d[i];
		d[i] = (uint16_t)(Q2 - (x + (Q2 & (x >> 16))));
	}
}
#endif

/* see inner.h */
TARGET_SSE2
void
mq2poly_mod_to_signed(unsigned logn, uint16_t *d)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xq = _mm_set1_epi16(Q2);
		__m128i xh = _mm_set1_epi16((Q2 - 1) >> 1);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xd = _mm_loadu_si128((__m128i *)(d + i));
			xd = _mm_sub_epi16(xd,
				_mm_and_si128(xq, _mm_cmpgt_epi16(xd, xh)));
			_mm_storeu_si128((__m128i *)(d + i), xd);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xq = vdupq_n_s16(Q2);
		int16x8_t xh = vdupq_n_s16((Q2 - 1) >> 1);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xd = vld1q_s16((int16_t *)(d + i));
			xd = vsubq_s16(xd, vandq_s16(xq, vcgtq_s16(xd, xh)));
			vst1q_s16((int16_t *)(d + i), xd);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = d[i];
		x -= Q2 & ((((Q2 - 1) >> 1) - x) >> 16);
		d[i] = (uint16_t)x;
	}
}

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_unsigned_to_mod(unsigned logn, uint16_t *d)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xq = _mm_set1_epi16(Q2);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xd = _mm_loadu_si128((__m128i *)(d + i));
			xd = _mm_or_si128(xd, _mm_and_si128(xq,
				_mm_cmpeq_epi16(xd, _mm_setzero_si128())));
			_mm_storeu_si128((__m128i *)(d + i), xd);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xz = vdupq_n_s16(0);
		int16x8_t xq = vdupq_n_s16(Q2);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xd = vld1q_s16((int16_t *)(d + i));
			xd = vorrq_s16(xd, vandq_s16(xq, vceqq_s16(xd, xz)));
			vst1q_s16((int16_t *)(d + i), xd);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = d[i];
		x += Q2 & ((x - 1) >> 16);
		d[i] = (uint16_t)x;
	}
}
#endif

#if !FNDSA_ASM_CORTEXM4

#if FNDSA_SSE2

TARGET_SSE2
static inline void
sse2_NTT16(__m128i *a0, __m128i *a1, size_t k)
{
	__m128i xa0, xa1, xt1, xt2, xt3, xt4, xg1, xg2, xg3;
	uint16_t g1_0, g1_1;

	xa0 = *a0;
	xa1 = *a1;

	/* t = 16, m = 1 */
	xt1 = xa0;
	xt2 = mq2_mmul_x8(xa1, _mm_set1_epi16(mq2_GM[k]));
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */

	/* t = 8, m = 2 */
	xt1 = _mm_unpacklo_epi64(xa0, xa1);
	xt2 = _mm_unpackhi_epi64(xa0, xa1);
	g1_0 = mq2_GM[(k << 1) + 0];
	g1_1 = mq2_GM[(k << 1) + 1];
	xg1 = _mm_setr_epi16(g1_0, g1_0, g1_0, g1_0, g1_1, g1_1, g1_1, g1_1);
	xt2 = mq2_mmul_x8(xt2, xg1);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  2  3  8  9 10 11 */
	/* xa1:  4  5  6  7 12 13 14 15 */

	/* t = 4, m = 4 */
	xt3 = _mm_shuffle_epi32(xa0, 0xD8);
	xt4 = _mm_shuffle_epi32(xa1, 0xD8);
	xt1 = _mm_unpacklo_epi32(xt3, xt4);
	xt2 = _mm_unpackhi_epi32(xt3, xt4);
	xg2 = _mm_setr_epi32(
		*(int32_t *)(mq2_GM + (k << 2)),
		*(int32_t *)(mq2_GM + (k << 2) + 2),
		0, 0);
	xg2 = _mm_unpacklo_epi16(xg2, xg2);
	xt2 = mq2_mmul_x8(xt2, xg2);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  4  5  8  9 12 13 */
	/* xa1:  2  3  6  7 10 11 14 15 */

	/* t = 2, m = 8 */
	xt3 = _mm_unpacklo_epi16(xa0, xa1);
	xt4 = _mm_unpackhi_epi16(xa0, xa1);
	/* xt3:  0  2  1  3  4  6  5  7 */
	/* xt4:  8 10  9 11 12 14 13 15 */
	xt3 = _mm_shuffle_epi32(xt3, 0xD8);
	xt4 = _mm_shuffle_epi32(xt4, 0xD8);
	/* xt3:  0  2  4  6  1  3  5  7 */
	/* xt4:  8 10 12 14  9 11 13 15 */
	xt1 = _mm_unpacklo_epi64(xt3, xt4);
	xt2 = _mm_unpackhi_epi64(xt3, xt4);
	xg3 = _mm_loadu_si128((__m128i *)(mq2_GM + (k << 3)));
	xt2 = mq2_mmul_x8(xt2, xg3);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  2  4  6  8 10 12 14 */
	/* xa1:  1  3  5  7  9 11 13 15 */
	*a0 = _mm_unpacklo_epi16(xa0, xa1);
	*a1 = _mm_unpackhi_epi16(xa0, xa1);
}

/* see inner.h */
TARGET_SSE2
void
mq2poly_NTT(unsigned logn, uint16_t *d)
{
	if (logn >= 4) {
		__m128i *dp = (__m128i *)d;
		size_t n = (size_t)1 << logn;
		size_t t = n >> 3;
		for (unsigned lm = 0; lm < (logn - 4); lm ++) {
			size_t m = (size_t)1 << lm;
			size_t ht = t >> 1;
			size_t j0 = 0;
			for (size_t i = 0; i < m; i ++) {
				__m128i xs = _mm_set1_epi16(mq2_GM[i + m]);
				for (size_t j = 0; j < ht; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + ht;
					__m128i x1, x2;
					x1 = _mm_loadu_si128(dp + j1);
					x2 = _mm_loadu_si128(dp + j2);
					x2 = mq2_mmul_x8(x2, xs);
					_mm_storeu_si128(dp + j1,
						mq2_add_x8(x1, x2));
					_mm_storeu_si128(dp + j2,
						mq2_sub_x8(x1, x2));
				}
				j0 += t;
			}
			t = ht;
		}
		size_t m = n >> 4;
		for (size_t i = 0; i < m; i ++) {
			__m128i xa0 = _mm_loadu_si128(dp + (i << 1) + 0);
			__m128i xa1 = _mm_loadu_si128(dp + (i << 1) + 1);
			sse2_NTT16(&xa0, &xa1, i + m);
			_mm_storeu_si128(dp + (i << 1) + 0, xa0);
			_mm_storeu_si128(dp + (i << 1) + 1, xa1);
		}
	} else {
		size_t t = (size_t)1 << logn;
		for (unsigned lm = 0; lm < logn; lm ++) {
			size_t m = (size_t)1 << lm;
			size_t ht = t >> 1;
			size_t j0 = 0;
			for (size_t i = 0; i < m; i ++) {
				uint32_t s = mq2_GM[i + m];
				for (size_t j = 0; j < ht; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + ht;
					uint32_t x1 = d[j1];
					uint32_t x2 = mq2_mmul(d[j2], s);
					d[j1] = (uint16_t)mq2_add(x1, x2);
					d[j2] = (uint16_t)mq2_sub(x1, x2);
				}
				j0 += t;
			}
			t = ht;
		}
	}
}

#elif FNDSA_NEON

TARGET_NEON
static inline void
neon_NTT16(int16x8_t *a0, int16x8_t *a1, size_t k)
{
	int16x8_t xa0, xa1, xt1, xt2, xt3, xt4, xg1, xg2, xg3;
	uint16_t g1_0, g1_1;

	xa0 = *a0;
	xa1 = *a1;

	/* t = 16, m = 1 */
	xt1 = xa0;
	xt2 = mq2_mmul_x8(xa1, vdupq_n_s16(mq2_GM[k]));
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */

	/* t = 8, m = 2 */
	xt1 = vreinterpretq_s16_s64(vzip1q_s64(
		vreinterpretq_s64_s16(xa0),
		vreinterpretq_s64_s16(xa1)));
	xt2 = vreinterpretq_s16_s64(vzip2q_s64(
		vreinterpretq_s64_s16(xa0),
		vreinterpretq_s64_s16(xa1)));
	g1_0 = mq2_GM[(k << 1) + 0];
	g1_1 = mq2_GM[(k << 1) + 1];
	xg1 = vcombine_s16(vdup_n_s16(g1_0), vdup_n_s16(g1_1));
	xt2 = mq2_mmul_x8(xt2, xg1);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  2  3  8  9 10 11 */
	/* xa1:  4  5  6  7 12 13 14 15 */

	/* t = 4, m = 4 */
	xt1 = vreinterpretq_s16_s32(vtrn1q_s32(
		vreinterpretq_s32_s16(xa0),
		vreinterpretq_s32_s16(xa1)));
	xt2 = vreinterpretq_s16_s32(vtrn2q_s32(
		vreinterpretq_s32_s16(xa0),
		vreinterpretq_s32_s16(xa1)));
	xg2 = vreinterpretq_s16_u64(
		vdupq_n_u64(*(uint64_t *)(mq2_GM + (k << 2))));
	xg2 = vzip1q_s16(xg2, xg2);
	xt2 = mq2_mmul_x8(xt2, xg2);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  1  4  5  8  9 12 13 */
	/* xa1:  2  3  6  7 10 11 14 15 */

	/* t = 2, m = 8 */
	xt1 = vtrn1q_s16(xa0, xa1);
	xt2 = vtrn2q_s16(xa0, xa1);
	xg3 = vld1q_s16((int16_t *)(mq2_GM + (k << 3)));
	xt2 = mq2_mmul_x8(xt2, xg3);
	xa0 = mq2_add_x8(xt1, xt2);
	xa1 = mq2_sub_x8(xt1, xt2);

	/* xa0:  0  2  4  6  8 10 12 14 */
	/* xa1:  1  3  5  7  9 11 13 15 */
	*a0 = vzip1q_s16(xa0, xa1);
	*a1 = vzip2q_s16(xa0, xa1);
}

/* see inner.h */
TARGET_NEON
void
mq2poly_NTT(unsigned logn, uint16_t *d)
{
	if (logn >= 4) {
		size_t n = (size_t)1 << logn;
		size_t t = n >> 3;
		for (unsigned lm = 0; lm < (logn - 4); lm ++) {
			size_t m = (size_t)1 << lm;
			size_t ht = t >> 1;
			size_t j0 = 0;
			for (size_t i = 0; i < m; i ++) {
				int16x8_t xs = vdupq_n_s16(mq2_GM[i + m]);
				for (size_t j = 0; j < ht; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + ht;
					int16x8_t x1 = vld1q_s16(
						(int16_t *)d + (j1 << 3));
					int16x8_t x2 = vld1q_s16(
						(int16_t *)d + (j2 << 3));
					x2 = mq2_mmul_x8(x2, xs);
					vst1q_s16((int16_t *)d + (j1 << 3),
						mq2_add_x8(x1, x2));
					vst1q_s16((int16_t *)d + (j2 << 3),
						mq2_sub_x8(x1, x2));
				}
				j0 += t;
			}
			t = ht;
		}
		size_t m = n >> 4;
		for (size_t i = 0; i < m; i ++) {
			int16x8_t xa0 = vld1q_s16((int16_t *)d + (i << 4) + 0);
			int16x8_t xa1 = vld1q_s16((int16_t *)d + (i << 4) + 8);
			neon_NTT16(&xa0, &xa1, i + m);
			vst1q_s16((int16_t *)d + (i << 4) + 0, xa0);
			vst1q_s16((int16_t *)d + (i << 4) + 8, xa1);
		}
	} else {
		size_t t = (size_t)1 << logn;
		for (unsigned lm = 0; lm < logn; lm ++) {
			size_t m = (size_t)1 << lm;
			size_t ht = t >> 1;
			size_t j0 = 0;
			for (size_t i = 0; i < m; i ++) {
				uint32_t s = mq2_GM[i + m];
				for (size_t j = 0; j < ht; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + ht;
					uint32_t x1 = d[j1];
					uint32_t x2 = mq2_mmul(d[j2], s);
					d[j1] = (uint16_t)mq2_add(x1, x2);
					d[j2] = (uint16_t)mq2_sub(x1, x2);
				}
				j0 += t;
			}
			t = ht;
		}
	}
}

#else

/* see inner.h */
void
mq2poly_NTT(unsigned logn, uint16_t *d)
{
	size_t t = (size_t)1 << logn;
	for (unsigned lm = 0; lm < logn; lm ++) {
		size_t m = (size_t)1 << lm;
		size_t ht = t >> 1;
		size_t j0 = 0;
		for (size_t i = 0; i < m; i ++) {
			uint32_t s = mq2_GM[i + m];
			for (size_t j = 0; j < ht; j ++) {
				size_t j1 = j0 + j;
				size_t j2 = j1 + ht;
				uint32_t x1 = d[j1];
				uint32_t x2 = mq2_mmul(d[j2], s);
				d[j1] = (uint16_t)mq2_add(x1, x2);
				d[j2] = (uint16_t)mq2_sub(x1, x2);
			}
			j0 += t;
		}
		t = ht;
	}
}

#endif

#endif

#if !FNDSA_ASM_CORTEXM4

#if FNDSA_SSE2

TARGET_SSE2
static inline void
sse2_iNTT16(__m128i *a0, __m128i *a1, size_t k)
{
	__m128i xa0, xa1, xt1, xt2, xt3, xt4, xig1, xig2, xig3;
	uint16_t ig1_0, ig1_1;

	xa0 = *a0;
	xa1 = *a1;

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */
	xt1 = _mm_unpacklo_epi16(xa0, xa1);
	xt2 = _mm_unpackhi_epi16(xa0, xa1);
	xt3 = _mm_unpacklo_epi16(xt1, xt2);
	xt4 = _mm_unpackhi_epi16(xt1, xt2);
	xa0 = _mm_unpacklo_epi16(xt3, xt4);
	xa1 = _mm_unpackhi_epi16(xt3, xt4);

	/* xa0:  0  2  4  6  8 10 12 14 */
	/* xa1:  1  3  5  7  9 11 13 15 */
	xt1 = mq2_add_x8(xa0, xa1);
	xt2 = mq2_sub_x8(xa0, xa1);
	xig3 = _mm_loadu_si128((__m128i *)(mq2_iGM + (k << 3)));
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig3);

	/* xa0:  0  2  4  6  8 10 12 14 */
	/* xa1:  1  3  5  7  9 11 13 15 */
	xt1 = _mm_unpacklo_epi16(xa0, xa1);
	xt2 = _mm_unpackhi_epi16(xa0, xa1);
	xt3 = _mm_shuffle_epi32(xt1, 0xD8);
	xt4 = _mm_shuffle_epi32(xt2, 0xD8);
	xa0 = _mm_unpacklo_epi64(xt3, xt4);
	xa1 = _mm_unpackhi_epi64(xt3, xt4);

	/* xa0:  0  1  4  5  8  9 12 13 */
	/* xa1:  2  3  6  7 10 11 14 15 */
	xt1 = mq2_add_x8(xa0, xa1);
	xt2 = mq2_sub_x8(xa0, xa1);
	xig2 = _mm_setr_epi32(
		*(int32_t *)(mq2_iGM + (k << 2)),
		*(int32_t *)(mq2_iGM + (k << 2) + 2),
		0, 0);
	xig2 = _mm_unpacklo_epi16(xig2, xig2);
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig2);

	/* xa0:  0  1  4  5  8  9 12 13 */
	/* xa1:  2  3  6  7 10 11 14 15 */
	xt1 = _mm_shuffle_epi32(xa0, 0xD8);
	xt2 = _mm_shuffle_epi32(xa1, 0xD8);
	xa0 = _mm_unpacklo_epi32(xt1, xt2);
	xa1 = _mm_unpackhi_epi32(xt1, xt2);

	/* xa0:  0  1  2  3  8  9 10 11 */
	/* xa1:  4  5  6  7 12 13 14 15 */
	xt1 = mq2_add_x8(xa0, xa1);
	xt2 = mq2_sub_x8(xa0, xa1);
	ig1_0 = mq2_iGM[(k << 1) + 0];
	ig1_1 = mq2_iGM[(k << 1) + 1];
	xig1 = _mm_setr_epi16(
		ig1_0, ig1_0, ig1_0, ig1_0, ig1_1, ig1_1, ig1_1, ig1_1);
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig1);

	/* xa0:  0  1  2  3  8  9 10 11 */
	/* xa1:  4  5  6  7 12 13 14 15 */
	xt1 = _mm_unpacklo_epi64(xa0, xa1);
	xt2 = _mm_unpackhi_epi64(xa0, xa1);
	xa0 = xt1;
	xa1 = xt2;

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */
	xt1 = mq2_add_x8(xa0, xa1);
	xt2 = mq2_sub_x8(xa0, xa1);
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, _mm_set1_epi16(mq2_iGM[k]));

	*a0 = xa0;
	*a1 = xa1;
}

/* see inner.h */
TARGET_SSE2
void
mq2poly_iNTT(unsigned logn, uint16_t *d)
{
	if (logn >= 4) {
		__m128i *dp = (__m128i *)d;
		size_t n = (size_t)1 << logn;
		size_t m = n >> 4;
		for (size_t i = 0; i < m; i ++) {
			__m128i xa0 = _mm_loadu_si128(dp + (i << 1) + 0);
			__m128i xa1 = _mm_loadu_si128(dp + (i << 1) + 1);
			sse2_iNTT16(&xa0, &xa1, i + m);
			_mm_storeu_si128(dp + (i << 1) + 0, xa0);
			_mm_storeu_si128(dp + (i << 1) + 1, xa1);
		}
		size_t t = 2;
		for (unsigned lm = 4; lm < logn; lm ++) {
			size_t hm = (size_t)1 << (logn - 1 - lm);
			size_t dt = t << 1;
			size_t j0 = 0;
			for (size_t i = 0; i < hm; i ++) {
				__m128i xs = _mm_set1_epi16(mq2_iGM[i + hm]);
				for (size_t j = 0; j < t; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + t;
					__m128i x1, x2;
					x1 = _mm_loadu_si128(dp + j1);
					x2 = _mm_loadu_si128(dp + j2);
					_mm_storeu_si128(dp + j1,
						mq2_half_x8(
							mq2_add_x8(x1, x2)));
					_mm_storeu_si128(dp + j2,
						mq2_mmul_x8(xs,
							mq2_sub_x8(x1, x2)));
				}
				j0 += dt;
			}
			t = dt;
		}
	} else {
		size_t t = 1;
		for (unsigned lm = 0; lm < logn; lm ++) {
			size_t hm = (size_t)1 << (logn - 1 - lm);
			size_t dt = t << 1;
			size_t j0 = 0;
			for (size_t i = 0; i < hm; i ++) {
				uint32_t s = mq2_iGM[i + hm];
				for (size_t j = 0; j < t; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + t;
					uint32_t x1 = d[j1];
					uint32_t x2 = d[j2];
					d[j1] = mq2_half(mq2_add(x1, x2));
					d[j2] = mq2_mmul(mq2_sub(x1, x2), s);
				}
				j0 += dt;
			}
			t = dt;
		}
	}
}

#elif FNDSA_NEON

TARGET_NEON
static inline void
neon_iNTT16(int16x8_t *a0, int16x8_t *a1, size_t k)
{
	int16x8_t xa0, xa1, xt1, xt2, xt3, xt4, xig1, xig2, xig3;
	uint16_t ig1_0, ig1_1;

	xa0 = *a0;
	xa1 = *a1;

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */
	xt3 = vuzp1q_s16(xa0, xa1);
	xt4 = vuzp2q_s16(xa0, xa1);
	xt1 = mq2_add_x8(xt3, xt4);
	xt2 = mq2_sub_x8(xt3, xt4);
	xig3 = vld1q_s16((int16_t *)(mq2_iGM + (k << 3)));
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig3);

	/* xa0:  0  2  4  6  8 10 12 14 */
	/* xa1:  1  3  5  7  9 11 13 15 */
	xt3 = vtrn1q_s16(xa0, xa1);
	xt4 = vtrn2q_s16(xa0, xa1);
	xt1 = mq2_add_x8(xt3, xt4);
	xt2 = mq2_sub_x8(xt3, xt4);
	xig2 = vreinterpretq_s16_u64(
		vdupq_n_u64(*(uint64_t *)(mq2_iGM + (k << 2))));
	xig2 = vzip1q_s16(xig2, xig2);
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig2);

	/* xa0:  0  1  4  5  8  9 12 13 */
	/* xa1:  2  3  6  7 10 11 14 15 */
	xt3 = vreinterpretq_s16_s32(vtrn1q_s32(
		vreinterpretq_s32_s16(xa0), vreinterpretq_s32_s16(xa1)));
	xt4 = vreinterpretq_s16_s32(vtrn2q_s32(
		vreinterpretq_s32_s16(xa0), vreinterpretq_s32_s16(xa1)));
	xt1 = mq2_add_x8(xt3, xt4);
	xt2 = mq2_sub_x8(xt3, xt4);
	ig1_0 = mq2_iGM[(k << 1) + 0];
	ig1_1 = mq2_iGM[(k << 1) + 1];
	xig1 = vcombine_s16(vdup_n_s16(ig1_0), vdup_n_s16(ig1_1));
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, xig1);

	/* xa0:  0  1  2  3  8  9 10 11 */
	/* xa1:  4  5  6  7 12 13 14 15 */
	xt3 = vreinterpretq_s16_s64(vzip1q_s64(
		vreinterpretq_s64_s16(xa0), vreinterpretq_s64_s16(xa1)));
	xt4 = vreinterpretq_s16_s64(vzip2q_s64(
		vreinterpretq_s64_s16(xa0), vreinterpretq_s64_s16(xa1)));
	xt1 = mq2_add_x8(xt3, xt4);
	xt2 = mq2_sub_x8(xt3, xt4);
	xa0 = mq2_half_x8(xt1);
	xa1 = mq2_mmul_x8(xt2, vdupq_n_s16(mq2_iGM[k]));

	/* xa0:  0  1  2  3  4  5  6  7 */
	/* xa1:  8  9 10 11 12 13 14 15 */
	*a0 = xa0;
	*a1 = xa1;
}

/* see inner.h */
TARGET_NEON
void
mq2poly_iNTT(unsigned logn, uint16_t *d)
{
	if (logn >= 4) {
		size_t n = (size_t)1 << logn;
		size_t m = n >> 4;
		for (size_t i = 0; i < m; i ++) {
			int16x8_t xa0 = vld1q_s16((int16_t *)d + (i << 4) + 0);
			int16x8_t xa1 = vld1q_s16((int16_t *)d + (i << 4) + 8);
			neon_iNTT16(&xa0, &xa1, i + m);
			vst1q_s16((int16_t *)d + (i << 4) + 0, xa0);
			vst1q_s16((int16_t *)d + (i << 4) + 8, xa1);
		}
		size_t t = 2;
		for (unsigned lm = 4; lm < logn; lm ++) {
			size_t hm = (size_t)1 << (logn - 1 - lm);
			size_t dt = t << 1;
			size_t j0 = 0;
			for (size_t i = 0; i < hm; i ++) {
				int16x8_t xs = vdupq_n_s16(mq2_iGM[i + hm]);
				for (size_t j = 0; j < t; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + t;
					int16x8_t x1 = vld1q_s16(
						(int16_t *)d + (j1 << 3));
					int16x8_t x2 = vld1q_s16(
						(int16_t *)d + (j2 << 3));
					vst1q_s16((int16_t *)d + (j1 << 3),
						mq2_half_x8(
							mq2_add_x8(x1, x2)));
					vst1q_s16((int16_t *)d + (j2 << 3),
						mq2_mmul_x8(xs,
							mq2_sub_x8(x1, x2)));
				}
				j0 += dt;
			}
			t = dt;
		}
	} else {
		size_t t = 1;
		for (unsigned lm = 0; lm < logn; lm ++) {
			size_t hm = (size_t)1 << (logn - 1 - lm);
			size_t dt = t << 1;
			size_t j0 = 0;
			for (size_t i = 0; i < hm; i ++) {
				uint32_t s = mq2_iGM[i + hm];
				for (size_t j = 0; j < t; j ++) {
					size_t j1 = j0 + j;
					size_t j2 = j1 + t;
					uint32_t x1 = d[j1];
					uint32_t x2 = d[j2];
					d[j1] = mq2_half(mq2_add(x1, x2));
					d[j2] = mq2_mmul(mq2_sub(x1, x2), s);
				}
				j0 += dt;
			}
			t = dt;
		}
	}
}

#else

/* see inner.h */
void
mq2poly_iNTT(unsigned logn, uint16_t *d)
{
	if (logn == 0) {
		return;
	}
	size_t t = 1;
	for (unsigned lm = 0; lm < logn; lm ++) {
		size_t hm = (size_t)1 << (logn - 1 - lm);
		size_t dt = t << 1;
		size_t j0 = 0;
		for (size_t i = 0; i < hm; i ++) {
			uint32_t s = mq2_iGM[i + hm];
			for (size_t j = 0; j < t; j ++) {
				size_t j1 = j0 + j;
				size_t j2 = j1 + t;
				uint32_t x1 = d[j1];
				uint32_t x2 = d[j2];
				d[j1] = (uint16_t)mq2_half(mq2_add(x1, x2));
				d[j2] = (uint16_t)mq2_mmul(mq2_sub(x1, x2), s);
			}
			j0 += dt;
		}
		t = dt;
	}
}

#endif

#endif

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_mul_ntt(unsigned logn, uint16_t *a, const uint16_t *b)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xR2 = _mm_set1_epi16(R2);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xb = _mm_loadu_si128((__m128i *)(b + i));
			__m128i xc = mq2_mmul_x8(mq2_mmul_x8(xa, xb), xR2);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xR2 = vdupq_n_s16(R2);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xb = vld1q_s16((int16_t *)(b + i));
			int16x8_t xc = mq2_mmul_x8(mq2_mmul_x8(xa, xb), xR2);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_mmul(mq2_mmul(a[i], b[i]), R2);
	}
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see sign_inner.h */
TARGET_SSE2
void
mq2poly_muladj_ntt(unsigned logn, uint16_t *a, const uint16_t *b)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xR2 = _mm_set1_epi16(R2);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xb = _mm_loadu_si128(
				(__m128i *)(b + n - 8 - i));
			xb = _mm_or_si128(
				_mm_srli_epi32(xb, 16),
				_mm_slli_epi32(xb, 16));
			xb = _mm_shuffle_epi32(xb, 0x1B);
			__m128i xc = mq2_mmul_x8(mq2_mmul_x8(xa, xb), xR2);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xR2 = vdupq_n_s16(R2);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xb = vld1q_s16((int16_t *)(b + n - 8 - i));
			xb = vrev64q_s16(xb);
			xb = vextq_s16(xb, xb, 4);
			int16x8_t xc = mq2_mmul_x8(mq2_mmul_x8(xa, xb), xR2);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_mmul(mq2_mmul(a[i], b[n - 1 - i]), R2);
	}
}
#endif

/* see inner.h */
TARGET_SSE2
void
mq2poly_muladj_x2_ntt(unsigned logn, uint16_t *a, const uint16_t *b)
{
	// TODO: Arm Cortex M4 optimization
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
#if FNDSA_SSE2
	if (logn >= 4) {
		__m128i xR2 = _mm_set1_epi16(R2);
		for (size_t i = 0; i < hn; i += 8) {
			__m128i xa0 = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xa1 = _mm_loadu_si128(
				(__m128i *)(a + n - 8 - i));
			__m128i xb0 = _mm_loadu_si128((__m128i *)(b + i));
			__m128i xb1 = _mm_loadu_si128(
				(__m128i *)(b + n - 8 - i));
			xa1 = _mm_or_si128(
				_mm_srli_epi32(xa1, 16),
				_mm_slli_epi32(xa1, 16));
			xa1 = _mm_shuffle_epi32(xa1, 0x1B);
			xb1 = _mm_or_si128(
				_mm_srli_epi32(xb1, 16),
				_mm_slli_epi32(xb1, 16));
			xb1 = _mm_shuffle_epi32(xb1, 0x1B);
			__m128i xc = mq2_mmul_x8(mq2_add_x8(
				mq2_mmul_x8(xa0, xa1),
				mq2_mmul_x8(xb0, xb1)), xR2);
			_mm_storeu_si128((__m128i *)(a + i), xc);
			xc = _mm_or_si128(
				_mm_srli_epi32(xc, 16),
				_mm_slli_epi32(xc, 16));
			xc = _mm_shuffle_epi32(xc, 0x1B);
			_mm_storeu_si128((__m128i *)(a + n - 8 - i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 4) {
		int16x8_t xR2 = vdupq_n_s16(R2);
		for (size_t i = 0; i < hn; i += 8) {
			int16x8_t xa0 = vld1q_s16((int16_t *)(a + i));
			int16x8_t xa1 = vld1q_s16(
				(int16_t *)(a + n - 8 - i));
			int16x8_t xb0 = vld1q_s16((int16_t *)(b + i));
			int16x8_t xb1 = vld1q_s16(
				(int16_t *)(b + n - 8 - i));
			xa1 = vrev64q_s16(xa1);
			xb1 = vrev64q_s16(xb1);
			xa1 = vextq_s16(xa1, xa1, 4);
			xb1 = vextq_s16(xb1, xb1, 4);
			int16x8_t xc = mq2_mmul_x8(mq2_add_x8(
				mq2_mmul_x8(xa0, xa1),
				mq2_mmul_x8(xb0, xb1)), xR2);
			vst1q_s16((int16_t *)(a + i), xc);
			xc = vrev64q_s16(xc);
			xc = vextq_s16(xc, xc, 4);
			vst1q_s16((int16_t *)(a + n - 8 - i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < hn; i ++) {
		a[i] = (uint16_t)mq2_mmul(mq2_add(
			mq2_mmul(a[i], a[n - 1 - i]),
			mq2_mmul(b[i], b[n - 1 - i])), R2);
	}
	for (size_t i = hn; i < n; i ++) {
		a[i] = a[n - 1 - i];
	}
}

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_sub(unsigned logn, uint16_t *a, const uint16_t *b)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xb = _mm_loadu_si128((__m128i *)(b + i));
			__m128i xc = mq2_sub_x8(xa, xb);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xb = vld1q_s16((int16_t *)(b + i));
			int16x8_t xc = mq2_sub_x8(xa, xb);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_sub(a[i], b[i]);
	}
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
TARGET_SSE2
void
mq2poly_add(unsigned logn, uint16_t *a, const uint16_t *b)
{
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xb = _mm_loadu_si128((__m128i *)(b + i));
			__m128i xc = mq2_add_x8(xa, xb);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xb = vld1q_s16((int16_t *)(b + i));
			int16x8_t xc = mq2_add_x8(xa, xb);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_add(a[i], b[i]);
	}
}
#endif

#if !FNDSA_ASM_CORTEXM4
/* see sign_inner.h */
TARGET_SSE2
void
mq2poly_mulconst(unsigned logn, uint16_t *a, unsigned m)
{
	size_t n = (size_t)1 << logn;
	m = mq2_mmul(m, R2);
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xm = _mm_set1_epi16(m);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xc = mq2_mmul_x8(xa, xm);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xm = vdupq_n_s16(m);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xc = mq2_mmul_x8(xa, xm);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_mmul(a[i], m);
	}
}
#endif

/* see sign_inner.h */
TARGET_SSE2
void
mq2poly_addmulconst(unsigned logn, uint16_t *a, unsigned m, unsigned s)
{
	// TODO: Arm Cortex M4 optimization
	size_t n = (size_t)1 << logn;
	m = mq2_mmul(m, R2);
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xm = _mm_set1_epi16(m);
		__m128i xs = _mm_set1_epi16(s);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xa = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xc = mq2_mmul_x8(mq2_add_x8(xa, xs), xm);
			_mm_storeu_si128((__m128i *)(a + i), xc);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xm = vdupq_n_s16(m);
		int16x8_t xs = vdupq_n_s16(s);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xa = vld1q_s16((int16_t *)(a + i));
			int16x8_t xc = mq2_mmul_x8(mq2_add_x8(xa, xs), xm);
			vst1q_s16((int16_t *)(a + i), xc);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		a[i] = (uint16_t)mq2_mmul(mq2_add(a[i], s), m);
	}
}

/* see sign_inner.h */
TARGET_SSE2
void
mq2mqpoly_adjust_save(unsigned logn, int16_t *a, const int16_t *b, uint8_t *adj)
{
	// TODO: Arm Cortex M4 optimization
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xq2 = _mm_set1_epi16(Q2);
		__m128i xqp = _mm_set1_epi16(Q);
		__m128i xqm = _mm_set1_epi16(-Q);
		for (size_t i = 0; i < n; i += 8) {
			__m128i x1 = _mm_loadu_si128((__m128i *)(a + i));
			__m128i x2 = _mm_loadu_si128((__m128i *)(b + i));
			__m128i x = _mm_sub_epi16(x1, x2);

			/* Adjustment is needed if x is not equal to
			   -q, 0 or +q */
			__m128i tp = _mm_cmpeq_epi16(x, xqp);
			__m128i tz = _mm_cmpeq_epi16(x, _mm_setzero_si128());
			__m128i tm = _mm_cmpeq_epi16(x, xqm);
			__m128i xm = _mm_or_si128(_mm_or_si128(tp, tm), tz);
			xm = _mm_xor_si128(xm, _mm_set1_epi16(-1));

			/* Extract and store adjustement bits. */
			unsigned m = _mm_movemask_epi8(xm);
			m = (m & 0x1111) | ((m >> 1) & 0x2222);
			m = (m | (m >> 2)) & 0x0F0F;
			adj[i >> 3] = (uint8_t)(m | (m >> 4));

			/* Adjustment is +q' or -q', depending on the
			   value sign. */
			__m128i xs = _mm_srai_epi16(x1, 15);
			__m128i xd = _mm_sub_epi16(_mm_xor_si128(xq2, xs), xs);
			x1 = _mm_sub_epi16(x1, _mm_and_si128(xm, xd));
			_mm_storeu_si128((__m128i *)(a + i), x1);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xq2 = vdupq_n_s16(Q2);
		int16x8_t xqp = vdupq_n_s16(Q);
		int16x8_t xqm = vdupq_n_s16(-Q);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t x1 = vld1q_s16((int16_t *)(a + i));
			int16x8_t x2 = vld1q_s16((int16_t *)(b + i));
			int16x8_t x = vsubq_s16(x1, x2);

			/* Adjustment is needed if x is not equal to
			   -q, 0 or +q */
			int16x8_t tp = vceqq_s16(x, xqp);
			int16x8_t tz = vceqzq_s16(x);
			int16x8_t tm = vceqq_s16(x, xqm);
			int16x8_t xm = vorrq_s16(vorrq_s16(tp, tm), tz);
			xm = vmvnq_s16(xm);

			/* Extract and store adjustement bits. */
			uint64_t mm = vget_lane_u64(
				vreinterpret_u64_s8(vmovn_s16(xm)), 0);
			mm &= 0x0101010101010101;
			mm |= (mm >> 7);
			mm |= (mm >> 14);
			mm |= (mm >> 28);
			adj[i >> 3] = (uint8_t)mm;

			/* Adjustment is +q' or -q', depending on the
			   value sign. */
			int16x8_t xs = vshrq_n_s16(x1, 15);
			int16x8_t xd = vsubq_s16(veorq_s16(xq2, xs), xs);
			x1 = vsubq_s16(x1, vandq_s16(xm, xd));
			vst1q_s16((int16_t *)(a + i), x1);
		}
		return;
	}
#endif
	if (logn >= 3) {
		memset(adj, 0, n >> 3);
	} else {
		adj[0] = 0;
	}
	for (size_t i = 0; i < n; i ++) {
		int32_t y1 = a[i];
		int32_t y2 = b[i];

		/* Input ranges:
		      -9216 <= y1 <= +9216
		      -6144 <= y2 <= +6144
		   An adjustment on y1 is needed if the two values do not
		   agree, i.e. y1 - y2 != 0 mod q. We have:
		     -15360 <= y1 - y2 <= +15360
		   Thus, for agreeing values, y1 - y2 must be equal to
		   either -12289, 0, or +12289. */
		uint32_t x0 = (uint32_t)(y1 - y2);
		uint32_t xp = x0 - Q;
		uint32_t xm = x0 + Q;
		uint32_t xa = (x0 | -x0) & (xp | -xp) & (xm | -xm);

		/* If y1 - y2 = 0 mod q, then xa = 0; otherwise, the top
		   16 bits of xa are equal to 0xFFFF. */
		adj[i >> 3] |= ((xa >> 16) & 1) << (i & 7);

		/* If y1 < 0, adjustement is +q'.
		   If y1 > 0, adjustement is -q'.
		   If hi16(xa) = 0, then there is no adjustment. */
		uint32_t s = ~(uint32_t)(y1 >> 16);
		uint32_t d = (xa >> 16) & ((Q2 ^ s) - s);
		a[i] = (int16_t)(y1 + *(int32_t *)&d);
	}
}

/* see sign_inner.h */
TARGET_SSE2
void
mq2mqpoly_adjust(unsigned logn, int16_t *a, const uint8_t *adj)
{
	// TODO: Arm Cortex M4 optimization
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xq2 = _mm_set1_epi16(Q2);
		for (size_t i = 0; i < n; i += 8) {
			/* Get mask bits distributed into an SSE2 register
			   (16-bit mask per value). */
			uint32_t m0 = adj[i >> 3];
			m0 |= (m0 << 7) & 0xFF00;
			m0 |= (m0 << 14) & 0xFFFF0000;
			uint32_t m1 = m0 >> 4;
			__m128i xm = _mm_setr_epi32(
				*(int32_t *)&m0, *(int32_t *)&m1, 0, 0);
			xm = _mm_unpacklo_epi8(xm, _mm_setzero_si128());
			xm = _mm_srai_epi16(_mm_slli_epi16(xm, 15), 15);

			/* Adjustment is +q' or -q', depending on the
			   value sign. */
			__m128i x = _mm_loadu_si128((__m128i *)(a + i));
			__m128i xs = _mm_srai_epi16(x, 15);
			x = _mm_sub_epi16(x,
				_mm_and_si128(_mm_andnot_si128(xs, xm), xq2));
			x = _mm_add_epi16(x,
				_mm_and_si128(_mm_and_si128(xs, xm), xq2));
			_mm_storeu_si128((__m128i *)(a + i), x);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xq2 = vdupq_n_s16(Q2);
		for (size_t i = 0; i < n; i += 8) {
			/* Get mask bits distributed into an SSE2 register
			   (16-bit mask per value). */
			uint32_t m0 = adj[i >> 3];
			m0 |= (m0 << 7) & 0xFF00;
			m0 |= (m0 << 14) & 0xFFFF0000;
			uint32_t m1 = m0 >> 4;
			uint64_t mm = (uint64_t)m0 | ((uint64_t)m1 << 32);
			int8x8_t vm = vcreate_s8(mm & 0x0101010101010101);
			int16x8_t xm = vmovl_s8(vneg_s8(vm));

			/* Adjustment is +q' or -q', depending on the
			   value sign. */
			int16x8_t x = vld1q_s16((int16_t *)(a + i));
			int16x8_t xs = vshrq_n_s16(x, 15);
			int16x8_t xd = vsubq_s16(veorq_s16(xq2, xs), xs);
			x = vsubq_s16(x, vandq_s16(xd, xm));
			vst1q_s16((int16_t *)(a + i), x);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		int32_t y1 = a[i];
		uint32_t xa = (adj[i >> 3] >> (i & 7)) & 1;

		/* If y1 < 0, adjustement is +q'.
		   If y1 > 0, adjustement is -q'. */
		uint32_t s = ~(uint32_t)(y1 >> 16);
		uint32_t d = -xa & ((Q2 ^ s) - s);
		a[i] = (int16_t)(y1 + *(int32_t *)&d);
	}
}

/* see sign_inner.h */
TARGET_SSE2
void
mq2mqpoly_CRT(unsigned logn, int32_t *d, const int16_t *aq, const int16_t *aq2)
{
	// TODO: Arm Cortex M4 optimization
	size_t n = (size_t)1 << logn;
#if FNDSA_SSE2
	if (logn >= 3) {
		__m128i xq = _mm_set1_epi16(Q);
		__m128i xq2 = _mm_set1_epi16(Q2);
		__m128i xiq = _mm_set1_epi16(Q2_IQ_MR);
		__m128i xqq2 = _mm_set1_epi32(Q * Q2);
		__m128i xhqq2 = _mm_set1_epi32((Q * Q2 - 1) >> 1);
		for (size_t i = 0; i < n; i += 8) {
			__m128i xmq = _mm_loadu_si128((__m128i *)(aq + i));
			__m128i xmq2 = _mm_loadu_si128((__m128i *)(aq2 + i));

			/* u <- aq2[i] - aq[i] mod q'  (unsigned) */
			__m128i xw = _mm_sub_epi16(xmq, xmq2);
			__m128i xu = _mm_sub_epi16(xq2, _mm_add_epi16(xw,
				_mm_and_si128(xq2, _mm_srai_epi16(xw, 15))));

			/* u <- (aq2[i] - aq[i])/q mod q'  (unsigned) */
			xu = mq2_mmul_x8(xu, xiq);

			/* (y0,y1) <- u*q + aq[i]  (32-bit) */
			__m128i t0 = _mm_mullo_epi16(xu, xq);
			__m128i t1 = _mm_mulhi_epu16(xu, xq);
			__m128i y0 = _mm_unpacklo_epi16(t0, t1);
			__m128i y1 = _mm_unpackhi_epi16(t0, t1);
			__m128i t2 = _mm_srai_epi16(xmq, 15);
			__m128i z0 = _mm_unpacklo_epi16(xmq, t2);
			__m128i z1 = _mm_unpackhi_epi16(xmq, t2);
			y0 = _mm_add_epi32(y0, z0);
			y1 = _mm_add_epi32(y1, z1);

			/* Signed normalization of (y0,y1)  (mod q*q') */
			y0 = _mm_sub_epi32(y0, _mm_and_si128(
				_mm_cmpgt_epi32(y0, xhqq2), xqq2));
			y1 = _mm_sub_epi32(y1, _mm_and_si128(
				_mm_cmpgt_epi32(y1, xhqq2), xqq2));

			/* Store back result. */
			_mm_storeu_si128((__m128i *)(d + i), y0);
			_mm_storeu_si128((__m128i *)(d + i + 4), y1);
		}
		return;
	}
#elif FNDSA_NEON
	if (logn >= 3) {
		int16x8_t xq = vdupq_n_s16(Q);
		int16x4_t xql = vdup_n_s16(Q);
		int16x8_t xq2 = vdupq_n_s16(Q2);
		int16x8_t xiq = vdupq_n_s16(Q2_IQ_MR);
		int32x4_t xqq2 = vdupq_n_s32(Q * Q2);
		int32x4_t xhqq2 = vdupq_n_s32((Q * Q2 - 1) >> 1);
		for (size_t i = 0; i < n; i += 8) {
			int16x8_t xmq = vld1q_s16((int16_t *)(aq + i));
			int16x8_t xmq2 = vld1q_s16((int16_t *)(aq2 + i));

			/* u <- aq2[i] - aq[i] mod q'  (unsigned) */
			int16x8_t xw = vsubq_s16(xmq, xmq2);
			int16x8_t xu = vsubq_s16(xq2, vaddq_s16(xw,
				vandq_s16(xq2, vshrq_n_s16(xw, 15))));

			/* u <- (aq2[i] - aq[i])/q mod q'  (unsigned) */
			xu = mq2_mmul_x8(xu, xiq);

			/* (y0,y1) <- u*q + aq[i]  (32-bit) */
			int32x4_t y0 = vmull_s16(vget_low_s16(xu), xql);
			int32x4_t y1 = vmull_high_s16(xu, xq);
			y0 = vaddq_s32(y0, vmovl_s16(vget_low_s16(xmq)));
			y1 = vaddq_s32(y1, vmovl_s16(vget_high_s16(xmq)));

			/* Signed normalization of (y0,y1)  (mod q*q') */
			y0 = vsubq_s32(y0, vandq_s32(
				vcgtq_s32(y0, xhqq2), xqq2));
			y1 = vsubq_s32(y1, vandq_s32(
				vcgtq_s32(y1, xhqq2), xqq2));

			/* Store back result. */
			vst1q_s32((int32_t *)(d + i), y0);
			vst1q_s32((int32_t *)(d + i + 4), y1);
		}
		return;
	}
#endif
	for (size_t i = 0; i < n; i ++) {
		int32_t xq = aq[i];
		int32_t xq2 = aq2[i];
		uint32_t u = (uint32_t)(xq2 - xq);

		/* Range: -15360 <= u <= +15360 */
		uint32_t w = -u;
		u = Q2 - (w + (Q2 & (w >> 16)));
		u = mq2_mmul(u, Q2_IQ_MR);

		/* Range: 1 <= u <= 18433 */
		u = (uint32_t)xq + Q * u;

		/* Range: -6143 <= u <= 226529281 */
		uint32_t v = (((uint32_t)Q * Q2) >> 1) - u;
		u -= (uint32_t)(*(int32_t *)&v >> 30) & ((uint32_t)Q * Q2);

		d[i] = *(int32_t *)&u;
	}
}

/* Constants are public so that they can be shared with alternate
   implementations. */
ALIGN32
const uint16_t mq2_GM[] = {
	 4564, 17110, 12162, 16208, 10701,  9705,  3451,  5078, 12400, 10202,
	 8245, 13131,  4631,  3492, 17179,  5622,  5537,  3399,  2485,  9938,
	  345, 14064, 10152,   789,  5092, 15713, 12632,  6516, 16107,  2314,
	15385, 17281,   383,  5515,  5019, 13218,  3293,  4728,  9704, 14263,
	 4417,   218, 16011,  2568,  5635,  8516, 18352, 12887, 13102, 15257,
	14316, 12813,  8886, 11051, 13356, 15353, 12059,  6880, 17926, 11710,
	 8052,  1737, 16384, 18094,  3410, 14787, 13788, 14210,  2656, 17550,
	 7950,  4311, 18150,  4973, 11548,  7848, 15326, 15517,    97, 11648,
	17990, 17685, 10847, 14695,   282,  1558,  6535, 10743,  2399,   181,
	10165,  8051, 12204, 18401, 13377,  7233, 10892, 15728, 15002, 11766,
	 8462, 15245, 12420,  8613,  5053, 12360, 17415, 12678,  4606,   870,
	 8429,  9572,  6542,  1892,  4008, 17045,   371, 10155,   819, 15114,
	 1522, 13638, 16576, 17586, 16840,  7671, 13873, 12065,  7433,  7599,
	 2497,  5298, 16406,  3443,  9437,  6905, 14589, 17851,   209, 17496,
	 1698,  7028,  4444,  8211,  9159, 16089, 16741,  9085,  2658,  4488,
	 8650,  3995,  6532, 11903,   508,   192,  4039, 17347, 12742,  6993,
	14812, 17645,  4527,   695,  8380, 16230,  2153,  3136,  2133,  4725,
	 9230, 13213,  6548, 18005,  6108, 16097, 16952, 13519, 16207, 12802,
	16047,  7081, 12818,  8328, 17091,  8927,  9558,  9273,  9301, 10337,
	11142,  5082, 11846, 15508, 17108,  8498, 16135,  3776,  6752, 12857,
	 3590,   486,  3056,  4203, 10364, 17125, 14532,  3025, 18386, 12029,
	 1983,  7426, 13553,   623,  6269, 15287, 16399, 12294,  6987,  8011,
	 1378, 14019,  3042,  3472,  4734, 12820, 16363,  7781,  3644, 16472,
	 3523, 14104, 11521, 18288, 13956,  4549,  6314, 16320, 16373, 16203,
	15299,  7524,  9080, 15914, 17765, 12520,  5829, 13379,  9482,  7938,
	  760, 13350,  9526, 15502, 16160,  6398,  7067,  1655,  3428,  7827,
	10564,  1235, 10800,  8291, 15614, 14755,  8732,  3010, 12821,  7168,
	 8131,  1912,  8093, 10461, 12301, 11616, 13947,  8029, 15138,  8334,
	13345, 13462,  7201, 11285,  8232,  5869,  5652,  8087,  9232,   151,
	 5425, 15984,  9061, 10972,   874,  6136,  9299,  4966, 10442,  5398,
	 6605, 14398,  7625,  7091, 10974, 14743,  6836, 17243,   504,  7883,
	10503, 12533,  3111, 13658,  1303,  6153, 12791,   335, 16064,  6652,
	14432, 10970,   558,  5436,   300, 13031, 12835,  7899,  8435,  7252,
	 2970, 12879,  2786, 16438, 16584,  2204,  5974,  6467,  7971, 14624,
	 9637,  9448, 18144,  7293, 18121, 10042,  1398, 12430,   157,  6881,
	18084, 12060,  5783,   444, 14853,  7936, 13337, 10411,  4401, 12549,
	17699,  1174,  1162,  5374,  3796,   709,  1424,  8521,  2238,   991,
	 9114, 15056,  4900, 16221,   731, 18419, 14885,  1707, 11644,  7594,
	14783,  4281, 12810,  5277, 10528, 15155, 16633, 13979,  5573,  7912,
	15085,  4250, 13224, 11094,  1717, 11970,  4689, 11787,   613, 14891,
	 6892,  1734, 15910, 17044,  1022, 16497,  7473,  4421, 17061,  2094,
	17491, 14013,  1872, 13480, 10045, 17585,  1562, 10460, 12143, 11266,
	 2168, 15769,  3047,  7683, 14260,  9889, 14090, 14179,  4404, 11389,
	11461,  4622, 18349, 14047,  7466, 13272,  5005, 12487,   615,  1829,
	13229, 15305,  3467, 11180,  2855,  8191,  3868,  9735, 18383, 13189,
	  933,  7900, 18340, 17527,  4316, 14694,  5680,  9549, 15669,  5777,
	17938,  7070, 11080,  4478,  1466, 10714, 15409,  8001,  7888,  3707,
	14283,  7140,  3046, 14214, 15419, 16423, 18200, 10217, 10615, 18381,
	13138,  1337,  8483,  7125,  6741, 10966, 18359,  4036, 11656,  2954,
	 5907,  1652, 12095, 11393, 12093,  6022, 11472,  6513, 15239, 12291,
	16914,  3635,  2907,   373, 12897,  8503, 16298,  8337, 10348, 11023,
	 8932,  5553, 12984, 11729,  9882, 13024,   556,    65, 10270,  4317,
	14404,  9508,  9191,  9860, 14257, 11049, 13040, 14653, 13038,  9282,
	10349,  4492,  6555,  9154,  8558, 14991,  4583,  3619,   379, 13206,
	11105,  7100, 15820, 14978,  7277, 12620,  3196, 11513,  7268, 16100,
	   46, 12935, 10191,  4142,  9281, 11926, 14900, 14340, 16894,  5224,
	 9309, 13388, 13942,  3818,  2937,  7206, 14135, 15212,  7925,  1689,
	 8800,  1294,  5524, 14570, 16368, 11992,  9491,  4458,  3910, 11928,
	13598,  1656,  3586,  8177, 13056,  2322, 16649,  1648, 14699, 18328,
	 1843,   116, 10016,  4221,  3330,  2710,  5358, 11169, 13567,  1354,
	 8715,  3439,  8805,  5505, 10680, 17825, 14534,  8396,  4185,  3904,
	 8543,  2358, 13314, 13160, 14784, 16183,  3842, 13644, 17524,  1253,
	13782, 16530, 12687, 15971, 13700, 17515,  2420, 10494,  7049,  8615,
	15561, 10671, 10485,  1060,  1583,  2340,  6599, 16718,  5525,  8039,
	12196, 15350, 10577,  8497, 16786, 10118, 13406,  2164,   696,  7375,
	 3971,   630, 13829,  4501, 10704,  8545,  8124, 10763,  4718,  6718,
	13636, 11540, 16886,  2173, 13510,  4961,  9652,  3648,  3009, 16232,
	 2469,  3836,  4933,  3461, 12281, 13205, 11756, 13442,  4041,  4285,
	 3661, 16043,  9473, 11418, 13814, 10301,  5454, 10915,  8727, 17232,
	13005,  3609,  9965,  5508,  3913, 10768, 11368,  3716, 15705, 10290,
	10822, 12073,  8935,  4393,  3878, 18157, 11691, 13998, 11637, 16445,
	17690,  4654, 12911,  9234,  2765,  6125, 12586, 12014, 18046,  2176,
	17540,  7355,   811, 12063, 17878, 11837,  8513, 13958, 16653, 12390,
	 3722,  4745,  7749,  8299,  2499, 10669, 16214,  3951, 15969,   375,
	13937, 18040, 11638,  9914, 16136, 15678,  7102, 12699,  9368, 15152,
	16159, 12929, 14186, 13925,  6623,  7438,  5741, 16684,   153, 14572,
	14261,  3358, 14440, 14021, 15097, 18043, 12112, 10964,  5242, 13012,
	 9833,  1249, 16386,  5032,  2437, 10065,  1738,  3850,    11,  1891,
	 3970,  7161,  7025, 17895,  6303, 14429, 12523, 17941,  6931,  5087,
	11127, 10882, 13926, 16149,  7788, 11652,  8944,   913, 15223,  6189,
	 9511,  2869, 10910,  8768,  6262,  5705, 16606,  5986, 10784,  2189,
	14068, 10397, 14897, 15500, 15844,  5698,  5743,  3622,   853, 14256,
	 9576,  2313, 15227, 16931,  3810,  1440,  6324,  6309,  3400,  6365,
	10288, 15790, 16146,  5667, 10602, 11119,  5700,  7960,  4236,  2617,
	12801,  8757,  1131,  5072, 16068, 17394,  1735,  5010,  2908, 12275,
	 3985,  1361, 17206, 13615, 12942,  9536, 12505,  6468,  8129, 14974,
	 2983,  1708, 11802,  7944, 17712,  8436,  5712,  3320, 13774, 13479,
	 9887, 17235,  4487,  3873,  3645,  9941, 16825, 13471,  8623, 14435,
	 5656,   396,  7269,  9569,   935, 13271, 13889, 18167,  6320, 14000,
	   40, 15255,  4382,  7607,  3761,  8098, 15702, 11450,  2666,  7539,
	13722,  2864, 10120,  7018, 11627,  8023, 14190,  6234, 15359,  2757,
	11647,  6434,  1917, 14513,  7362, 10475,   985,    82, 12956, 10267,
	10798,  2920,   535,  8185, 17135, 16491,  6525,  2321, 11245, 14410,
	 9521, 11291,  4326,  4683,  2594, 16946, 12878,  3561,  9648, 11339,
	 9944, 13628, 14996, 14086, 16837,  8831, 12823, 12539,  2930, 16057,
	11685, 16318, 11722, 14300, 10574,  9657, 17379,  8165, 18193,   635,
	17483, 10962, 17727,  2636, 16666,  1219,  8272,  2691, 15755, 15534,
	 2783, 17598,  9028,  5299,  7757, 11350,  9421,   803, 16276,  4555,
	 2408, 15134, 13315,  6629,  2575, 12004, 16466, 17109, 14006,  9793,
	17355, 17445,  9993,  6970, 13713,  6344, 17481,  5591, 17027,  2952,
	  268,   827,  1635, 12955,  8609, 13704,  8571,  3820, 15205, 13149,
	13046, 12333,  8005, 13766, 18367,  7087,  5414, 14093, 14734, 10939,
	12282,  6674,  3811, 13342
};
ALIGN32
const uint16_t mq2_iGM[] = {
	 2282,  9878, 10329, 12352, 15894,  7491,  4364,  3866, 15622,   627,
	16687,  6901,  2651,  5094, 13332, 12233,   576,  1524, 17276,  1163,
	15175, 12117,  1360, 15887,  8822, 13357, 11401,  9044, 13464,  7974,
	 7517,  6448,  9386, 10241,  8348, 14407, 12578,  9470, 14993,  3187,
	 1540, 11755,  3691, 13990,  2810, 11275,  1588, 11882,  2773,  9257,
	14175,  6399, 17149,  1211, 18324,  7008,  2085, 13581, 16069,  7570,
	11824,  6707,  6459,  9025,  3184,  2280,  5381, 10013,  9640, 10145,
	11614, 17672, 10876,  8807,  4139,  9031,   694, 16429, 17487, 15162,
	13647,  5002, 17998, 16130, 12094,   509, 12253,  6690,  4910, 12223,
	 1594, 14202, 12550, 10932, 10569, 12987,  5600,  2528,    16, 12331,
	 5191,  4134,  9126,  8017,  3845,  5949, 17654, 18292,  1869,  3793,
	  374,  9438, 12609,  9168,  1458, 10770, 14509, 12659,  6730,  9358,
	 7061, 14458,  9658, 17105, 11328, 11539,  1823, 16728, 15234, 10353,
	10682, 13670, 11758, 18053, 14464, 13692,  2527,  6302, 12173,   334,
	10476, 13893, 14671,  1567,  1115,  1030, 10273, 15276,  6942, 11455,
	 9289,  3456, 11381,  7455, 10197, 16611,  5326,  1035, 12023, 16066,
	16697, 16912,  2207, 17744,  5211,  5723, 12286,  1017,  1573,  6082,
	 8905,  2440, 14720,  8225,  3202,  9240,  7704, 11167,   654, 13251,
	 7115, 16905, 18190, 16638,  2788, 15057, 16545,  1149, 14184,  9879,
	10679, 12510, 15892, 12862,  4048,  4566,  4580, 13654,  4753,   671,
	14269, 12024,  5676,  1193, 12032,  1113,  2457,  9957,  1168, 15379,
	  214, 15159,  2610, 13818,  6854,  8150, 16865,  8140, 10318, 14243,
	 8869,  6953,   394, 11027,  5720, 12062,   543,  7197, 18337, 18179,
	 3265, 15167,  7219, 14108, 16189, 17104,  4674,   846,  1172,  4637,
	 5111, 16211, 14919, 17584,  9685,  9112,   291,  1922,  5764,  4498,
	 7495, 10230, 15784,  7968,  5417,  5500,  6440, 13967,  3705, 13259,
	 5048, 10284,  4965,  2768,  9030,  7763,  7399,  9976,  3071,  1597,
	 5960, 12697, 15422,  3170,  3520,  3169, 17607,  6263, 16956, 12605,
	16415,    37, 12950,  5846,  5654,  4975,  8548, 11864,    26,  3909,
	 4108,  9333,  1005,  1507, 11326, 16910, 14863,  2075,  7363, 14489,
	 5216,  1512, 13076, 17700, 16194, 12893, 14898,  9464,  6328,  1382,
	 4442, 15593, 11086, 16275,   453,  9263, 14483,  8750,  2622,    25,
	 4349, 16499,  5121,  7789, 12843,  7483,  1564,  2602,  8302,  8909,
	 2973,  6714, 11797, 14700,  2193,    42, 16122,  3486,  3522, 16231,
	 2127, 11388,  4272, 11303,  5375,  7693,  1332, 17349, 12800,  3145,
	13203, 17652,   424,  4194, 11693, 17497,  2210,   471, 17386,   686,
	 7006,  5480,   968, 17922,  9911, 10478, 17566, 14987,  1771,  8910,
	 3323,  6872, 12448,  8358, 12886, 11821, 16308,  1674, 14477,  6430,
	 2227,   900,  1639, 13169,  6578, 12028,  7076,  1825, 14636, 12611,
	 8363,  1774,     7,  8851,  1106, 15983, 10905, 13876,  8721, 17314,
	 4956, 17721,  8862, 16535, 15746, 17852, 17846,   367,  2942,  7016,
	 4011,  2548, 14465,  1790, 18211,  6325, 12403,  9391,  5776,  9138,
	12218, 17734, 13412,   156,  5570,  9361, 13709,  4398, 11121,  5231,
	 5983, 15446, 17331, 10141, 10214, 17040,  2777, 16948, 14807,  4999,
	 5267,  2799,  2701, 18283, 15715, 18154, 12948, 11217, 15107, 10401,
	 9049,  2821,  6140,  8565, 11604,  7661,  2950,  3965,  5275, 18181,
	  595, 15015,  1845, 12946,  5671,  5404, 11234,  5914, 15734, 13212,
	15950,  4567, 15365, 17996, 12947,  4686, 10441,  6504,  9141, 13817,
	 5173, 15607,  6282, 14317,  3574,  5616, 11702,  2544, 14266, 10864,
	 5202,  2243, 12625,  3066,  3986,  5170, 17477,  5151, 14849,  2806,
	16928, 14067,  1839, 10626,  5071, 13033,  8599, 13151,  5303, 16719,
	 8389,  5683, 11762,  7311, 15096, 12292,  3747, 11066,  2170, 15726,
	 5673,    33, 11550,  5214,  3050, 11910,  2642,  1614, 16523,  4931,
	11581,  4912,  2739,  8399,  8803, 18299, 16957,   703,  6421,   476,
	15261,  2360, 14948,  4220,   494,   539,  4320, 11430,   662, 10200,
	12431,  7929,  5902,  2559, 10866, 17229,  6939, 10295,  8815,  4506,
	12758,  5338,  6567, 13919,  9634,  7825, 10666,  1339,  7871, 14297,
	 8607, 10100, 17115,   353, 12952,   475,  8899,   120,  5134,   527,
	 4388, 13146, 11283, 12572, 10274,  3374,  1188, 16968,  2947,  2805,
	 4801,   798, 11390, 10935, 11619, 13461,  3547, 13609,  7436, 11994,
	 9960, 17136,  6875, 16270,  3571,  4456, 11228,  3594,  8056,  5954,
	  971,   649,  5124,  8949, 16973, 13034,  4083, 11955, 18392,  8724,
	 3979, 14752,  1960,  8258, 15216,  3393,  7838,  1537, 15316, 11338,
	 5205,  3403, 14924, 13373, 17001, 11572,  5447, 17100, 12708, 10582,
	14384,  7336,  5413, 16242,  1589, 18413, 11433, 15273,   133,  2272,
	 2581,  8749,  4432,  5582, 18235, 15605,  1999,  4905,  2481,   804,
	 4246,  7394,  7280,  6973,   599,  4273,  2477, 11546, 16773, 15577,
	14215,  9577, 14461, 12532, 17579,  7725, 10946,  5152, 15199,  2964,
	13665, 11962,  2409,  9830,  8536,  7224,  3079, 16979, 15928,  8349,
	 9736, 10399, 15897,  8651,  4838,  2816,  7908, 16315, 14453, 15583,
	 3657, 13132,  6383, 10360, 10538, 13289,  6034, 16733,  6062, 15271,
	17713, 16528,   751,  1603,  8060, 13645, 11305,  8790, 16622,  6345,
	15584, 10511, 10683,  1768,  4018, 11399,  8122, 13041, 15440, 10130,
	 6364, 15302, 14049, 12978,  7782,  4461,  6122,  1605,  8760, 13961,
	12607, 14539,  1142, 11470, 12992,  3653,  6673,  5751,   246,  2955,
	 2002,  6065,   269,  5704,  5636, 16448,  8271,  9211, 16508, 17564,
	 4184,  7998, 15917, 10240,  8592,  4300, 11927, 15812, 12951, 12377,
	  195,  1668,  2206, 11213, 16754,  2086, 11147,  9140, 10091,  6346,
	14714,  5905,  2254, 11340,  2752,  1137, 10857, 13749,  2867, 14882,
	10594, 10365, 13476, 12614,  9413,  2248,  9029,  1232,  7241, 10326,
	 3882,  7967,  5067,  5342,  6844, 16572, 12238,   890, 11454,  4960,
	 3298,  9494,  3185,  8811,  5539,  9663, 17345,  9410, 12426, 12140,
	 6154,  7834, 13816,  2761, 16106,  9588,   994,  3398, 11434,  3371,
	  138, 16494,  7020,  4749,  3180, 13022, 13288,  1364, 16575, 12749,
	13049,  7260, 15679,  4234,  7412,  2714,  9817,  4853,  3759, 15706,
	 4066, 11526, 12724,  4480,  1195,  7386,  7074,  7196, 11712, 12555,
	 2614,  3076,  7486,  6750, 16515,  7982, 10317,  7712, 16609, 13607,
	 6736, 11678,  8130,  9990, 12663, 11615, 15074, 16074,  3835, 14371,
	 4944, 13081,  6966,  2302, 18118,  7231,  5529, 18085, 17351, 11730,
	13374, 10040,  4968,  3928, 10758, 12335,  5197,  6454, 10074,  5917,
	17263,  8425, 17903,  3974,  3881,  1436,  4909,  5692, 13186, 17223,
	  459, 11583,  1231,  2873, 10168, 11542,  8590,  9671, 11611, 16512,
	 1125, 11041, 11853, 11776, 17254,  4945, 16481,  7124, 14235, 11166,
	  304, 13093,  6464,  4814,  7497,  4859, 17756,  2433,  3632, 15754,
	17078, 16768,  7106, 13425, 18375,  8295,  9269,  1867, 17609,   892,
	17272, 11905,  5128, 16640, 17605, 11634, 12469, 16478, 16204,  4471,
	12437, 10249, 11148, 15671, 17786, 14033,  8372,  5254, 10827,  2149,
	14830,  7748, 16524, 11462, 11739,  4562, 15821,  9986, 11263, 10983,
	12470,  4576, 16362,  4121,  2749, 18410, 10383, 14799,  3460, 16835,
	12123,  5578, 10944, 10523, 14883,  3664, 11830,  9027,  7407,  6925,
	 1721, 14154, 13856,  5939, 16187,  4042, 13792, 11914,  1890, 11913,
	 3692,  2088, 13503,  4621, 13679, 11231,  7058, 13298,  9184, 18155,
	11921, 13492,  3352, 11941
};
