/*
 * Big integer support code.
 */

#include "kgen_inner.h"

/* see kgen_inner.h */
uint32_t
zint_mul_small(uint32_t *m, size_t len, uint32_t x)
{
	uint32_t cc = 0;
	for (size_t i = 0; i < len; i ++) {
		uint64_t z = (uint64_t)m[i] * (uint64_t)x + cc;
		m[i] = (uint32_t)z & 0x7FFFFFFF;
		cc = (uint32_t)(z >> 31);
	}
	return cc;
}

/* see kgen_inner.h */
uint32_t
zint_mod_small_unsigned(const uint32_t *d, size_t len, size_t stride,
	uint32_t p, uint32_t p0i, uint32_t R2)
{
	/*
	 * Algorithm: we inject words one by one, starting with the high
	 * word. Each step is:
	 *  - multiply x by 2^31
	 *  - add new word
	 */
	uint32_t x = 0;
	uint32_t z = mp_half(R2, p);
	d += len * stride;
	for (uint32_t i = len; i > 0; i --) {
		d -= stride;
		uint32_t w = *d - p;
		w += p & tbmask(w);
		x = mp_mmul(x, z, p, p0i);
		x = mp_add(x, w, p);
	}
	return x;
}

#if FNDSA_AVX2
TARGET_AVX2
__m256i
avx2_zint_mod_small_unsigned_x8(const uint32_t *d, size_t len, size_t stride,
	__m256i yp, __m256i yp0i, __m256i yR2)
{
	__m256i yx = _mm256_setzero_si256();
	__m256i yz = mp_half_x8(yR2, yp);
	d += len * stride;
	for (size_t i = len; i > 0; i --) {
		d -= stride;
		__m256i yw = _mm256_sub_epi32(
			_mm256_loadu_si256((__m256i *)d), yp);
		yw = _mm256_add_epi32(yw,
			_mm256_and_si256(yp, _mm256_srai_epi32(yw, 31)));
		yx = mp_mmul_x8(yx, yz, yp, yp0i);
		yx = mp_add_x8(yx, yw, yp);
	}
	return yx;
}
#endif

/* see kgen_inner.h */
void
zint_add_mul_small(uint32_t *restrict x, size_t len, size_t xstride,
	const uint32_t *restrict y, uint32_t s)
{
	uint32_t cc = 0;
	for (size_t i = 0; i < len; i ++) {
		uint32_t xw, yw;
		uint64_t z;

		xw = *x;
		yw = y[i];
		z = (uint64_t)yw * (uint64_t)s + (uint64_t)xw + (uint64_t)cc;
		*x = (uint32_t)z & 0x7FFFFFFF;
		cc = (uint32_t)(z >> 31);
		x += xstride;
	}
	*x = cc;
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_zint_add_mul_small_x8(uint32_t *restrict d, size_t len, size_t dstride,
	const uint32_t *restrict a, __m256i ys)
{
	__m256i cc0 = _mm256_setzero_si256();
	__m256i cc1 = _mm256_setzero_si256();
	__m256i ys0 = ys;
	__m256i ys1 = _mm256_srli_epi64(ys, 32);
	__m256i yw32 = _mm256_set1_epi64x(0xFFFFFFFF);
	__m256i ym31 = _mm256_set1_epi32(0x7FFFFFFF);
	for (size_t i = 0; i < len; i ++) {
		__m256i ya = _mm256_set1_epi64x(a[i]);
		__m256i z0 = _mm256_mul_epu32(ya, ys0);
		__m256i z1 = _mm256_mul_epu32(ya, ys1);
		__m256i yd = _mm256_loadu_si256((__m256i *)d);
		__m256i yd0 = _mm256_and_si256(yd, yw32);
		__m256i yd1 = _mm256_srli_epi64(yd, 32);
		z0 = _mm256_add_epi64(z0, _mm256_add_epi64(yd0, cc0));
		z1 = _mm256_add_epi64(z1, _mm256_add_epi64(yd1, cc1));
		cc0 = _mm256_srli_epi64(z0, 31);
		cc1 = _mm256_srli_epi64(z1, 31);
		yd = _mm256_blend_epi32(z0, _mm256_slli_epi64(z1, 32), 0xAA);
		_mm256_storeu_si256((__m256i *)d, _mm256_and_si256(yd, ym31));
		d += dstride;
	}

	_mm256_storeu_si256((__m256i *)d,
		_mm256_and_si256(_mm256_blend_epi32(
			cc0, _mm256_slli_epi64(cc1, 32), 0xAA), ym31));
}
#endif

/* see kgen_inner.h */
void
zint_norm_zero(uint32_t *restrict x, size_t len, size_t xstride,
	const uint32_t *restrict m)
{
	/*
	 * Compare x with m/2. We use the shifted version of m, and m
	 * is odd, so we really compare with (m-1)/2; we want to perform
	 * the subtraction if and only if x > (m-1)/2.
	 */
	uint32_t r = 0;
	uint32_t bb = 0;
	x += len * xstride;
	size_t i = len;
	while (i -- > 0) {
		x -= xstride;
		/*
		 * Get the two words to compare in wx and wp (both over
		 * 31 bits exactly).
		 */
		uint32_t wx = *x;
		uint32_t wp = (m[i] >> 1) | (bb << 30);
		bb = m[i] & 1;

		/*
		 * We set cc to -1, 0 or 1, depending on whether wp is
		 * lower than, equal to, or greater than wx.
		 */
		uint32_t cc = wp - wx;
		cc = ((-cc) >> 31) | -(cc >> 31);

		/*
		 * If r != 0 then it is either 1 or -1, and we keep its
		 * value. Otherwise, if r = 0, then we replace it with cc.
		 */
		r |= cc & ((r & 1) - 1);
	}

	/*
	 * At this point, r = -1, 0 or 1, depending on whether (m-1)/2
	 * is lower than, equal to, or greater than x. We thus want to
	 * do the subtraction only if r = -1.
	 */
	uint32_t cc = 0;
	uint32_t s = tbmask(r);
	for (size_t j = 0; j < len; j ++) {
		uint32_t xw = *x;
		uint32_t w = xw - m[j] - cc;
		cc = w >> 31;
		xw ^= ((w & 0x7FFFFFFF) ^ xw) & s;
		*x = xw;
		x += xstride;
	}
}

#if FNDSA_AVX2
TARGET_AVX2
static void
zint_norm_zero_x8(uint32_t *restrict x, size_t len, size_t xstride,
	const uint32_t *restrict m)
{
	/*
	 * Compare x with m/2. We use the shifted version of m, and m
	 * is odd, so we really compare with (m-1)/2; we want to perform
	 * the subtraction if and only if x > (m-1)/2.
	 */
	__m256i yr = _mm256_setzero_si256();
	__m256i yone = _mm256_set1_epi32(1);
	uint32_t bb = 0;
	x += len * xstride;
	size_t i = len;
	while (i -- > 0) {
		x -= xstride;
		/*
		 * Get the two words to compare in wx and wp (both over
		 * 31 bits exactly).
		 */
		__m256i yx = _mm256_loadu_si256((__m256i *)x);
		uint32_t wp = (m[i] >> 1) | (bb << 30);
		bb = m[i] & 1;

		/*
		 * We set cc to -1, 0 or 1, depending on whether wp is
		 * lower than, equal to, or greater than wx.
		 */
		__m256i ycc = _mm256_sub_epi32(_mm256_set1_epi32(wp), yx);
		ycc = _mm256_or_si256(
			_mm256_srli_epi32(_mm256_sub_epi32(
				_mm256_setzero_si256(), ycc), 31),
			_mm256_srai_epi32(ycc, 31));

		/*
		 * If r != 0 then it is either 1 or -1, and we keep its
		 * value. Otherwise, if r = 0, then we replace it with cc.
		 */
		yr = _mm256_or_si256(yr, _mm256_and_si256(ycc,
			_mm256_sub_epi32(_mm256_and_si256(yr, yone), yone)));
	}

	/*
	 * At this point, r = -1, 0 or 1, depending on whether (m-1)/2
	 * is lower than, equal to, or greater than x. We thus want to
	 * do the subtraction only if r = -1.
	 */
	__m256i ycc = _mm256_setzero_si256();
	__m256i ym = _mm256_srai_epi32(yr, 31);
	__m256i y31 = _mm256_set1_epi32(0x7FFFFFFF);
	for (size_t j = 0; j < len; j ++) {
		__m256i yx = _mm256_loadu_si256((__m256i *)x);
		__m256i y = _mm256_sub_epi32(
			_mm256_sub_epi32(yx, ycc),
			_mm256_set1_epi32(m[j]));
		ycc = _mm256_srli_epi32(y, 31);
		yx = _mm256_or_si256(
			_mm256_andnot_si256(ym, yx),
			_mm256_and_si256(ym, _mm256_and_si256(y, y31)));
		_mm256_storeu_si256((__m256i *)x, yx);
		x += xstride;
	}
}
#endif

/* see kgen_inner.h */
void
zint_rebuild_CRT(uint32_t *restrict xx, size_t xlen, size_t n,
	size_t num_sets, int normalize_signed, uint32_t *restrict tmp)
{
	size_t uu = 0;
	tmp[0] = PRIMES[0].p;
	for (size_t i = 1; i < xlen; i ++) {
		/*
		 * At the entry of each loop iteration:
		 *  - the first i words of each array have been
		 *    reassembled;
		 *  - the first i words of tmp[] contains the
		 * product of the prime moduli processed so far.
		 *
		 * We call 'q' the product of all previous primes.
		 */
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t s = PRIMES[i].s;
		uu += n;
		size_t kk = 0;
		for (size_t k = 0; k < num_sets; k ++) {
			for (size_t j = 0; j < n; j ++) {
				/*
				 * xp = the integer x modulo the prime p for
				 *      this iteration
				 * xq = (x mod q) mod p
				 */
				uint32_t xp = xx[kk + j + uu];
				uint32_t xq = zint_mod_small_unsigned(
					xx + kk + j, i, n, p, p0i, R2);

				/*
				 * New value is:
				 *   (x mod q) + q*(s*(xp - xq) mod p)
				 */
				uint32_t xr = mp_mmul(
					s, mp_sub(xp, xq, p), p, p0i);
				zint_add_mul_small(xx + kk + j, i, n, tmp, xr);
			}
			kk += n * xlen;
		}

		/*
		 * Update product of primes in tmp[].
		 */
		tmp[i] = zint_mul_small(tmp, i, p);
	}

	/*
	 * Normalize the reconstructed values around 0.
	 */
	if (normalize_signed) {
		size_t kk = 0;
		for (size_t k = 0; k < num_sets; k ++) {
			for (size_t j = 0; j < n; j ++) {
				zint_norm_zero(xx + kk + j, xlen, n, tmp);
			}
			kk += n * xlen;
		}
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_zint_rebuild_CRT(uint32_t *restrict xx, size_t xlen, size_t n,
	size_t num_sets, int normalize_signed, uint32_t *restrict tmp)
{
	size_t uu = 0;
	tmp[0] = PRIMES[0].p;
	for (size_t i = 1; i < xlen; i ++) {
		/*
		 * At the entry of each loop iteration:
		 *  - the first i words of each array have been
		 *    reassembled;
		 *  - the first i words of tmp[] contains the
		 * product of the prime moduli processed so far.
		 *
		 * We call 'q' the product of all previous primes.
		 */
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t s = PRIMES[i].s;
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yp0i = _mm256_set1_epi32(p0i);
		__m256i yR2 = _mm256_set1_epi32(R2);
		__m256i ys = _mm256_set1_epi32(s);
		uu += n;
		size_t kk = 0;
		for (size_t k = 0; k < num_sets; k ++) {
			size_t j = 0;
			for (; (j + 7) < n; j += 8) {
				__m256i y1 = _mm256_loadu_si256(
					(__m256i *)(xx + kk + uu + j));
				__m256i y2 = avx2_zint_mod_small_unsigned_x8(
					xx + kk + j, i, n, yp, yp0i, yR2);
				__m256i yr = mp_mmul_x8(ys,
					mp_sub_x8(y1, y2, yp), yp, yp0i);
				avx2_zint_add_mul_small_x8(
					xx + kk + j, i, n, tmp, yr);
			}
			for (; j < n; j ++) {
				/*
				 * xp = the integer x modulo the prime p for
				 *      this iteration
				 * xq = (x mod q) mod p
				 */
				uint32_t xp = xx[kk + j + uu];
				uint32_t xq = zint_mod_small_unsigned(
					xx + kk + j, i, n, p, p0i, R2);

				/*
				 * New value is:
				 *   (x mod q) + q*(s*(xp - xq) mod p)
				 */
				uint32_t xr = mp_mmul(
					s, mp_sub(xp, xq, p), p, p0i);
				zint_add_mul_small(xx + kk + j, i, n, tmp, xr);
			}
			kk += n * xlen;
		}

		/*
		 * Update product of primes in tmp[].
		 */
		tmp[i] = zint_mul_small(tmp, i, p);
	}

	/*
	 * Normalize the reconstructed values around 0.
	 */
	if (normalize_signed) {
		size_t kk = 0;
		for (size_t k = 0; k < num_sets; k ++) {
			size_t j = 0;
			for (; (j + 7) < n; j += 8) {
				zint_norm_zero_x8(xx + kk + j, xlen, n, tmp);
			}
			for (; j < n; j ++) {
				zint_norm_zero(xx + kk + j, xlen, n, tmp);
			}
			kk += n * xlen;
		}
	}
}
#endif

/* see kgen_inner.h */
void
zint_negate(uint32_t *a, size_t len, uint32_t ctl)
{
	/*
	 * If ctl = 0xFFFFFFFF then we flip the bits of a by XORing with
	 * 0x7FFFFFFF, and we add 1 to the value. If ctl = 0 then we XOR
	 * with 0 and add 0, which leaves the value unchanged.
	 */
	uint32_t cc = -ctl;
	uint32_t m = ctl >> 1;
	for (size_t i = 0; i < len; i ++) {
		uint32_t aw;

		aw = a[i];
		aw = (aw ^ m) + cc;
		a[i] = aw & 0x7FFFFFFF;
		cc = aw >> 31;
	}
}

/*
 * Replace a with (a*xa+b*xb)/(2^31) and b with (a*ya+b*yb)/(2^31).
 * The low bits are dropped (the caller should compute the coefficients
 * such that these dropped bits are all zeros). If either or both
 * yields a negative value, then the value is negated.
 *
 * Returned value is:
 *  0  both values were positive
 *  1  new a had to be negated
 *  2  new b had to be negated
 *  3  both new a and new b had to be negated
 *
 * Coefficients xa, xb, ya and yb may use the full signed 32-bit range.
 * Integers a and b use stride 1.
 */
static uint32_t
zint_co_reduce(uint32_t *restrict a, uint32_t *restrict b, size_t len,
	int64_t xa, int64_t xb, int64_t ya, int64_t yb)
{
	int64_t cca = 0;
	int64_t ccb = 0;
	for (size_t i = 0; i < len; i ++) {
		uint32_t wa, wb;
		uint64_t za, zb;

		wa = a[i];
		wb = b[i];
		za = wa * (uint64_t)xa + wb * (uint64_t)xb + (uint64_t)cca;
		zb = wa * (uint64_t)ya + wb * (uint64_t)yb + (uint64_t)ccb;
		if (i > 0) {
			a[i - 1] = (uint32_t)za & 0x7FFFFFFF;
			b[i - 1] = (uint32_t)zb & 0x7FFFFFFF;
		}
		cca = *(int64_t *)&za >> 31;
		ccb = *(int64_t *)&zb >> 31;
	}
	a[len - 1] = (uint32_t)cca & 0x7FFFFFFF;
	b[len - 1] = (uint32_t)ccb & 0x7FFFFFFF;

	uint32_t nega = (uint32_t)(cca >> 63);
	uint32_t negb = (uint32_t)(ccb >> 63);
	zint_negate(a, len, nega);
	zint_negate(b, len, negb);
	return -nega | (-negb << 1);
}

/*
 * Finish modular reduction. Rules on input parameters:
 *
 *   if neg = 1, then -m <= a < 0
 *   if neg = 0, then 0 <= a < 2*m
 *
 * If neg = 0, then the top word of a[] is allowed to use 32 bits.
 *
 * Modulus m must be odd. Integers a and m have the same length len,
 * and both use stride 1.
 */
static void
zint_finish_mod(uint32_t *restrict a, const uint32_t *restrict m,
	size_t len, uint32_t neg)
{
	/*
	 * First pass: compare a (assumed nonnegative) with m. Note that
	 * if the top word uses 32 bits, subtracting m must yield a
	 * value less than 2^31 since a < 2*m.
	 */
	uint32_t cc = 0;
	for (size_t i = 0; i < len; i ++) {
		cc = (a[i] - m[i] - cc) >> 31;
	}

	/*
	 * If neg = 1 then we must add m (regardless of cc)
	 * If neg = 0 and cc = 0 then we must subtract m
	 * If neg = 0 and cc = 1 then we must do nothing
	 *
	 * In the loop below, we conditionally subtract either m or -m
	 * from a. Word xm is a word of m (if neg = 0) or -m (if neg = 1);
	 * but if neg = 0 and cc = 1, then ym = 0 and it forces mw to 0.
	 */
	uint32_t xm = -neg >> 1;
	uint32_t ym = -(neg | (1 - cc));
	cc = neg;
	for (size_t i = 0; i < len; i ++) {
		uint32_t mw = (m[i] ^ xm) & ym;
		uint32_t aw = a[i] - mw - cc;
		a[i] = aw & 0x7FFFFFFF;
		cc = aw >> 31;
	}
}

/*
 * Replace a with (a*xa+b*xb)/(2^31) mod m, and b with
 * (a*ya+b*yb)/(2^31) mod m. Modulus m must be odd; m0i = -1/m[0] mod 2^31.
 * Integers a, b and m all have length len and stride 1.
 */
static void
zint_co_reduce_mod(uint32_t *restrict a, uint32_t *restrict b,
	const uint32_t *restrict m, size_t len,
	uint32_t m0i, int64_t xa, int64_t xb, int64_t ya, int64_t yb)
{
	/*
	 * These are actually four combined Montgomery multiplications.
	 */
	int64_t cca = 0;
	int64_t ccb = 0;
	uint32_t fa = ((a[0] * (uint32_t)xa + b[0] * (uint32_t)xb) * m0i)
		& 0x7FFFFFFF;
	uint32_t fb = ((a[0] * (uint32_t)ya + b[0] * (uint32_t)yb) * m0i)
		& 0x7FFFFFFF;
	for (size_t i = 0; i < len; i ++) {
		uint32_t wa = a[i];
		uint32_t wb = b[i];
		uint64_t za = wa * (uint64_t)xa + wb * (uint64_t)xb
			+ m[i] * (uint64_t)fa + (uint64_t)cca;
		uint64_t zb = wa * (uint64_t)ya + wb * (uint64_t)yb
			+ m[i] * (uint64_t)fb + (uint64_t)ccb;
		if (i > 0) {
			a[i - 1] = (uint32_t)za & 0x7FFFFFFF;
			b[i - 1] = (uint32_t)zb & 0x7FFFFFFF;
		}
		cca = *(int64_t *)&za >> 31;
		ccb = *(int64_t *)&zb >> 31;
	}
	a[len - 1] = (uint32_t)cca;
	b[len - 1] = (uint32_t)ccb;

	/*
	 * At this point:
	 *   -m <= a < 2*m
	 *   -m <= b < 2*m
	 * (this is a case of Montgomery reduction)
	 * The top words of 'a' and 'b' may have a 32-th bit set.
	 * We want to add or subtract the modulus, as required.
	 */
	zint_finish_mod(a, m, len, (uint32_t)((uint64_t)cca >> 63));
	zint_finish_mod(b, m, len, (uint32_t)((uint64_t)ccb >> 63));
}

/*
 * Given an odd x, compute -1/x mod 2^31.
 */
static inline uint32_t
mp_ninv31(uint32_t x)
{
	uint32_t y = 2 - x;
	y *= 2 - x * y;
	y *= 2 - x * y;
	y *= 2 - x * y;
	y *= 2 - x * y;
	return (-y) & 0x7FFFFFFF;
}

/* see kgen_inner.h */
int
zint_bezout(uint32_t *restrict u, uint32_t *restrict v,
	const uint32_t *restrict x, const uint32_t *restrict y,
	size_t len, uint32_t *restrict tmp)
{
	if (len == 0) {
		return 0;
	}

	/*
	 * Algorithm is basically the optimized binary GCD as described in:
	 *    https://eprint.iacr.org/2020/972
	 * The paper shows that with registers of size 2*k bits, one can
	 * do k-1 inner iterations and get a reduction by k-1 bits. In
	 * fact, it also works with registers of 2*k-1 bits (though not
	 * 2*k-2; the "upper half" of the approximation must have at
	 * least one extra bit). Here, we want to perform 31 inner
	 * iterations (since that maps well to Montgomery reduction with
	 * our 31-bit words) so we must use 63-bit approximations.
	 *
	 * We also slightly expand the original algorithm by maintaining
	 * four coefficients (u0, u1, v0 and v1) instead of the two
	 * coefficients (u, v), because we want a full Bezout relation,
	 * not just a modular inverse.
	 *
	 * We set up integers u0, v0, u1, v1, a and b. Throughout the
	 * algorithm, they maintain the following invariants:
	 *   a = x*u0 - y*v0
	 *   b = x*u1 - y*v1
	 *   0 <= a <= x
	 *   0 <= b <= y
	 *   0 <= u0 < y
	 *   0 <= v0 < x
	 *   0 <= u1 <= y
	 *   0 <= v1 < x
	 */
	uint32_t *u0 = tmp;
	uint32_t *v0 = u0 + len;
	uint32_t *u1 = u;
	uint32_t *v1 = v;
	uint32_t *a = v0 + len;
	uint32_t *b = a + len;

	/*
	 * We'll need the Montgomery reduction coefficients.
	 */
	uint32_t x0i = mp_ninv31(x[0]);
	uint32_t y0i = mp_ninv31(y[0]);

	/*
	 * Initial values:
	 *   a = x   u0 = 1   v0 = 0
	 *   b = y   u1 = y   v1 = x - 1
	 * Note that x is odd, so computing x-1 is easy.
	 */
	memcpy(a, x, len * sizeof *x);
	memcpy(b, y, len * sizeof *y);
	u0[0] = 1;
	memset(u0 + 1, 0, (len - 1) * sizeof *u0);
	memset(v0, 0, len * sizeof *v0);
	memcpy(u1, y, len * sizeof *u1);
	memcpy(v1, x, len * sizeof *v1);
	v1[0] --;

	/*
	 * Each input operand may be as large as 31*len bits, and we
	 * reduce the total length by at least 31 bits at each iteration.
	 */
	for (uint32_t num = 62 * (uint32_t)len + 31; num >= 31; num -= 31) {
		/*
		 * Extract the top 32 bits of a and b: if j is such that:
		 *   2^(j-1) <= max(a,b) < 2^j
		 * then we want:
		 *   xa = (2^31)*floor(a / 2^(j-32)) + (a mod 2^31)
		 *   xb = (2^31)*floor(a / 2^(j-32)) + (b mod 2^31)
		 * (if j < 63 then xa = a and xb = b).
		 */
		uint32_t c0 = 0xFFFFFFFF;
		uint32_t c1 = 0xFFFFFFFF;
		uint32_t cp = 0xFFFFFFFF;
		uint32_t a0 = 0;
		uint32_t a1 = 0;
		uint32_t b0 = 0;
		uint32_t b1 = 0;
		size_t j = len;
		while (j -- > 0) {
			uint32_t aw = a[j];
			uint32_t bw = b[j];
			a1 ^= c1 & (a1 ^ aw);
			a0 ^= c0 & (a0 ^ aw);
			b1 ^= c1 & (b1 ^ bw);
			b0 ^= c0 & (b0 ^ bw);
			cp = c0;
			c0 = c1;
			c1 &= (((aw | bw) + 0x7FFFFFFF) >> 31) - 1;
		}

		/*
		 * Possible situations:
		 *   cp = 0, c0 = 0, c1 = 0
		 *     j >= 63, top words of a and b are in a0:a1 and b0:b1
		 *     (a1 and b1 are highest, a1|b1 != 0)
		 *
		 *   cp = -1, c0 = 0, c1 = 0
		 *     32 <= j <= 62, a0:a1 and b0:b1 contain a and b, exactly
		 *
		 *   cp = -1, c0 = -1, c1 = 0
		 *     j <= 31, a0 and a1 both contain a, b0 and b1 contain b
		 *
		 * When j >= 63, we align the top words to ensure that we get
		 * the full 32 bits. We also take care to always call
		 * lzcnt() with a non-zero operand.
		 */
		unsigned s = lzcnt_nonzero(a1 | b1 | ((cp & c0) >> 1));
		uint32_t ha = (a1 << s) | (a0 >> (31 - s));
		uint32_t hb = (b1 << s) | (b0 >> (31 - s));

		/*
		 * If j <= 62, then we instead use the non-aligned bits.
		 */
		ha ^= (cp & (ha ^ a1));
		hb ^= (cp & (hb ^ b1));

		/*
		 * If j <= 31, then all of the above was bad, and we simply
		 * clear the upper bits.
		 */
		ha &= ~c0;
		hb &= ~c0;

		/*
		 * Assemble the approximate values xa and xb (63 bits each).
		 */
		uint64_t xa = ((uint64_t)ha << 31) | a[0];
		uint64_t xb = ((uint64_t)hb << 31) | b[0];

		/*
		 * Compute reduction factors:
		 *   a' = a*pa + b*pb
		 *   b' = a*qa + b*qb
		 * such that a' and b' are both multiples of 2^31, but are
		 * only marginally larger than a and b.
		 * Each coefficient is in the -(2^31-1)..+2^31 range. To keep
		 * them on 32-bit values, we compute pa+(2^31-1)... and so on.
		 */
		uint64_t fg0 = 1;
		uint64_t fg1 = (uint64_t)1 << 32;
		for (int i = 0; i < 31; i ++) {
			uint64_t a_odd = -(xa & 1);
			uint64_t dx = xa - xb;
			dx = (uint64_t)(*(int64_t *)&dx >> 63);
			uint64_t swap = a_odd & dx;
			uint64_t t1 = swap & (xa ^ xb);
			xa ^= t1;
			xb ^= t1;
			uint64_t t2 = swap & (fg0 ^ fg1);
			fg0 ^= t2;
			fg1 ^= t2;
			xa -= a_odd & xb;
			fg0 -= a_odd & fg1;
			xa >>= 1;
			fg1 <<= 1;
		}

		/*
		 * Split update factors.
		 */
		fg0 += (uint64_t)0x7FFFFFFF7FFFFFFF;
		fg1 += (uint64_t)0x7FFFFFFF7FFFFFFF;
		int64_t f0 = (int64_t)(fg0 & 0xFFFFFFFF) - 0x7FFFFFFF;
		int64_t g0 = (int64_t)(fg0 >> 32) - 0x7FFFFFFF;
		int64_t f1 = (int64_t)(fg1 & 0xFFFFFFFF) - 0x7FFFFFFF;
		int64_t g1 = (int64_t)(fg1 >> 32) - 0x7FFFFFFF;

		/*
		 * Apply the update factors.
		 */
		uint32_t negab = zint_co_reduce(a, b, len, f0, g0, f1, g1);
		f0 -= (f0 + f0) & -(int64_t)(negab & 1);
		g0 -= (g0 + g0) & -(int64_t)(negab & 1);
		f1 -= (f1 + f1) & -(int64_t)(negab >> 1);
		g1 -= (g1 + g1) & -(int64_t)(negab >> 1);
		zint_co_reduce_mod(u0, u1, y, len, y0i, f0, g0, f1, g1);
		zint_co_reduce_mod(v0, v1, x, len, x0i, f0, g0, f1, g1);
	}

	/*
	 * b contains GCD(x,y), provided that x and y were indeed odd.
	 * Result is correct if the GCD is 1.
	 */
	uint32_t r = b[0] ^ 1;
	for (size_t j = 1; j < len; j ++) {
		r |= b[j];
	}
	r |= (x[0] & y[0] & 1) ^ 1;
	return 1 - ((r | -r) >> 31);
}

/* see kgen_inner.h */
void
zint_add_scaled_mul_small(uint32_t *restrict x, size_t xlen,
	const uint32_t *restrict y, size_t ylen, size_t stride,
	int32_t k, uint32_t sch, uint32_t scl)
{
	if (ylen == 0) {
		return;
	}

	uint32_t ysign = -(y[stride * (ylen - 1)] >> 30) >> 1;
	uint32_t tw = 0;
	int32_t cc = 0;
	x += sch * stride;
	for (size_t i = sch; i < xlen; i ++) {
		/*
		 * Get the next word of (2^sc)*y.
		 */
		uint32_t wy;
		if (ylen > 0) {
			wy = *y;
			y += stride;
			ylen --;
		} else {
			wy = ysign;
		}
		uint32_t wys = ((wy << scl) & 0x7FFFFFFF) | tw;
		tw = wy >> (31 - scl);

		/*
		 * The expression below does not overflow.
		 */
		uint64_t z = (uint64_t)((int64_t)wys * (int64_t)k
			+ (int64_t)*x + cc);
		*x = (uint32_t)z & 0x7FFFFFFF;
		x += stride;

		/*
		 * New carry word is a _signed_ right-shift of z.
		 */
		uint32_t ccu = (uint32_t)(z >> 31);
		cc = *(int32_t *)&ccu;
	}
}

/* see kgen_inner.h */
void
zint_sub_scaled(uint32_t *restrict x, size_t xlen,
	const uint32_t *restrict y, size_t ylen, size_t stride,
	uint32_t sch, uint32_t scl)
{
	if (ylen == 0) {
		return;
	}

	uint32_t ysign = -(y[stride * (ylen - 1)] >> 30) >> 1;
	uint32_t tw = 0;
	int32_t cc = 0;
	x += sch * stride;
	for (size_t i = sch; i < xlen; i ++) {
		/*
		 * Get the next word of (2^sc)*y.
		 */
		uint32_t wy;
		if (ylen > 0) {
			wy = *y;
			y += stride;
			ylen --;
		} else {
			wy = ysign;
		}
		uint32_t wys = ((wy << scl) & 0x7FFFFFFF) | tw;
		tw = wy >> (31 - scl);

		uint32_t w = *x - wys - cc;
		*x = w & 0x7FFFFFFF;
		cc = w >> 31;
		x += stride;
	}
}
