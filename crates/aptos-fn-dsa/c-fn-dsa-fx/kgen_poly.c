/*
 * Polynomials with integer coefficients.
 */

#include "kgen_inner.h"

/* see kgen_inner.h */
void
poly_mp_set_small(unsigned logn, uint32_t *restrict d,
	const int8_t *restrict f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		d[i] = mp_set(f[i], p);
	}
}

#if FNDSA_AVX2
/* see kgen_inner.h */
TARGET_AVX2
void
avx2_poly_mp_set_small(unsigned logn, uint32_t *restrict d,
	const int8_t *restrict f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	if (n >= 8) {
		__m256i yp = _mm256_set1_epi32(p);
		for (size_t i = 0; i < n; i += 8) {
			__m256i yf = _mm256_setr_epi32(
				f[i + 0], f[i + 1], f[i + 2], f[i + 3],
				f[i + 4], f[i + 5], f[i + 6], f[i + 7]);
			yf = mp_set_x8(yf, yp);
			_mm256_storeu_si256((__m256i *)(d + i), yf);
		}
		return;
	}
	for (size_t i = 0; i < n; i ++) {
		d[i] = mp_set(f[i], p);
	}
}
#endif

/* see kgen_inner.h */
void
poly_mp_set(unsigned logn, uint32_t *f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = f[i];
		x |= (x & 0x40000000) << 1;
		f[i] = mp_set(*(int32_t *)&x, p);
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_poly_mp_set(unsigned logn, uint32_t *f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	if (n >= 8) {
		__m256i yps = _mm256_set1_epi32(p + 0x80000000);
		__m256i yt = _mm256_set1_epi32(0x3FFFFFFF);
		for (size_t i = 0; i < n; i += 8) {
			__m256i yf = _mm256_loadu_si256((__m256i *)(f + i));
			yf = _mm256_add_epi32(yf, _mm256_and_si256(yps,
				_mm256_cmpgt_epi32(yf, yt)));
			_mm256_storeu_si256((__m256i *)(f + i), yf);
		}
		return;
	}
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = f[i];
		x |= (x & 0x40000000) << 1;
		f[i] = mp_set(*(int32_t *)&x, p);
	}
}
#endif

/* see kgen_inner.h */
void
poly_mp_norm(unsigned logn, uint32_t *f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		f[i] = (uint32_t)mp_norm(f[i], p) & 0x7FFFFFFF;
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_poly_mp_norm(unsigned logn, uint32_t *f, uint32_t p)
{
	size_t n = (size_t)1 << logn;
	if (n >= 8) {
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yhp = _mm256_srli_epi32(yp, 1);
		__m256i ym = _mm256_set1_epi32(0x7FFFFFFF);
		for (size_t i = 0; i < n; i += 8) {
			__m256i yf = _mm256_loadu_si256((__m256i *)(f + i));
			yf = _mm256_and_si256(ym, mp_norm_x8(yf, yp, yhp));
			_mm256_storeu_si256((__m256i *)(f + i), yf);
		}
		return;
	}
	for (size_t i = 0; i < n; i ++) {
		f[i] = (uint32_t)mp_norm(f[i], p) & 0x7FFFFFFF;
	}
}
#endif

/* see kgen_inner.h */
int
poly_big_to_small(unsigned logn, int8_t *restrict d,
	const uint32_t *restrict s, int lim)
{
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		uint32_t x = s[i];
		x |= (x & 0x40000000) << 1;
		int32_t z = *(int32_t *)&x;
		if (z < -lim || z > lim) {
			return 0;
		}
		d[i] = (int8_t)z;
	}
	return 1;
}

/* see kgen_inner.h */
uint32_t
poly_max_bitlength(unsigned logn, const uint32_t *f, size_t flen)
{
	if (flen == 0) {
		return 0;
	}

	size_t n = (size_t)1 << logn;
	uint32_t t = 0;
	uint32_t tk = 0;
	for (size_t i = 0; i < n; i ++, f ++) {
		/* Extend sign bit into a 31-bit mask. */
		uint32_t m = -(f[(flen - 1) << logn] >> 30) & 0x7FFFFFFF;

		/* Get top non-zero sign-adjusted word, with index. */
		uint32_t c = 0;
		uint32_t ck = 0;
		for (size_t j = 0; j < flen; j ++) {
			uint32_t w = f[j << logn] ^ m; /* sign-adjusted word */
			uint32_t nz = ((w - 1) >> 31) - 1;
			c ^= nz & (c ^ w);
			ck ^= nz & (ck ^ (uint32_t)j);
		}

		/* If ck > tk, or tk == ck but c > t, then (c,ck) must
		   replace (t,tk) as current candidate for top word/index. */
		uint32_t rr = tbmask((tk - ck) | (((tk ^ ck) - 1) & (t - c)));
		t ^= rr & (t ^ c);
		tk ^= rr & (tk ^ ck);
	}

	/*
	 * Get bit length of the top word (which has been sign-adjusted)
	 * and return the result.
	 */
	return 31 * tk + 32 - lzcnt(t);
}

/* see kgen_inner.h */
void
poly_big_to_fixed(unsigned logn, fxr *restrict d, const uint32_t *restrict f,
	size_t len, uint32_t sc)
{
	size_t n = (size_t)1 << logn;
	if (len == 0) {
		memset(d, 0, n * sizeof *d);
		return;
	}

	/*
	 * We split the bit length into sch and scl such that:
	 *   sc = 31*sch + scl
	 * We also want scl in the 1..31 range, not 0..30. It may happen
	 * that sch becomes -1, which will "wrap around" (harmlessly).
	 *
	 * For each coefficient, we need three words, each with a given
	 * left shift (negative for a right shift):
	 *    sch-1   1 - scl
	 *    sch     32 - scl
	 *    sch+1   63 - scl
	 */
	uint32_t sch, scl;
	DIVREM31(sch, scl, sc);
	uint32_t z = (scl - 1) >> 31;
	sch -= z;
	scl |= 31 & -z;

	uint32_t t0 = (uint32_t)(sch - 1) & 0xFFFFFF;
	uint32_t t1 = sch & 0xFFFFFF;
	uint32_t t2 = (uint32_t)(sch + 1) & 0xFFFFFF;

	for (size_t i = 0; i < n; i ++, f ++) {
		uint32_t w0, w1, w2, ws, xl, xh;

		w0 = 0;
		w1 = 0;
		w2 = 0;
		for (size_t j = 0; j < len; j ++) {
			uint32_t t, w;

			w = f[j << logn];
			t = (uint32_t)j & 0xFFFFFF;
			w0 |= w & -((uint32_t)((t ^ t0) - 1) >> 31);
			w1 |= w & -((uint32_t)((t ^ t1) - 1) >> 31);
			w2 |= w & -((uint32_t)((t ^ t2) - 1) >> 31);
		}

		/*
		 * If there were not enough words for the requested
		 * scaling, then we must supply copies with the proper
		 * sign.
		 */
		ws = -(f[(len - 1) << logn] >> 30) >> 1;
		w0 |= ws & -((uint32_t)((uint32_t)len - sch) >> 31);
		w1 |= ws & -((uint32_t)((uint32_t)len - sch - 1) >> 31);
		w2 |= ws & -((uint32_t)((uint32_t)len - sch - 2) >> 31);

		/*
		 * Assemble the 64-bit value with the shifts. We assume
		 * that shifts on 32-bit values are constant-time with
		 * regard to the shift count (this should be true on all
		 * modern architectures; the last notable arch on which
		 * shift timing depended on the count was the Pentium IV).
		 *
		 * Since the shift count (scl) is guaranteed to be in 1..31,
		 * we do not have special cases to handle.
		 *
		 * We must sign-extend w2 to ensure the sign bit is properly
		 * set in the fnr value.
		 */
		w2 |= (uint32_t)(w2 & 0x40000000) << 1;
		xl = (w0 >> (scl - 1)) | (w1 << (32 - scl));
		xh = (w1 >> scl) | (w2 << (31 - scl));
		d[i] = fxr_of_scaled32((uint64_t)xl | ((uint64_t)xh << 32));
	}
}

/* see kgen_inner.h */
void
poly_sub_scaled(unsigned logn,
	uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc)
{
	if (flen == 0) {
		return;
	}
	uint32_t sch, scl;
	DIVREM31(sch, scl, sc);
	if (sch >= Flen) {
		return;
	}
	F += (size_t)sch << logn;
	Flen -= sch;
	switch (logn) {
	case 1: {
		uint32_t t0 = 0;
		uint32_t t1 = 0;
		uint32_t signf0 = -(f[(flen << 1) - 2] >> 30) >> 1;
		uint32_t signf1 = -(f[(flen << 1) - 1] >> 30) >> 1;
		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int64_t cc0 = 0;
		int64_t cc1 = 0;
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			uint32_t f0, f1;
			if (i < flen) {
				f0 = f[(i << 1) + 0];
				f1 = f[(i << 1) + 1];
			} else {
				f0 = signf0;
				f1 = signf1;
			}
			uint32_t fs0 = ((f0 << scl) & 0x7FFFFFFF) | t0;
			uint32_t fs1 = ((f1 << scl) & 0x7FFFFFFF) | t1;
			t0 = f0 >> (31 - scl);
			t1 = f1 >> (31 - scl);

			uint32_t F0 = F[(i << 1) + 0];
			uint32_t F1 = F[(i << 1) + 1];
			int64_t z0 = (int64_t)F0 + cc0
				- (int64_t)fs0 * (int64_t)k0
				+ (int64_t)fs1 * (int64_t)k1;
			int64_t z1 = (int64_t)F1 + cc1
				- (int64_t)fs0 * (int64_t)k1
				- (int64_t)fs1 * (int64_t)k0;
			F[(i << 1) + 0] = (uint32_t)z0 & 0x7FFFFFFF;
			F[(i << 1) + 1] = (uint32_t)z1 & 0x7FFFFFFF;
			cc0 = z0 >> 31;
			cc1 = z1 >> 31;
		}
		return;
	}

	case 2: {
		uint32_t t0 = 0;
		uint32_t t1 = 0;
		uint32_t t2 = 0;
		uint32_t t3 = 0;
		uint32_t signf0 = -(f[(flen << 2) - 4] >> 30) >> 1;
		uint32_t signf1 = -(f[(flen << 2) - 3] >> 30) >> 1;
		uint32_t signf2 = -(f[(flen << 2) - 2] >> 30) >> 1;
		uint32_t signf3 = -(f[(flen << 2) - 1] >> 30) >> 1;
		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int32_t k2 = k[2];
		int32_t k3 = k[3];
		int64_t cc0 = 0;
		int64_t cc1 = 0;
		int64_t cc2 = 0;
		int64_t cc3 = 0;
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			uint32_t f0, f1, f2, f3;
			if (i < flen) {
				f0 = f[(i << 2) + 0];
				f1 = f[(i << 2) + 1];
				f2 = f[(i << 2) + 2];
				f3 = f[(i << 2) + 3];
			} else {
				f0 = signf0;
				f1 = signf1;
				f2 = signf2;
				f3 = signf3;
			}
			uint32_t fs0 = ((f0 << scl) & 0x7FFFFFFF) | t0;
			uint32_t fs1 = ((f1 << scl) & 0x7FFFFFFF) | t1;
			uint32_t fs2 = ((f2 << scl) & 0x7FFFFFFF) | t2;
			uint32_t fs3 = ((f3 << scl) & 0x7FFFFFFF) | t3;
			t0 = f0 >> (31 - scl);
			t1 = f1 >> (31 - scl);
			t2 = f2 >> (31 - scl);
			t3 = f3 >> (31 - scl);

			uint32_t F0 = F[(i << 2) + 0];
			uint32_t F1 = F[(i << 2) + 1];
			uint32_t F2 = F[(i << 2) + 2];
			uint32_t F3 = F[(i << 2) + 3];
			int64_t z0 = (int64_t)F0 + cc0
				- (int64_t)fs0 * (int64_t)k0
				+ (int64_t)fs1 * (int64_t)k3
				+ (int64_t)fs2 * (int64_t)k2
				+ (int64_t)fs3 * (int64_t)k1;
			int64_t z1 = (int64_t)F1 + cc1
				- (int64_t)fs0 * (int64_t)k1
				- (int64_t)fs1 * (int64_t)k0
				+ (int64_t)fs2 * (int64_t)k3
				+ (int64_t)fs3 * (int64_t)k2;
			int64_t z2 = (int64_t)F2 + cc2
				- (int64_t)fs0 * (int64_t)k2
				- (int64_t)fs1 * (int64_t)k1
				- (int64_t)fs2 * (int64_t)k0
				+ (int64_t)fs3 * (int64_t)k3;
			int64_t z3 = (int64_t)F3 + cc3
				- (int64_t)fs0 * (int64_t)k3
				- (int64_t)fs1 * (int64_t)k2
				- (int64_t)fs2 * (int64_t)k1
				- (int64_t)fs3 * (int64_t)k0;
			F[(i << 2) + 0] = (uint32_t)z0 & 0x7FFFFFFF;
			F[(i << 2) + 1] = (uint32_t)z1 & 0x7FFFFFFF;
			F[(i << 2) + 2] = (uint32_t)z2 & 0x7FFFFFFF;
			F[(i << 2) + 3] = (uint32_t)z3 & 0x7FFFFFFF;
			cc0 = z0 >> 31;
			cc1 = z1 >> 31;
			cc2 = z2 >> 31;
			cc3 = z3 >> 31;
		}
		return;
	}

	case 3: {
		uint32_t t0 = 0;
		uint32_t t1 = 0;
		uint32_t t2 = 0;
		uint32_t t3 = 0;
		uint32_t t4 = 0;
		uint32_t t5 = 0;
		uint32_t t6 = 0;
		uint32_t t7 = 0;
		uint32_t signf0 = -(f[(flen << 3) - 8] >> 30) >> 1;
		uint32_t signf1 = -(f[(flen << 3) - 7] >> 30) >> 1;
		uint32_t signf2 = -(f[(flen << 3) - 6] >> 30) >> 1;
		uint32_t signf3 = -(f[(flen << 3) - 5] >> 30) >> 1;
		uint32_t signf4 = -(f[(flen << 3) - 4] >> 30) >> 1;
		uint32_t signf5 = -(f[(flen << 3) - 3] >> 30) >> 1;
		uint32_t signf6 = -(f[(flen << 3) - 2] >> 30) >> 1;
		uint32_t signf7 = -(f[(flen << 3) - 1] >> 30) >> 1;
		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int32_t k2 = k[2];
		int32_t k3 = k[3];
		int32_t k4 = k[4];
		int32_t k5 = k[5];
		int32_t k6 = k[6];
		int32_t k7 = k[7];
		int64_t cc0 = 0;
		int64_t cc1 = 0;
		int64_t cc2 = 0;
		int64_t cc3 = 0;
		int64_t cc4 = 0;
		int64_t cc5 = 0;
		int64_t cc6 = 0;
		int64_t cc7 = 0;
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			uint32_t f0, f1, f2, f3, f4, f5, f6, f7;
			if (i < flen) {
				f0 = f[(i << 3) + 0];
				f1 = f[(i << 3) + 1];
				f2 = f[(i << 3) + 2];
				f3 = f[(i << 3) + 3];
				f4 = f[(i << 3) + 4];
				f5 = f[(i << 3) + 5];
				f6 = f[(i << 3) + 6];
				f7 = f[(i << 3) + 7];
			} else {
				f0 = signf0;
				f1 = signf1;
				f2 = signf2;
				f3 = signf3;
				f4 = signf4;
				f5 = signf5;
				f6 = signf6;
				f7 = signf7;
			}
			uint32_t fs0 = ((f0 << scl) & 0x7FFFFFFF) | t0;
			uint32_t fs1 = ((f1 << scl) & 0x7FFFFFFF) | t1;
			uint32_t fs2 = ((f2 << scl) & 0x7FFFFFFF) | t2;
			uint32_t fs3 = ((f3 << scl) & 0x7FFFFFFF) | t3;
			uint32_t fs4 = ((f4 << scl) & 0x7FFFFFFF) | t4;
			uint32_t fs5 = ((f5 << scl) & 0x7FFFFFFF) | t5;
			uint32_t fs6 = ((f6 << scl) & 0x7FFFFFFF) | t6;
			uint32_t fs7 = ((f7 << scl) & 0x7FFFFFFF) | t7;
			t0 = f0 >> (31 - scl);
			t1 = f1 >> (31 - scl);
			t2 = f2 >> (31 - scl);
			t3 = f3 >> (31 - scl);
			t4 = f4 >> (31 - scl);
			t5 = f5 >> (31 - scl);
			t6 = f6 >> (31 - scl);
			t7 = f7 >> (31 - scl);

			uint32_t F0 = F[(i << 3) + 0];
			uint32_t F1 = F[(i << 3) + 1];
			uint32_t F2 = F[(i << 3) + 2];
			uint32_t F3 = F[(i << 3) + 3];
			uint32_t F4 = F[(i << 3) + 4];
			uint32_t F5 = F[(i << 3) + 5];
			uint32_t F6 = F[(i << 3) + 6];
			uint32_t F7 = F[(i << 3) + 7];
			int64_t z0 = (int64_t)F0 + cc0
				- (int64_t)fs0 * (int64_t)k0
				+ (int64_t)fs1 * (int64_t)k7
				+ (int64_t)fs2 * (int64_t)k6
				+ (int64_t)fs3 * (int64_t)k5
				+ (int64_t)fs4 * (int64_t)k4
				+ (int64_t)fs5 * (int64_t)k3
				+ (int64_t)fs6 * (int64_t)k2
				+ (int64_t)fs7 * (int64_t)k1;
			int64_t z1 = (int64_t)F1 + cc1
				- (int64_t)fs0 * (int64_t)k1
				- (int64_t)fs1 * (int64_t)k0
				+ (int64_t)fs2 * (int64_t)k7
				+ (int64_t)fs3 * (int64_t)k6
				+ (int64_t)fs4 * (int64_t)k5
				+ (int64_t)fs5 * (int64_t)k4
				+ (int64_t)fs6 * (int64_t)k3
				+ (int64_t)fs7 * (int64_t)k2;
			int64_t z2 = (int64_t)F2 + cc2
				- (int64_t)fs0 * (int64_t)k2
				- (int64_t)fs1 * (int64_t)k1
				- (int64_t)fs2 * (int64_t)k0
				+ (int64_t)fs3 * (int64_t)k7
				+ (int64_t)fs4 * (int64_t)k6
				+ (int64_t)fs5 * (int64_t)k5
				+ (int64_t)fs6 * (int64_t)k4
				+ (int64_t)fs7 * (int64_t)k3;
			int64_t z3 = (int64_t)F3 + cc3
				- (int64_t)fs0 * (int64_t)k3
				- (int64_t)fs1 * (int64_t)k2
				- (int64_t)fs2 * (int64_t)k1
				- (int64_t)fs3 * (int64_t)k0
				+ (int64_t)fs4 * (int64_t)k7
				+ (int64_t)fs5 * (int64_t)k6
				+ (int64_t)fs6 * (int64_t)k5
				+ (int64_t)fs7 * (int64_t)k4;
			int64_t z4 = (int64_t)F4 + cc4
				- (int64_t)fs0 * (int64_t)k4
				- (int64_t)fs1 * (int64_t)k3
				- (int64_t)fs2 * (int64_t)k2
				- (int64_t)fs3 * (int64_t)k1
				- (int64_t)fs4 * (int64_t)k0
				+ (int64_t)fs5 * (int64_t)k7
				+ (int64_t)fs6 * (int64_t)k6
				+ (int64_t)fs7 * (int64_t)k5;
			int64_t z5 = (int64_t)F5 + cc5
				- (int64_t)fs0 * (int64_t)k5
				- (int64_t)fs1 * (int64_t)k4
				- (int64_t)fs2 * (int64_t)k3
				- (int64_t)fs3 * (int64_t)k2
				- (int64_t)fs4 * (int64_t)k1
				- (int64_t)fs5 * (int64_t)k0
				+ (int64_t)fs6 * (int64_t)k7
				+ (int64_t)fs7 * (int64_t)k6;
			int64_t z6 = (int64_t)F6 + cc6
				- (int64_t)fs0 * (int64_t)k6
				- (int64_t)fs1 * (int64_t)k5
				- (int64_t)fs2 * (int64_t)k4
				- (int64_t)fs3 * (int64_t)k3
				- (int64_t)fs4 * (int64_t)k2
				- (int64_t)fs5 * (int64_t)k1
				- (int64_t)fs6 * (int64_t)k0
				+ (int64_t)fs7 * (int64_t)k7;
			int64_t z7 = (int64_t)F7 + cc7
				- (int64_t)fs0 * (int64_t)k7
				- (int64_t)fs1 * (int64_t)k6
				- (int64_t)fs2 * (int64_t)k5
				- (int64_t)fs3 * (int64_t)k4
				- (int64_t)fs4 * (int64_t)k3
				- (int64_t)fs5 * (int64_t)k2
				- (int64_t)fs6 * (int64_t)k1
				- (int64_t)fs7 * (int64_t)k0;
			F[(i << 3) + 0] = (uint32_t)z0 & 0x7FFFFFFF;
			F[(i << 3) + 1] = (uint32_t)z1 & 0x7FFFFFFF;
			F[(i << 3) + 2] = (uint32_t)z2 & 0x7FFFFFFF;
			F[(i << 3) + 3] = (uint32_t)z3 & 0x7FFFFFFF;
			F[(i << 3) + 4] = (uint32_t)z4 & 0x7FFFFFFF;
			F[(i << 3) + 5] = (uint32_t)z5 & 0x7FFFFFFF;
			F[(i << 3) + 6] = (uint32_t)z6 & 0x7FFFFFFF;
			F[(i << 3) + 7] = (uint32_t)z7 & 0x7FFFFFFF;
			cc0 = z0 >> 31;
			cc1 = z1 >> 31;
			cc2 = z2 >> 31;
			cc3 = z3 >> 31;
			cc4 = z4 >> 31;
			cc5 = z5 >> 31;
			cc6 = z6 >> 31;
			cc7 = z7 >> 31;
		}
		return;
	}
	}
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		int32_t kf = -k[i];
		uint32_t *x = F + i;
		for (size_t j = 0; j < n; j ++) {
			zint_add_scaled_mul_small(
				x, Flen, f + j, flen, n, kf, 0, scl);
			if (i + j == n - 1) {
				x = F;
				kf = -kf;
			} else {
				x ++;
			}
		}
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_poly_sub_scaled(unsigned logn,
	uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc)
{
	if (flen == 0) {
		return;
	}
	uint32_t sch, scl;
	DIVREM31(sch, scl, sc);
	if (sch >= Flen) {
		return;
	}
	F += (size_t)sch << logn;
	Flen -= sch;
	switch (logn) {
	case 1: {
		uint32_t t0 = 0;
		uint32_t t1 = 0;
		uint32_t signf0 = -(f[(flen << 1) - 2] >> 30) >> 1;
		uint32_t signf1 = -(f[(flen << 1) - 1] >> 30) >> 1;
		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int64_t cc0 = 0;
		int64_t cc1 = 0;
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			uint32_t f0, f1;
			if (i < flen) {
				f0 = f[(i << 1) + 0];
				f1 = f[(i << 1) + 1];
			} else {
				f0 = signf0;
				f1 = signf1;
			}
			uint32_t fs0 = ((f0 << scl) & 0x7FFFFFFF) | t0;
			uint32_t fs1 = ((f1 << scl) & 0x7FFFFFFF) | t1;
			t0 = f0 >> (31 - scl);
			t1 = f1 >> (31 - scl);

			uint32_t F0 = F[(i << 1) + 0];
			uint32_t F1 = F[(i << 1) + 1];
			int64_t z0 = (int64_t)F0 + cc0
				- (int64_t)fs0 * (int64_t)k0
				+ (int64_t)fs1 * (int64_t)k1;
			int64_t z1 = (int64_t)F1 + cc1
				- (int64_t)fs0 * (int64_t)k1
				- (int64_t)fs1 * (int64_t)k0;
			F[(i << 1) + 0] = (uint32_t)z0 & 0x7FFFFFFF;
			F[(i << 1) + 1] = (uint32_t)z1 & 0x7FFFFFFF;
			cc0 = z0 >> 31;
			cc1 = z1 >> 31;
		}
		return;
	}

	case 2: {
		__m128i xt = _mm_setzero_si128();
		__m128i xsignf = _mm_loadu_si128(
			(const __m128i *)(f + (flen << 2) - 4));
		xsignf = _mm_srli_epi32(
			_mm_srai_epi32(_mm_slli_epi32(xsignf, 1), 31), 1);
		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int32_t k2 = k[2];
		int32_t k3 = k[3];
		__m256i yk0 = _mm256_setr_epi32(-k0, 0, -k1, 0, -k2, 0, -k3, 0);
		__m256i yk1 = _mm256_setr_epi32(+k3, 0, -k0, 0, -k1, 0, -k2, 0);
		__m256i yk2 = _mm256_setr_epi32(+k2, 0, +k3, 0, -k0, 0, -k1, 0);
		__m256i yk3 = _mm256_setr_epi32(+k1, 0, +k2, 0, +k3, 0, -k0, 0);
		__m128i xscl = _mm_cvtsi32_si128(scl);
		__m128i xnscl = _mm_cvtsi32_si128(31 - scl);
		__m256i ycc = _mm256_setzero_si256();
		__m128i x31 = _mm_set1_epi32(0x7FFFFFFF);
		__m256i y31 = _mm256_set1_epi32(0x7FFFFFFF);
		__m256i y31lo = _mm256_set1_epi64x(0x7FFFFFFF);
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			__m128i xf;
			if (i < flen) {
				xf = _mm_loadu_si128((__m128i *)(f + (i << 2)));
			} else {
				xf = xsignf;
			}
			__m128i xfs = _mm_or_si128(xt,
				_mm_and_si128(_mm_sll_epi32(xf, xscl), x31));
			xt = _mm_srl_epi32(xf, xnscl);

			__m256i yfs0 = _mm256_broadcastd_epi32(xfs);
			__m256i yfs1 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs, 4));
			__m256i yfs2 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs, 8));
			__m256i yfs3 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs, 12));

			__m256i yF = _mm256_castsi128_si256(
				_mm_loadu_si128((__m128i *)(F + (i << 2))));
			yF = _mm256_shuffle_epi32(
				_mm256_permute4x64_epi64(yF, 0x10), 0x10);
			yF = _mm256_and_si256(yF, y31lo);

			__m256i yv0 = _mm256_mul_epi32(yfs0, yk0);
			__m256i yv1 = _mm256_mul_epi32(yfs1, yk1);
			__m256i yv2 = _mm256_mul_epi32(yfs2, yk2);
			__m256i yv3 = _mm256_mul_epi32(yfs3, yk3);
			__m256i yz = _mm256_add_epi64(
				_mm256_add_epi64(
					_mm256_add_epi64(yv0, yv1),
					_mm256_add_epi64(yv2, yv3)),
				_mm256_add_epi64(ycc, yF));
			ycc = _mm256_blend_epi32(
				_mm256_srli_epi64(yz, 31),
				_mm256_srai_epi32(yz, 31), 0xAA);
			yz = _mm256_shuffle_epi32(
				_mm256_and_si256(yz, y31), 0x88);
			yz = _mm256_permute4x64_epi64(yz, 0x88);
			__m128i xF = _mm256_castsi256_si128(yz);
			_mm_storeu_si128((__m128i *)(F + (i << 2)), xF);
		}
		return;
	}

	case 3: {
		__m256i yt = _mm256_setzero_si256();
		__m256i ysignf = _mm256_loadu_si256(
			(const __m256i *)(f + (flen << 3) - 8));
		ysignf = _mm256_srli_epi32(
			_mm256_srai_epi32(_mm256_slli_epi32(ysignf, 1), 31), 1);

		int32_t k0 = k[0];
		int32_t k1 = k[1];
		int32_t k2 = k[2];
		int32_t k3 = k[3];
		int32_t k4 = k[4];
		int32_t k5 = k[5];
		int32_t k6 = k[6];
		int32_t k7 = k[7];
		__m256i yk0l = _mm256_setr_epi32(
			-k0, -k1, -k2, -k3, -k4, -k5, -k6, -k7);
		__m256i yk1l = _mm256_setr_epi32(
			+k7, -k0, -k1, -k2, -k3, -k4, -k5, -k6);
		__m256i yk2l = _mm256_setr_epi32(
			+k6, +k7, -k0, -k1, -k2, -k3, -k4, -k5);
		__m256i yk3l = _mm256_setr_epi32(
			+k5, +k6, +k7, -k0, -k1, -k2, -k3, -k4);
		__m256i yk4l = _mm256_setr_epi32(
			+k4, +k5, +k6, +k7, -k0, -k1, -k2, -k3);
		__m256i yk5l = _mm256_setr_epi32(
			+k3, +k4, +k5, +k6, +k7, -k0, -k1, -k2);
		__m256i yk6l = _mm256_setr_epi32(
			+k2, +k3, +k4, +k5, +k6, +k7, -k0, -k1);
		__m256i yk7l = _mm256_setr_epi32(
			+k1, +k2, +k3, +k4, +k5, +k6, +k7, -k0);
		__m256i yk0h = _mm256_srli_epi64(yk0l, 32);
		__m256i yk1h = _mm256_srli_epi64(yk1l, 32);
		__m256i yk2h = _mm256_srli_epi64(yk2l, 32);
		__m256i yk3h = _mm256_srli_epi64(yk3l, 32);
		__m256i yk4h = _mm256_srli_epi64(yk4l, 32);
		__m256i yk5h = _mm256_srli_epi64(yk5l, 32);
		__m256i yk6h = _mm256_srli_epi64(yk6l, 32);
		__m256i yk7h = _mm256_srli_epi64(yk7l, 32);

		__m128i xscl = _mm_cvtsi32_si128(scl);
		__m128i xnscl = _mm_cvtsi32_si128(31 - scl);
		__m256i ycc0 = _mm256_setzero_si256();
		__m256i ycc1 = _mm256_setzero_si256();
		__m256i y31 = _mm256_set1_epi32(0x7FFFFFFF);
		__m256i y31lo = _mm256_set1_epi64x(0x7FFFFFFF);
		for (size_t i = 0; i < Flen; i ++) {
			/*
			 * Next word, shifted.
			 */
			__m256i yf;
			if (i < flen) {
				yf = _mm256_loadu_si256(
					(const __m256i *)(f + (i << 3)));
			} else {
				yf = ysignf;
			}
			__m256i yfs = _mm256_or_si256(yt,
				_mm256_and_si256(
					_mm256_sll_epi32(yf, xscl), y31));
			yt = _mm256_srl_epi32(yf, xnscl);

			__m128i xfs0 = _mm256_castsi256_si128(yfs);
			__m128i xfs1 = _mm256_extracti128_si256(yfs, 1);
			__m256i yfs0 = _mm256_broadcastd_epi32(xfs0);
			__m256i yfs1 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs0, 4));
			__m256i yfs2 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs0, 8));
			__m256i yfs3 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs0, 12));
			__m256i yfs4 = _mm256_broadcastd_epi32(xfs1);
			__m256i yfs5 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs1, 4));
			__m256i yfs6 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs1, 8));
			__m256i yfs7 = _mm256_broadcastd_epi32(
				_mm_bsrli_si128(xfs1, 12));

			__m256i yF = _mm256_loadu_si256(
				(const __m256i *)(F + (i << 3)));
			__m256i yF0 = _mm256_and_si256(yF, y31lo);
			__m256i yF1 = _mm256_srli_epi64(yF, 32);

			__m256i yv0l = _mm256_mul_epi32(yfs0, yk0l);
			__m256i yv0h = _mm256_mul_epi32(yfs0, yk0h);
			__m256i yv1l = _mm256_mul_epi32(yfs1, yk1l);
			__m256i yv1h = _mm256_mul_epi32(yfs1, yk1h);
			__m256i yv2l = _mm256_mul_epi32(yfs2, yk2l);
			__m256i yv2h = _mm256_mul_epi32(yfs2, yk2h);
			__m256i yv3l = _mm256_mul_epi32(yfs3, yk3l);
			__m256i yv3h = _mm256_mul_epi32(yfs3, yk3h);
			__m256i yv4l = _mm256_mul_epi32(yfs4, yk4l);
			__m256i yv4h = _mm256_mul_epi32(yfs4, yk4h);
			__m256i yv5l = _mm256_mul_epi32(yfs5, yk5l);
			__m256i yv5h = _mm256_mul_epi32(yfs5, yk5h);
			__m256i yv6l = _mm256_mul_epi32(yfs6, yk6l);
			__m256i yv6h = _mm256_mul_epi32(yfs6, yk6h);
			__m256i yv7l = _mm256_mul_epi32(yfs7, yk7l);
			__m256i yv7h = _mm256_mul_epi32(yfs7, yk7h);

			__m256i yz0 = _mm256_add_epi64(
				_mm256_add_epi64(ycc0, yF0),
				_mm256_add_epi64(
					_mm256_add_epi64(
						_mm256_add_epi64(yv0l, yv1l),
						_mm256_add_epi64(yv2l, yv3l)),
					_mm256_add_epi64(
						_mm256_add_epi64(yv4l, yv5l),
						_mm256_add_epi64(yv6l, yv7l))));
			__m256i yz1 = _mm256_add_epi64(
				_mm256_add_epi64(ycc1, yF1),
				_mm256_add_epi64(
					_mm256_add_epi64(
						_mm256_add_epi64(yv0h, yv1h),
						_mm256_add_epi64(yv2h, yv3h)),
					_mm256_add_epi64(
						_mm256_add_epi64(yv4h, yv5h),
						_mm256_add_epi64(yv6h, yv7h))));
			ycc0 = _mm256_blend_epi32(
				_mm256_srli_epi64(yz0, 31),
				_mm256_srai_epi32(yz0, 31), 0xAA);
			ycc1 = _mm256_blend_epi32(
				_mm256_srli_epi64(yz1, 31),
				_mm256_srai_epi32(yz1, 31), 0xAA);
			yF = _mm256_or_si256(
				_mm256_and_si256(yz0, y31lo),
				_mm256_slli_epi64(
					_mm256_and_si256(yz1, y31lo), 32));
			_mm256_storeu_si256((__m256i *)(F + (i << 3)), yF);
		}
		return;
	}
	}
	size_t n = (size_t)1 << logn;
	for (size_t i = 0; i < n; i ++) {
		int32_t kf = -k[i];
		uint32_t *x = F + i;
		for (size_t j = 0; j < n; j ++) {
			zint_add_scaled_mul_small(
				x, Flen, f + j, flen, n, kf, 0, scl);
			if (i + j == n - 1) {
				x = F;
				kf = -kf;
			} else {
				x ++;
			}
		}
	}
}
#endif

/* see kgen_inner.h */
void
poly_sub_scaled_ntt(unsigned logn, uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc, uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	size_t tlen = flen + 1;
	uint32_t *gm = tmp;
	uint32_t *igm = gm + n;
	uint32_t *fk = igm + n;
	uint32_t *t1 = fk + (tlen << logn);
	uint32_t sch, scl;
	DIVREM31(sch, scl, sc);

	/*
	 * Compute k*f in fk[], in RNS notation.
	 * f is assumed to be already in RNS+NTT over flen+1 words.
	 */
	for (size_t i = 0; i < tlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		mp_mkgmigm(logn, gm, igm, PRIMES[i].g, PRIMES[i].ig, p, p0i);
		for (size_t j = 0; j < n; j ++) {
			t1[j] = mp_set(k[j], p);
		}
		mp_NTT(logn, t1, gm, p, p0i);

		const uint32_t *fs = f + (i << logn);
		uint32_t *ff = fk + (i << logn);
		for (size_t j = 0; j < n; j ++) {
			ff[j] = mp_mmul(
				mp_mmul(t1[j], fs[j], p, p0i), R2, p, p0i);
		}
		mp_iNTT(logn, ff, igm, p, p0i);
	}

	/*
	 * Rebuild k*f.
	 */
	zint_rebuild_CRT(fk, tlen, n, 1, 1, t1);

	/*
	 * Subtract k*f, scaled, from F.
	 */
	for (size_t i = 0; i < n; i ++) {
		zint_sub_scaled(F + i, Flen, fk + i, tlen, n, sch, scl);
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_poly_sub_scaled_ntt(unsigned logn, uint32_t *restrict F, size_t Flen,
	const uint32_t *restrict f, size_t flen,
	const int32_t *restrict k, uint32_t sc, uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	size_t tlen = flen + 1;
	uint32_t *gm = tmp;
	uint32_t *igm = gm + n;
	uint32_t *fk = igm + n;
	uint32_t *t1 = fk + (tlen << logn);
	uint32_t sch, scl;
	DIVREM31(sch, scl, sc);

	/*
	 * Compute k*f in fk[], in RNS notation.
	 * f is assumed to be already in RNS+NTT over flen+1 words.
	 */
	for (size_t i = 0; i < tlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		avx2_mp_mkgmigm(logn, gm, igm,
			PRIMES[i].g, PRIMES[i].ig, p, p0i);
		__m256i yp = _mm256_set1_epi32(p);
		for (size_t j = 0; j < n; j += 8) {
			__m256i yk = _mm256_loadu_si256((__m256i *)(k + j));
			_mm256_storeu_si256((__m256i *)(t1 + j),
				mp_set_x8(yk, yp));
		}
		avx2_mp_NTT(logn, t1, gm, p, p0i);

		const uint32_t *fs = f + (i << logn);
		uint32_t *ff = fk + (i << logn);
		__m256i yp0i = _mm256_set1_epi32(p0i);
		__m256i yR2 = _mm256_set1_epi32(R2);
		for (size_t j = 0; j < n; j += 8) {
			__m256i y1 = _mm256_loadu_si256((__m256i *)(t1 + j));
			__m256i y2 = _mm256_loadu_si256((__m256i *)(fs + j));
			_mm256_storeu_si256((__m256i *)(ff + j),
				mp_mmul_x8(
					mp_mmul_x8(y1, y2, yp, yp0i),
					yR2, yp, yp0i));
		}
		avx2_mp_iNTT(logn, ff, igm, p, p0i);
	}

	/*
	 * Rebuild k*f.
	 */
	avx2_zint_rebuild_CRT(fk, tlen, n, 1, 1, t1);

	/*
	 * Subtract k*f, scaled, from F.
	 */
	for (size_t i = 0; i < n; i ++) {
		zint_sub_scaled(F + i, Flen, fk + i, tlen, n, sch, scl);
	}
}
#endif

/* see kgen_inner.h */
void
poly_sub_kf_scaled_depth1(unsigned logn_top,
	uint32_t *restrict F, size_t FGlen,
	uint32_t *restrict k, uint32_t sc,
	const int8_t *restrict f,
	uint32_t *restrict tmp)
{
	unsigned logn = logn_top - 1;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	uint32_t *gm = tmp;
	uint32_t *t1 = gm + n;
	uint32_t *t2 = t1 + n;

	/*
	 * Step 1: convert F to RNS. Since FGlen is equal to 1 or 2,
	 * we do it with some specialized code. We assume that the RNS
	 * representation does not lose information (i.e. each signed
	 * coefficient is lower than (p0*p1)/2, with FGlen = 2 and the two
	 * prime moduli are p0 and p1).
	 */
	if (FGlen == 1) {
		uint32_t p = PRIMES[0].p;
		for (size_t i = 0; i < n; i ++) {
			uint32_t xf = F[i];
			xf |= (xf & 0x40000000) << 1;
			F[i] = mp_set(*(int32_t *)&xf, p);
		}
	} else {
		uint32_t p0 = PRIMES[0].p;
		uint32_t p0_0i = PRIMES[0].p0i;
		uint32_t z0 = mp_half(PRIMES[0].R2, p0);
		uint32_t p1 = PRIMES[1].p;
		uint32_t p1_0i = PRIMES[1].p0i;
		uint32_t z1 = mp_half(PRIMES[1].R2, p1);
		for (size_t i = 0; i < n; i ++) {
			uint32_t xl, xh, yl0, yh0, r0, yl1, yh1, r1;

			xl = F[i];
			xh = F[i + n] | ((F[i + n] & 0x40000000) << 1);
			yl0 = xl - (p0 & ~tbmask(xl - p0));
			yh0 = mp_set(*(int32_t *)&xh, p0);
			r0 = mp_add(yl0, mp_mmul(yh0, z0, p0, p0_0i), p0);
			yl1 = xl - (p1 & ~tbmask(xl - p1));
			yh1 = mp_set(*(int32_t *)&xh, p1);
			r1 = mp_add(yl1, mp_mmul(yh1, z1, p1, p1_0i), p1);
			F[i] = r0;
			F[i + n] = r1;
		}
	}

	/*
	 * Step 2: for FGlen small primes, convert F to RNS+NTT,
	 * and subtract (2^sc)*ft. The ft polynomial is computed
	 * in RNS+NTT dynamically.
	 */
	for (size_t i = 0; i < FGlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t R3 = mp_mmul(R2, R2, p, p0i);
		mp_mkgm(logn, gm, PRIMES[i].g, p, p0i);

		/*
		 * k <- (2^sc)*k (and into NTT).
		 */
		uint32_t scv = mp_mmul(
			(uint32_t)1 << (sc & 31), R2, p, p0i);
		for (uint32_t m = sc >> 5; m > 0; m --) {
			scv = mp_mmul(scv, R2, p, p0i);
		}
		for (size_t j = 0; j < n; j ++) {
			uint32_t x = mp_set(*(int32_t *)&k[j], p);
			k[j] = mp_mmul(scv, x, p, p0i);
		}
		mp_NTT(logn, k, gm, p, p0i);

		/*
		 * Convert F to NTT.
		 */
		uint32_t *Fu = F + (i << logn);
		mp_NTT(logn, Fu, gm, p, p0i);

		/*
		 * Given the top-level f, we obtain ft = N(f) (the f at
		 * depth 1) with:
		 *    f = f_e(X^2) + X*f_o(X^2)
		 * with f_e and f_o being modulo X^n+1. Then:
		 *    N(f) = f_e^2 - X*f_o^2
		 * The NTT representation of X is obtained from the gm[] tab:
		 *    NTT(X)[2*j + 0] = gm[j + n/2]
		 *    NTT(X)[2*j + 1] = -NTT(X)[2*j + 0]
		 * Note that the values in gm[] are in Montgomery
		 * representation.
		 */
		for (size_t j = 0; j < n; j ++) {
			t1[j] = mp_set(f[(j << 1) + 0], p);
			t2[j] = mp_set(f[(j << 1) + 1], p);
		}
		mp_NTT(logn, t1, gm, p, p0i);
		mp_NTT(logn, t2, gm, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			uint32_t xe0 = t1[(j << 1) + 0];
			uint32_t xe1 = t1[(j << 1) + 1];
			uint32_t xo0 = t2[(j << 1) + 0];
			uint32_t xo1 = t2[(j << 1) + 1];
			uint32_t xv0 = gm[hn + j];
			uint32_t xv1 = p - xv0;
			xe0 = mp_mmul(xe0, xe0, p, p0i);
			xe1 = mp_mmul(xe1, xe1, p, p0i);
			xo0 = mp_mmul(xo0, xo0, p, p0i);
			xo1 = mp_mmul(xo1, xo1, p, p0i);
			uint32_t xf0 = mp_sub(xe0,
				mp_mmul(xo0, xv0, p, p0i), p);
			uint32_t xf1 = mp_sub(xe1,
				mp_mmul(xo1, xv1, p, p0i), p);

			uint32_t xkf0 = mp_mmul(
				mp_mmul(xf0, k[(j << 1) + 0], p, p0i),
				R3, p, p0i);
			uint32_t xkf1 = mp_mmul(
				mp_mmul(xf1, k[(j << 1) + 1], p, p0i),
				R3, p, p0i);
			Fu[(j << 1) + 0] = mp_sub(Fu[(j << 1) + 0], xkf0, p);
			Fu[(j << 1) + 1] = mp_sub(Fu[(j << 1) + 1], xkf1, p);
		}

		/*
		 * Convert F back to RNS.
		 */
		mp_mkigm(logn, t1, PRIMES[i].ig, p, p0i);
		mp_iNTT(logn, Fu, t1, p, p0i);

		/*
		 * We replaced k (plain 32-bit) with (2^sc)*k (NTT). We must
		 * put it back to its initial value if there should be another
		 * iteration.
		 */
		if ((i + 1) < FGlen) {
			mp_iNTT(logn, k, t1, p, p0i);
			scv = (uint32_t)1 << (-sc & 31);
			for (uint32_t m = sc >> 5; m > 0; m --) {
				scv = mp_mmul(scv, 1, p, p0i);
			}
			for (size_t j = 0; j < n; j ++) {
				k[j] = (uint32_t)mp_norm(
					mp_mmul(scv, k[j], p, p0i), p);
			}
		}
	}

	/*
	 * Output F is in RNS (non-NTT), but we want plain integers.
	 */
	if (FGlen == 1) {
		uint32_t p = PRIMES[0].p;
		for (size_t i = 0; i < n; i ++) {
			F[i] = (uint32_t)mp_norm(F[i], p) & 0x7FFFFFFF;
		}
	} else {
		uint32_t p0 = PRIMES[0].p;
		uint32_t p1 = PRIMES[1].p;
		uint32_t p1_0i = PRIMES[1].p0i;
		uint32_t s = PRIMES[1].s;
		uint64_t pp = (uint64_t)p0 * (uint64_t)p1;
		uint64_t hpp = pp >> 1;
		for (size_t i = 0; i < n; i ++) {
			/*
			 * Apply CRT with two primes on the coefficient of F.
			 */
			uint32_t x0 = F[i];      /* mod p0 */
			uint32_t x1 = F[i + n];  /* mod p1 */
			uint32_t x0m1 = x0 - (p1 & ~tbmask(x0 - p1));
			uint32_t y = mp_mmul(
				mp_sub(x1, x0m1, p1), s, p1, p1_0i);
			uint64_t z = (uint64_t)x0 + (uint64_t)p0 * (uint64_t)y;
			z -= pp & -((hpp - z) >> 63);
			F[i] = (uint32_t)z & 0x7FFFFFFF;
			F[i + n] = (uint32_t)(z >> 31) & 0x7FFFFFFF;
		}
	}
}

#if FNDSA_AVX2
TARGET_AVX2
void
avx2_poly_sub_kf_scaled_depth1(unsigned logn_top,
	uint32_t *restrict F, size_t FGlen,
	uint32_t *restrict k, uint32_t sc,
	const int8_t *restrict f,
	uint32_t *restrict tmp)
{
	/* TODO: decide if more AVX2 optimizations are needed here; there
	   are still some loops which could potentially benefit, but
	   possibly at the cost of a code size increase. */
	unsigned logn = logn_top - 1;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	uint32_t *gm = tmp;
	uint32_t *t1 = gm + n;
	uint32_t *t2 = t1 + n;

	/*
	 * Step 1: convert F to RNS. Since FGlen is equal to 1 or 2,
	 * we do it with some specialized code. We assume that the RNS
	 * representation does not lose information (i.e. each signed
	 * coefficient is lower than (p0*p1)/2, with FGlen = 2 and the two
	 * prime moduli are p0 and p1).
	 */
	if (FGlen == 1) {
		uint32_t p = PRIMES[0].p;
		if (logn >= 3) {
			__m256i yp = _mm256_set1_epi32(p);
			__m256i *Fz = (__m256i *)F;
			__m256i yms = _mm256_set1_epi32(0x40000000);
			for (size_t i = 0; i < (n >> 3); i ++) {
				__m256i yf = _mm256_loadu_si256(Fz + i);
				__m256i yfs = _mm256_and_si256(yf, yms);
				yf = _mm256_or_si256(yf,
					_mm256_add_epi32(yfs, yfs));
				yf = mp_set_x8(yf, yp);
				_mm256_storeu_si256(Fz + i, yf);
			}
		} else {
			for (size_t i = 0; i < n; i ++) {
				uint32_t xf = F[i];
				xf |= (xf & 0x40000000) << 1;
				F[i] = mp_set(*(int32_t *)&xf, p);
			}
		}
	} else {
		uint32_t p0 = PRIMES[0].p;
		uint32_t p0_0i = PRIMES[0].p0i;
		uint32_t z0 = mp_half(PRIMES[0].R2, p0);
		uint32_t p1 = PRIMES[1].p;
		uint32_t p1_0i = PRIMES[1].p0i;
		uint32_t z1 = mp_half(PRIMES[1].R2, p1);
		if (logn >= 3) {
			__m256i yp0 = _mm256_set1_epi32(p0);
			__m256i yp0_0i = _mm256_set1_epi32(p0_0i);
			__m256i yz0 = _mm256_set1_epi32(z0);
			__m256i yp1 = _mm256_set1_epi32(p1);
			__m256i yp1_0i = _mm256_set1_epi32(p1_0i);
			__m256i yz1 = _mm256_set1_epi32(z1);
			__m256i *Fz = (__m256i *)F;
			__m256i yms = _mm256_set1_epi32(0x40000000);
			for (size_t i = 0; i < (n >> 3); i ++) {
				__m256i yl, yh, tl0, th0, r0, tl1, th1, r1;

				yl = _mm256_loadu_si256(Fz + i);
				yh = _mm256_loadu_si256(Fz + i + (n >> 3));
				yh = _mm256_or_si256(yh, _mm256_slli_epi32(
					_mm256_and_si256(yh, yms), 1));
				tl0 = _mm256_sub_epi32(yl, _mm256_andnot_si256(
					_mm256_cmpgt_epi32(yp0, yl), yp0));
				th0 = mp_set_x8(yh, yp0);
				r0 = mp_add_x8(tl0,
					mp_mmul_x8(th0, yz0, yp0, yp0_0i), yp0);
				tl1 = _mm256_sub_epi32(yl, _mm256_andnot_si256(
					_mm256_cmpgt_epi32(yp1, yl), yp1));
				th1 = mp_set_x8(yh, yp1);
				r1 = mp_add_x8(tl1,
					mp_mmul_x8(th1, yz1, yp1, yp1_0i), yp1);
				_mm256_storeu_si256(Fz + i, r0);
				_mm256_storeu_si256(Fz + i + (n >> 3), r1);
			}
		} else {
			for (size_t i = 0; i < n; i ++) {
				uint32_t xl, xh, yl0, yh0, r0, yl1, yh1, r1;

				xl = F[i];
				xh = F[i + n] | ((F[i + n] & 0x40000000) << 1);
				yl0 = xl - (p0 & ~tbmask(xl - p0));
				yh0 = mp_set(*(int32_t *)&xh, p0);
				r0 = mp_add(yl0,
					mp_mmul(yh0, z0, p0, p0_0i), p0);
				yl1 = xl - (p1 & ~tbmask(xl - p1));
				yh1 = mp_set(*(int32_t *)&xh, p1);
				r1 = mp_add(yl1,
					mp_mmul(yh1, z1, p1, p1_0i), p1);
				F[i] = r0;
				F[i + n] = r1;
			}
		}
	}

	/*
	 * Step 2: for FGlen small primes, convert F to RNS+NTT,
	 * and subtract (2^sc)*ft. The ft polynomial is computed
	 * in RNS+NTT dynamically.
	 */
	for (size_t i = 0; i < FGlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t R3 = mp_mmul(R2, R2, p, p0i);
		avx2_mp_mkgm(logn, gm, PRIMES[i].g, p, p0i);

		/*
		 * k <- (2^sc)*k (and into NTT).
		 */
		uint32_t scv = mp_mmul(
			(uint32_t)1 << (sc & 31), R2, p, p0i);
		for (uint32_t m = sc >> 5; m > 0; m --) {
			scv = mp_mmul(scv, R2, p, p0i);
		}
		for (size_t j = 0; j < n; j ++) {
			uint32_t x = mp_set(*(int32_t *)&k[j], p);
			k[j] = mp_mmul(scv, x, p, p0i);
		}
		avx2_mp_NTT(logn, k, gm, p, p0i);

		/*
		 * Convert F to NTT.
		 */
		uint32_t *Fu = F + (i << logn);
		avx2_mp_NTT(logn, Fu, gm, p, p0i);

		/*
		 * Given the top-level f, we obtain ft = N(f) (the f at
		 * depth 1) with:
		 *    f = f_e(X^2) + X*f_o(X^2)
		 * with f_e and f_o being modulo X^n+1. Then:
		 *    N(f) = f_e^2 - X*f_o^2
		 * The NTT representation of X is obtained from the gm[] tab:
		 *    NTT(X)[2*j + 0] = gm[j + n/2]
		 *    NTT(X)[2*j + 1] = -NTT(X)[2*j + 0]
		 * Note that the values in gm[] are in Montgomery
		 * representation.
		 */
		for (size_t j = 0; j < n; j ++) {
			t1[j] = mp_set(f[(j << 1) + 0], p);
			t2[j] = mp_set(f[(j << 1) + 1], p);
		}
		avx2_mp_NTT(logn, t1, gm, p, p0i);
		avx2_mp_NTT(logn, t2, gm, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			uint32_t xe0 = t1[(j << 1) + 0];
			uint32_t xe1 = t1[(j << 1) + 1];
			uint32_t xo0 = t2[(j << 1) + 0];
			uint32_t xo1 = t2[(j << 1) + 1];
			uint32_t xv0 = gm[hn + j];
			uint32_t xv1 = p - xv0;
			xe0 = mp_mmul(xe0, xe0, p, p0i);
			xe1 = mp_mmul(xe1, xe1, p, p0i);
			xo0 = mp_mmul(xo0, xo0, p, p0i);
			xo1 = mp_mmul(xo1, xo1, p, p0i);
			uint32_t xf0 = mp_sub(xe0,
				mp_mmul(xo0, xv0, p, p0i), p);
			uint32_t xf1 = mp_sub(xe1,
				mp_mmul(xo1, xv1, p, p0i), p);

			uint32_t xkf0 = mp_mmul(
				mp_mmul(xf0, k[(j << 1) + 0], p, p0i),
				R3, p, p0i);
			uint32_t xkf1 = mp_mmul(
				mp_mmul(xf1, k[(j << 1) + 1], p, p0i),
				R3, p, p0i);
			Fu[(j << 1) + 0] = mp_sub(Fu[(j << 1) + 0], xkf0, p);
			Fu[(j << 1) + 1] = mp_sub(Fu[(j << 1) + 1], xkf1, p);
		}

		/*
		 * Convert F back to RNS.
		 */
		avx2_mp_mkigm(logn, t1, PRIMES[i].ig, p, p0i);
		avx2_mp_iNTT(logn, Fu, t1, p, p0i);

		/*
		 * We replaced k (plain 32-bit) with (2^sc)*k (NTT). We must
		 * put it back to its initial value if there should be another
		 * iteration.
		 */
		if ((i + 1) < FGlen) {
			mp_iNTT(logn, k, t1, p, p0i);
			scv = (uint32_t)1 << (-sc & 31);
			for (uint32_t m = sc >> 5; m > 0; m --) {
				scv = mp_mmul(scv, 1, p, p0i);
			}
			for (size_t j = 0; j < n; j ++) {
				k[j] = (uint32_t)mp_norm(
					mp_mmul(scv, k[j], p, p0i), p);
			}
		}
	}

	/*
	 * Output F is in RNS (non-NTT), but we want plain integers.
	 */
	if (FGlen == 1) {
		uint32_t p = PRIMES[0].p;
		for (size_t i = 0; i < n; i ++) {
			F[i] = (uint32_t)mp_norm(F[i], p) & 0x7FFFFFFF;
		}
	} else {
		uint32_t p0 = PRIMES[0].p;
		uint32_t p1 = PRIMES[1].p;
		uint32_t p1_0i = PRIMES[1].p0i;
		uint32_t s = PRIMES[1].s;
		uint64_t pp = (uint64_t)p0 * (uint64_t)p1;
		uint64_t hpp = pp >> 1;
		for (size_t i = 0; i < n; i ++) {
			/*
			 * Apply CRT with two primes on the coefficient of F.
			 */
			uint32_t x0 = F[i];      /* mod p0 */
			uint32_t x1 = F[i + n];  /* mod p1 */
			uint32_t x0m1 = x0 - (p1 & ~tbmask(x0 - p1));
			uint32_t y = mp_mmul(
				mp_sub(x1, x0m1, p1), s, p1, p1_0i);
			uint64_t z = (uint64_t)x0 + (uint64_t)p0 * (uint64_t)y;
			z -= pp & -((hpp - z) >> 63);
			F[i] = (uint32_t)z & 0x7FFFFFFF;
			F[i + n] = (uint32_t)(z >> 31) & 0x7FFFFFFF;
		}
	}
}
#endif

/* see kgen_inner.h */
uint32_t
poly_sqnorm(unsigned logn, const int8_t *f)
{
	size_t n = (size_t)1 << logn;
	uint32_t s = 0;
	for (size_t i = 0; i < n; i ++) {
		int32_t x = f[i];
		s += (uint32_t)(x * x);
	}
	return s;
}

#if FNDSA_AVX2
TARGET_AVX2
uint32_t
avx2_poly_sqnorm(unsigned logn, const int8_t *f)
{
	size_t n = (size_t)1 << logn;
	if (logn >= 4) {
		__m256i ys = _mm256_setzero_si256();
		__m256i ym = _mm256_set1_epi32(0xFFFF);
		for (size_t i = 0; i < n; i += 16) {
			__m128i x = _mm_loadu_si128((const __m128i *)(f + i));
			__m256i y = _mm256_cvtepi8_epi16(x);
			y = _mm256_mullo_epi16(y, y);
			__m256i y0 = _mm256_and_si256(y, ym);
			__m256i y1 = _mm256_srli_epi32(y, 16);
			ys = _mm256_add_epi32(ys, _mm256_add_epi32(y0, y1));
		}
		ys = _mm256_add_epi32(ys, _mm256_srli_epi64(ys, 32));
		ys = _mm256_add_epi32(ys, _mm256_bsrli_epi128(ys, 8));
		return (uint32_t)_mm_cvtsi128_si32(
			_mm_add_epi32(
				_mm256_castsi256_si128(ys),
				_mm256_extracti128_si256(ys, 1)));
	}
	uint32_t s = 0;
	for (size_t i = 0; i < n; i ++) {
		int32_t x = f[i];
		s += (uint32_t)(x * x);
	}
	return s;
}
#endif
