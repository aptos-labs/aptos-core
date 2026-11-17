/*
 * NTRU equation.
 */

#include "kgen_inner.h"

#define Q   12289

static const uint16_t MAX_BL_SMALL[11] = {
	1, 1, 2, 3, 4, 8, 14, 27, 53, 104, 207
};
static const uint16_t MAX_BL_LARGE[10] = {
	1, 2, 3, 6, 11, 21, 40, 78, 155, 308
};
static const uint16_t WORD_WIN[10] = {
	1, 1, 2, 2, 2, 3, 3, 4, 5, 7
};
static const uint16_t MIN_SAVE_FG[11] = {
	0, 0, 1, 2, 2, 2, 2, 2, 3, 3, 4
};

/* Convert source f and g into RNS+NTT, at the start of the provided tmp[]
   (one word per coefficient). */
static void
make_fg_zero(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	uint32_t *ft = tmp;
	uint32_t *gt = ft + n;
	uint32_t *gm = gt + n;
	uint32_t p = PRIMES[0].p;
	uint32_t p0i = PRIMES[0].p0i;
	poly_mp_set_small(logn, ft, f, p);
	poly_mp_set_small(logn, gt, g, p);
	mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	mp_NTT(logn, ft, gm, p, p0i);
	mp_NTT(logn, gt, gm, p, p0i);
}

#if FNDSA_AVX2
TARGET_AVX2
static void
avx2_make_fg_zero(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	uint32_t *ft = tmp;
	uint32_t *gt = ft + n;
	uint32_t *gm = gt + n;
	uint32_t p = PRIMES[0].p;
	uint32_t p0i = PRIMES[0].p0i;
	avx2_poly_mp_set_small(logn, ft, f, p);
	avx2_poly_mp_set_small(logn, gt, g, p);
	avx2_mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	avx2_mp_NTT(logn, ft, gm, p, p0i);
	avx2_mp_NTT(logn, gt, gm, p, p0i);
}
#endif

/* One step of computing (f,g) at a given depth.
     Input: (f,g) of degree 2^(logn_top-depth)
     Output: (f',g') of degree 2^(logn_top-(depth+1))
   Input and output values are at the start of tmp[], in RNS+NTT notation.
  
   RAM USAGE: 3*(2^logn_top) (at most)
   (assumptions: max_bl_small[0] = max_bl_small[1] = 1, max_bl_small[2] = 2) */
static void
make_fg_step(unsigned logn_top, unsigned depth, uint32_t *tmp)
{
	unsigned logn = logn_top - depth;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	size_t slen = MAX_BL_SMALL[depth];
	size_t tlen = MAX_BL_SMALL[depth + 1];

	/* Layout:
	     fd    output f' (hn*tlen)
	     gd    output g' (hn*tlen)
	     fs    source (n*slen)
	     gs    source (n*slen)
	     t1    NTT support (n)
	     t2    extra (max(n, slen - n))  */
	uint32_t *fd = tmp;
	uint32_t *gd = fd + hn * tlen;
	uint32_t *fs = gd + hn * tlen;
	uint32_t *gs = fs + n * slen;
	uint32_t *t1 = gs + n * slen;
	uint32_t *t2 = t1 + n;
	memmove(fs, tmp, 2 * n * slen * sizeof *tmp);

	/* First slen words: we use the input values directly, and apply
	   inverse NTT as we go, so that we get the sources in RNS (non-NTT). */
	uint32_t *xf = fs;
	uint32_t *xg = gs;
	uint32_t *yf = fd;
	uint32_t *yg = gd;
	for (size_t i = 0; i < slen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		for (size_t j = 0; j < hn; j ++) {
			yf[j] = mp_mmul(
				mp_mmul(xf[2 * j], xf[2 * j + 1], p, p0i),
				R2, p, p0i);
			yg[j] = mp_mmul(
				mp_mmul(xg[2 * j], xg[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		mp_mkigm(logn, t1, PRIMES[i].ig, p, p0i);
		mp_iNTT(logn, xf, t1, p, p0i);
		mp_iNTT(logn, xg, t1, p, p0i);
		xf += n;
		xg += n;
		yf += hn;
		yg += hn;
	}

	/* Now that fs and gs are in RNS, rebuild their plain integer
	   coefficients. */
	zint_rebuild_CRT(fs, slen, n, 2, 1, t1);

	/* Remaining output words. */
	for (size_t i = slen; i < tlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t Rx = mp_Rx31(slen, p, p0i, R2);
		mp_mkgm(logn, t1, PRIMES[i].g, p, p0i);
		for (size_t j = 0; j < n; j ++) {
			t2[j] = zint_mod_small_signed(
				fs + j, slen, n, p, p0i, R2, Rx);
		}
		mp_NTT(logn, t2, t1, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			yf[j] = mp_mmul(
				mp_mmul(t2[2 * j], t2[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		yf += hn;
		for (size_t j = 0; j < n; j ++) {
			t2[j] = zint_mod_small_signed(
				gs + j, slen, n, p, p0i, R2, Rx);
		}
		mp_NTT(logn, t2, t1, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			yg[j] = mp_mmul(
				mp_mmul(t2[2 * j], t2[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		yg += hn;
	}
}

#if FNDSA_AVX2
TARGET_AVX2
static void
avx2_make_fg_step(unsigned logn_top, unsigned depth, uint32_t *tmp)
{
	unsigned logn = logn_top - depth;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	size_t slen = MAX_BL_SMALL[depth];
	size_t tlen = MAX_BL_SMALL[depth + 1];

	/* Layout:
	     fd    output f' (hn*tlen)
	     gd    output g' (hn*tlen)
	     fs    source (n*slen)
	     gs    source (n*slen)
	     t1    NTT support (n)
	     t2    extra (max(n, slen - n))  */
	uint32_t *fd = tmp;
	uint32_t *gd = fd + hn * tlen;
	uint32_t *fs = gd + hn * tlen;
	uint32_t *gs = fs + n * slen;
	uint32_t *t1 = gs + n * slen;
	uint32_t *t2 = t1 + n;
	memmove(fs, tmp, 2 * n * slen * sizeof *tmp);

	/* First slen words: we use the input values directly, and apply
	   inverse NTT as we go, so that we get the sources in RNS (non-NTT). */
	uint32_t *xf = fs;
	uint32_t *xg = gs;
	uint32_t *yf = fd;
	uint32_t *yg = gd;
	for (size_t i = 0; i < slen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		for (size_t j = 0; j < hn; j ++) {
			yf[j] = mp_mmul(
				mp_mmul(xf[2 * j], xf[2 * j + 1], p, p0i),
				R2, p, p0i);
			yg[j] = mp_mmul(
				mp_mmul(xg[2 * j], xg[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		mp_mkigm(logn, t1, PRIMES[i].ig, p, p0i);
		mp_iNTT(logn, xf, t1, p, p0i);
		mp_iNTT(logn, xg, t1, p, p0i);
		xf += n;
		xg += n;
		yf += hn;
		yg += hn;
	}

	/* Now that fs and gs are in RNS, rebuild their plain integer
	   coefficients. */
	avx2_zint_rebuild_CRT(fs, slen, n, 2, 1, t1);

	/* Remaining output words. */
	for (size_t i = slen; i < tlen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t Rx = mp_Rx31(slen, p, p0i, R2);
		avx2_mp_mkgm(logn, t1, PRIMES[i].g, p, p0i);
		if (logn >= 3) {
			__m256i yp = _mm256_set1_epi32(p);
			__m256i yp0i = _mm256_set1_epi32(p0i);
			__m256i yR2 = _mm256_set1_epi32(R2);
			__m256i yRx = _mm256_set1_epi32(Rx);
			for (size_t j = 0; j < n; j += 8) {
				__m256i yt = avx2_zint_mod_small_signed_x8(
					fs + j, slen, n, yp, yp0i, yR2, yRx);
				_mm256_storeu_si256((__m256i *)(t2 + j), yt);
			}
			avx2_mp_NTT(logn, t2, t1, p, p0i);
			for (size_t j = 0; j < hn; j += 4) {
				__m256i yt = _mm256_loadu_si256(
					(__m256i *)(t2 + (2 * j)));
				yt = mp_mmul_x4(yt,
					_mm256_srli_epi64(yt, 32), yp, yp0i);
				yt = mp_mmul_x4(yt, yR2, yp, yp0i);
				yt = _mm256_shuffle_epi32(yt, 0xD8);
				yt = _mm256_permute4x64_epi64(yt, 0xD8);
				_mm_storeu_si128((__m128i *)(yf + j),
					_mm256_castsi256_si128(yt));
			}
			yf += hn;
			for (size_t j = 0; j < n; j += 8) {
				__m256i yt = avx2_zint_mod_small_signed_x8(
					gs + j, slen, n, yp, yp0i, yR2, yRx);
				_mm256_storeu_si256((__m256i *)(t2 + j), yt);
			}
			avx2_mp_NTT(logn, t2, t1, p, p0i);
			for (size_t j = 0; j < hn; j += 4) {
				__m256i yt = _mm256_loadu_si256(
					(__m256i *)(t2 + (2 * j)));
				yt = mp_mmul_x4(yt,
					_mm256_srli_epi64(yt, 32), yp, yp0i);
				yt = mp_mmul_x4(yt, yR2, yp, yp0i);
				yt = _mm256_shuffle_epi32(yt, 0xD8);
				yt = _mm256_permute4x64_epi64(yt, 0xD8);
				_mm_storeu_si128((__m128i *)(yg + j),
					_mm256_castsi256_si128(yt));
			}
			yg += hn;
			continue;
		}
		for (size_t j = 0; j < n; j ++) {
			t2[j] = zint_mod_small_signed(
				fs + j, slen, n, p, p0i, R2, Rx);
		}
		avx2_mp_NTT(logn, t2, t1, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			yf[j] = mp_mmul(
				mp_mmul(t2[2 * j], t2[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		yf += hn;
		for (size_t j = 0; j < n; j ++) {
			t2[j] = zint_mod_small_signed(
				gs + j, slen, n, p, p0i, R2, Rx);
		}
		avx2_mp_NTT(logn, t2, t1, p, p0i);
		for (size_t j = 0; j < hn; j ++) {
			yg[j] = mp_mmul(
				mp_mmul(t2[2 * j], t2[2 * j + 1], p, p0i),
				R2, p, p0i);
		}
		yg += hn;
	}
}
#endif

/* Compute (f,g) at a specified depth, in RNS+NTT notation.
   Computed values are stored at the start of the provided tmp[] (slen
   words per coefficient).
  
   This function is for depth < logn_top. For the deepest layer, use
   make_fg_deepest().
  
   RAM USAGE: 3*(2^logn_top) */
static void
make_fg_intermediate(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	unsigned depth, uint32_t *tmp)
{
	make_fg_zero(logn_top, f, g, tmp);
	for (unsigned d = 0; d < depth; d ++) {
		make_fg_step(logn_top, d, tmp);
	}
}

#if FNDSA_AVX2
TARGET_AVX2
static void
avx2_make_fg_intermediate(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	unsigned depth, uint32_t *tmp)
{
	avx2_make_fg_zero(logn_top, f, g, tmp);
	for (unsigned d = 0; d < depth; d ++) {
		avx2_make_fg_step(logn_top, d, tmp);
	}
}
#endif

/* Compute (f,g) at the deepest level (i.e. get Res(f,X^n+1) and
   Res(g,X^n+1)). Intermediate (f,g) values (below the save threshold)
   are copied at the end of tmp (of size save_off words). */
static void
make_fg_deepest(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *tmp, size_t sav_off)
{
	make_fg_zero(logn_top, f, g, tmp);
	for (unsigned d = 0; d < logn_top; d ++) {
		make_fg_step(logn_top, d, tmp);

		/* make_fg_step() computes the (f,g) for depth d+1; we
		   save that value if d+1 is at least at the save
		   threshold, but is not the deepest level. */
		unsigned d2 = d + 1;
		if (d2 < logn_top && d2 >= MIN_SAVE_FG[logn_top]) {
			size_t slen = MAX_BL_SMALL[d2];
			size_t fglen = slen << (logn_top + 1 - d2);
			sav_off -= fglen;
			memmove(tmp + sav_off, tmp, fglen * sizeof *tmp);
		}
	}
}

#if FNDSA_AVX2
TARGET_AVX2
static void
avx2_make_fg_deepest(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *tmp, size_t sav_off)
{
	avx2_make_fg_zero(logn_top, f, g, tmp);
	for (unsigned d = 0; d < logn_top; d ++) {
		avx2_make_fg_step(logn_top, d, tmp);

		/* make_fg_step() computes the (f,g) for depth d+1; we
		   save that value if d+1 is at least at the save
		   threshold, but is not the deepest level. */
		unsigned d2 = d + 1;
		if (d2 < logn_top && d2 >= MIN_SAVE_FG[logn_top]) {
			size_t slen = MAX_BL_SMALL[d2];
			size_t fglen = slen << (logn_top + 1 - d2);
			sav_off -= fglen;
			memmove(tmp + sav_off, tmp, fglen * sizeof *tmp);
		}
	}
}
#endif

/* Error code: no error (so far) */
#define SOLVE_OK           0

/* Error code: GCD(Res(f,X^n+1), Res(g,X^n+1)) != 1 */
#define SOLVE_ERR_GCD      -1

/* Error code: reduction error (NTRU equation no longer fulfilled) */
#define SOLVE_ERR_REDUCE   -2

/* Error code: output (F,G) coefficients are off-limits */
#define SOLVE_ERR_LIMIT    -3

/* Offset in tmp[] for saving the intermediate (f,g) values. It is
   expressed in 32-bit words, and can use 'logn_top' to access the
   top-level degree. */
#define FG_SAVE_OFFSET   ((size_t)5 << logn_top)

/* Solve the NTRU equation at the deepest level. This computes the
   integers F and G such that Res(f,X^n+1)*G - Res(g,X^n+1)*F = q.
   F is written into tmp[].
  
   Returned value: 0 on success, a negative error code otherwise.
  
   RAM USAGE: max(3*(2^logn_top), 8*max_bl_small[depth]) */
static int
solve_NTRU_deepest(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp)
{
	/* Get (f,g) at the deepest level (i.e. Res(f,X^n+1) and Res(g,X^n+1)).
	   Obtained (f,g) are in RNS+NTT (since degree n = 1, this is
	   equivalent to RNS). */
	make_fg_deepest(logn_top, f, g, tmp, FG_SAVE_OFFSET);

	/* Reorganize memory:
	      Fp   output F (len)
	      Gp   output G (len)
	      fp   Res(f,X^n+1) (len)
	      gp   Res(g,X^n+1) (len)
	      t1   rest of temporary */
	size_t len = MAX_BL_SMALL[logn_top];
	uint32_t *Fp = tmp;
	uint32_t *Gp = Fp + len;
	uint32_t *fp = Gp + len;
	uint32_t *gp = fp + len;
	uint32_t *t1 = gp + len;
	memmove(fp, tmp, 2 * len * sizeof *tmp);

	/* Convert back the resultants into plain integers. */
	zint_rebuild_CRT(fp, len, 1, 2, 0, t1);

	/* Apply the binary GCD to get a solution (F,G) such that:
	     f*G - g*F = 1  */
	if (!zint_bezout(Gp, Fp, fp, gp, len, t1)) {
		return SOLVE_ERR_GCD;
	}

	/* Multiply the obtained (F,G) by q to get a proper solution:
	     f*G - g*F = q
	   (Only F is multiplied since G is ultimately discarded.) */
	if (zint_mul_small(Fp, len, Q) != 0) {
		return SOLVE_ERR_REDUCE;
	}
	return SOLVE_OK;
}
#if FNDSA_AVX2
TARGET_AVX2
static int
avx2_solve_NTRU_deepest(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp)
{
	/* Get (f,g) at the deepest level (i.e. Res(f,X^n+1) and Res(g,X^n+1)).
	   Obtained (f,g) are in RNS+NTT (since degree n = 1, this is
	   equivalent to RNS). */
	avx2_make_fg_deepest(logn_top, f, g, tmp, FG_SAVE_OFFSET);

	/* Reorganize memory:
	      Fp   output F (len)
	      Gp   output G (len)
	      fp   Res(f,X^n+1) (len)
	      gp   Res(g,X^n+1) (len)
	      t1   rest of temporary */
	size_t len = MAX_BL_SMALL[logn_top];
	uint32_t *Fp = tmp;
	uint32_t *Gp = Fp + len;
	uint32_t *fp = Gp + len;
	uint32_t *gp = fp + len;
	uint32_t *t1 = gp + len;
	memmove(fp, tmp, 2 * len * sizeof *tmp);

	/* Convert back the resultants into plain integers. */
	avx2_zint_rebuild_CRT(fp, len, 1, 2, 0, t1);

	/* Apply the binary GCD to get a solution (F,G) such that:
	     f*G - g*F = 1  */
	if (!zint_bezout(Gp, Fp, fp, gp, len, t1)) {
		return SOLVE_ERR_GCD;
	}

	/* Multiply the obtained (F,G) by q to get a proper solution:
	     f*G - g*F = q
	   (Only F is multiplied since G is ultimately discarded.) */
	if (zint_mul_small(Fp, len, Q) != 0) {
		return SOLVE_ERR_REDUCE;
	}
	return SOLVE_OK;
}
#endif

/* We use poly_sub_scaled() when log(n) < MIN_LOGN_FGNTT, and
   poly_sub_scaled_ntt() when log(n) >= MIN_LOGN_FGNTT. The NTT variant
   is faster at large degrees, but not at small degrees. */
#define MIN_LOGN_FGNTT   4

#if FNDSA_AVX2
/* The AVX2 implementation requires MIN_LOGN_FGNTT >= 3 */
#if MIN_LOGN_FGNTT < 3
#error Incorrect MIN_LOGN_FGNTT value
#endif
#endif

/* Solving the NTRU equation, intermediate level.
   Input is (F,G) from one level deeper (half-degree), in plain
   representation, at the start of tmp[]; output is (F,G) from this
   level, written at the start of tmp[].
  
   Returned value: 0 on success, a negative error code otherwise. */
static int
solve_NTRU_intermediate(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	unsigned depth, uint32_t *restrict tmp)
{
	unsigned logn = logn_top - depth;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	/* slen   size for (f,g) at this level (also size of output F)
	   llen   size for unreduced F at this level
	   dlen   size for input F from deeper level
	   Note: we always have llen >= dlen */
	size_t slen = MAX_BL_SMALL[depth];
	size_t llen = MAX_BL_LARGE[depth];
	size_t dlen = MAX_BL_SMALL[depth + 1];

	/* Fd   F from deeper level (dlen*hn)
	   fgt  f,g from this level (2*slen*n)  */
	uint32_t *Fd = tmp;
	uint32_t *fgt = Fd + dlen * hn;

	/* Get (f,g) for this level (in RNS+NTT). */
	if (depth < MIN_SAVE_FG[logn_top]) {
		make_fg_intermediate(logn_top, f, g, depth, fgt);
	} else {
		uint32_t *sav_fg = tmp + FG_SAVE_OFFSET;
		for (unsigned d = MIN_SAVE_FG[logn_top];
			d <= depth; d ++)
		{
			sav_fg -= MAX_BL_SMALL[d] << (logn_top + 1 - d);
		}
		memmove(fgt, sav_fg, 2 * slen * n * sizeof *fgt);
	}

	/* Move buffers so that we have room for the unreduced (F,G) at
	   this level.
	     Ft   F from this level (unreduced) (llen*n)
	     ft   f from this level (slen*n)
	     gt   g from this level (slen*n)
	     Fd   F from deeper level (dlen*hn)  */
	uint32_t *Ft = tmp;
	uint32_t *ft = Ft + llen * n;
	uint32_t *gt = ft + slen * n;
	Fd = gt + slen * n;
	uint32_t *t1 = Fd + dlen * hn;
	memmove(ft, fgt, 2 * n * slen * sizeof *ft);
	memmove(Fd, tmp, hn * dlen * sizeof *tmp);

	/* Convert Fd to RNS, with output temporarily stored in Ft. Fd
	   has degree hn only; we store the values for each modulus p in
	   the _last_ hn slots of the n-word line for that modulus. */
	for (size_t i = 0; i < llen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t Rx = mp_Rx31((unsigned)dlen, p, p0i, R2);
		uint32_t *xt = Ft + i * n + hn;
		for (size_t j = 0; j < hn; j ++) {
			xt[j] = zint_mod_small_signed(Fd + j, dlen, hn,
				p, p0i, R2, Rx);
		}
	}

	/* Fd is no longer needed. */
	t1 = Fd;

	/* Compute F (unreduced) modulo sufficiently many small primes.
	   We also un-NTT (f,g) as we go; when slen primes have been
	   processed, we obtain (f,g) in RNS, and we apply the CRT to
	   get (f,g) in plain representation. */
	for (size_t i = 0; i < llen; i ++) {
		/* If we have processed exactly slen primes, then (f,g)
		   are in RNS, and we can rebuild them. */
		if (i == slen) {
			zint_rebuild_CRT(ft, slen, n, 2, 1, t1);
		}

		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;

		/* Memory layout: we keep Ft, ft and gt; we append:
		     gm    NTT support (n)
		     igm   iNTT support (n)
		     gx    temporary g mod p (NTT) (n)  */
		uint32_t *gm = t1;
		uint32_t *igm = gm + n;
		uint32_t *gx = igm + n;
		mp_mkgmigm(logn, gm, igm, PRIMES[i].g, PRIMES[i].ig, p, p0i);
		if (i < slen) {
			memcpy(gx, gt + i * n, n * sizeof *gx);
			mp_iNTT(logn, ft + i * n, igm, p, p0i);
			mp_iNTT(logn, gt + i * n, igm, p, p0i);
		} else {
			uint32_t Rx = mp_Rx31((unsigned)slen, p, p0i, R2);
			for (size_t j = 0; j < n; j ++) {
				gx[j] = zint_mod_small_signed(gt + j, slen, n,
					p, p0i, R2, Rx);
			}
			mp_NTT(logn, gx, gm, p, p0i);
		}

		/* We have F from deeper level in Ft, in RNS. We apply
		   the NTT modulo p. */
		uint32_t *Fe = Ft + i * n;
		mp_NTT(logn - 1, Fe + hn, gm, p, p0i);

		/* Compute F (unreduced) modulo p. */
		for (size_t j = 0; j < hn; j ++) {
			uint32_t ga = gx[(j << 1) + 0];
			uint32_t gb = gx[(j << 1) + 1];
			uint32_t mFp = mp_mmul(Fe[j + hn], R2, p, p0i);
			Fe[(j << 1) + 0] = mp_mmul(gb, mFp, p, p0i);
			Fe[(j << 1) + 1] = mp_mmul(ga, mFp, p, p0i);
		}

		/* We want the new F in RNS only (no NTT). */
		mp_iNTT(logn, Fe, igm, p, p0i);
	}

	/* We no longer need g. */
	t1 = gt;

	/* Edge case: if slen == llen, then we have not rebuilt f
	   into plain representation yet, so we do it now. */
	if (slen == llen) {
		zint_rebuild_CRT(ft, slen, n, 1, 1, t1);
	}

	/* We now have the unreduced F in RNS. We rebuild its
	   plain representation. */
	zint_rebuild_CRT(Ft, llen, n, 1, 1, t1);

	/* We now reduce these F with Babai's nearest plane
	   algorithm. The reduction conceptually goes as follows:
	     k <- round((F*adj(f) + G*adj(g))/(f*adj(f) + g*adj(g)))
	     (F, G) <- (F - k*f, G - k*g)
	   We only have F; however, G is such that:
	     f*G - g*F = q
	   hence:
	     G = (q + g*F)/f
	   which we can move into the expression of k, which simplifies into:
	     k = round(F/f + q*adj(g)/(f*(f*adj(f) + g*adj(g))))
	   The second part only depends on f and g; moreover, it is
	   heuristically negligible, i.e. we can compute an approximate
	   value of k as:
	     k = round(F/f)
	   In practice, this approximation is good enough for our purposes,
	   which is to let the algorithm keep going (at the end, a less
	   approximate k is used to finish up the values).

	   We use fixed-point approximations of f and F to get a value k
	   as a small polynomial with scaling; we then apply k on the
	   full-width polynomial. Each iteration "shaves" a few bits off F.

	   We apply the process sufficiently many times to reduce F
	   to the size of f with a reasonable probability of success.
	   Since we want full constant-time processing, the number of
	   iterations and the accessed slots work on some assumptions on
	   the sizes of values (sizes have been measured over many samples,
	   and a margin of 5 times the standard deviation). */

	/* If depth is at least 2, and we will use the NTT to subtract
	   k*f from F, then we will need to convert f to NTT over
	   slen+1 words, which requires an extra word to ft. */
	int use_sub_ntt = (depth > 1 && logn >= MIN_LOGN_FGNTT);
	if (use_sub_ntt) {
		t1 += n;
	}

	/* New layout:
	     Ft    F from this level (unreduced) (llen*n)
	     ft    f from this level (slen*n) (+n if use_sub_ntt)
	     rt3   (n fxr = 2*n) */
	fxr *rt3 = (fxr *)t1;

	/* We consider only the top rlen words of f. */
	size_t rlen = WORD_WIN[depth];
	if (rlen > slen) {
		rlen = slen;
	}
	size_t blen = slen - rlen;
	uint32_t *ftb = ft + blen * n;
	uint32_t scale_fg = 31 * (uint32_t)blen;
	uint32_t scale_FG = 31 * (uint32_t)llen;

	/* Convert f into fixed-point approximations, into rt3. It is scaled
	   down by 2^(scale_fg + scale_x). scale_fg is public (it depends
	   only on the recursion depth), but scale_x comes from a measurement
	   on the actual coefficient values of f and is thus secret.

	   The value scale_x is adjusted so that the largest coefficient is
	   close to, but lower than, some limit t (in absolute value). The
	   limit t is chosen so that f*adj(f) does not overflow, i.e. all
	   coefficients must remain below 2^31.

	   Let n be the degree; we know that n <= 2^10. The squared norm
	   of a polynomial is the sum of the squared norms of the
	   coefficients, with the squared norm of a complex number being
	   the product of that number with its complex conjugate. If all
	   coefficients of f are less than t (in absolute value), then
	   the squared norm of f is less than n*t^2. The squared norm of
	   FFT(f) (f in FFT representation) is exactly n times the
	   squared norm of f, so this leads to n^2*t^2 as a maximum
	   bound. adj(f) has the same norm as f. This implies that each
	   complex coefficient of FFT(f) has a maximum squared norm of
	   n^2*t^2 (with a maximally imbalanced polynomial with all
	   coefficient but one being zero). The computation of f*adj(f)
	   exactly is, in FFT representation, the product of each
	   coefficient with its conjugate; thus, the coefficients of
	   f*adj(f), in FFT representation, are at most n^2*t^2.

	   Since we want the coefficients of f*adj(f) not to exceed
	   2^31, we need n^2*t^2 <= 2^31, i.e. n*t <= 2^15.5. We can adjust t
	   accordingly (called scale_t in the code below). We also need to
	   take care that t must not exceed scale_x. Approximation of f and
	   g are extracted with scale scale_fg + scale_x - scale_t, and
	   later fixed by dividing them by 2^scale_t. */
	uint32_t scale_x = poly_max_bitlength(logn, ftb, rlen);
	uint32_t scale_t = 15 - logn;
	scale_t ^= (scale_t ^ scale_x) & tbmask(scale_x - scale_t);
	uint32_t scdiff = scale_x - scale_t;

	poly_big_to_fixed(logn, rt3, ftb, rlen, scdiff);

	/* rt3 <- adj(f)/(f*adj(f))  (FFT)  */
	vect_FFT(logn, rt3);
	vect_inv_mul2e_fft(logn, rt3, scale_t);

	/* New layout:
	     Ft    F from this level (unreduced) (llen*n)
	     ft    f from this level (slen*n) (+n if use_sub_ntt)
	     rt3   (n fxr = 2*n)
	     rt1   (n fxr = 2*n)     |   k    (n)
	                             |   t2   (3*n)  */
	fxr *rt1 = rt3 + n;
	int32_t *k = (int32_t *)rt1;
	uint32_t *t2 = (uint32_t *)(k + n);

	/* If we are going to use poly_sub_scaled_ntt(), then we convert
	   f to the NTT representation. Since poly_sub_scaled_ntt()
	   itself will use more than n*(slen+2) words in t2[], we can do
	   the same here. */
	if (use_sub_ntt) {
		uint32_t *gm = t2;
		uint32_t *tn = gm + n;
		for (size_t i = 0; i <= slen; i ++) {
			uint32_t p = PRIMES[i].p;
			uint32_t p0i = PRIMES[i].p0i;
			uint32_t R2 = PRIMES[i].R2;
			uint32_t Rx = mp_Rx31((unsigned)slen, p, p0i, R2);
			mp_mkgm(logn, gm, PRIMES[i].g, p, p0i);
			for (size_t j = 0; j < n; j ++) {
				tn[j] = zint_mod_small_signed(
					ft + j, slen, n, p, p0i, R2, Rx);
			}
			mp_NTT(logn, tn, gm, p, p0i);
			tn += n;
		}
		tn = gm + n;
		memmove(ft, tn, (slen + 1) * n * sizeof *tn);
	}

	/* Reduce F repeatedly. */
	size_t FGlen = llen;
	uint32_t reduce_bits;
	switch (logn_top) {
	case 9:  reduce_bits = 9; break;
	case 10: reduce_bits = 8; break;
	default: reduce_bits = 10; break;
	}
	for (;;) {
		/* Convert the current F into fixed-point. We want
		   to apply scaling scale_FG + scale_x. */
		uint32_t tlen, toff;
		DIVREM31(tlen, toff, scale_FG);
		poly_big_to_fixed(logn, rt1,
			Ft + tlen * n, FGlen - tlen, scale_x + toff);

		/* rt1 <- (F*adj(f)) / (f*adj(f)) */
		vect_FFT(logn, rt1);
		vect_mul_fft(logn, rt1, rt3);
		vect_iFFT(logn, rt1);

		/* k <- round(rt1) */
		for (size_t i = 0; i < n; i ++) {
			k[i] = fxr_round(rt1[i]);
		}

		/* f is scaled by scale_fg + scale_x
		   F is scaled by scale_FG + scale_x
		   Thus, k is scaled by scale_FG - scale_fg, which is public. */
		uint32_t scale_k = scale_FG - scale_fg;
		if (depth == 1) {
			poly_sub_kf_scaled_depth1(logn_top, Ft, FGlen,
				(uint32_t *)k, scale_k, f, t2);
		} else if (use_sub_ntt) {
			poly_sub_scaled_ntt(logn, Ft, FGlen, ft, slen,
				k, scale_k, t2);
		} else {
			poly_sub_scaled(logn, Ft, FGlen, ft, slen, k, scale_k);
		}

		/* We now assume that F has shrunk by at least
		   reduce_bits. We adjust FGlen accordinly. */
		if (scale_FG <= scale_fg) {
			break;
		}
		if (scale_FG <= (scale_fg + reduce_bits)) {
			scale_FG = scale_fg;
		} else {
			scale_FG -= reduce_bits;
		}
		while (FGlen > slen
			&& 31 * (FGlen - slen) > scale_FG - scale_fg + 30)
		{
			/* We decrement FGlen; when we do so, we check that
			   it does not damage any of the values, i.e. that the
			   removed words are redundant with the remaining
			   words. In practice, this test reliably catches
			   reduction failures early enough. */
			FGlen --;
			uint32_t *xp = &Ft[(FGlen - 1) << logn];
			for (size_t i = 0; i < n; i ++) {
				uint32_t sw = -(xp[i] >> 30) >> 1;
				if (xp[i + n] != sw) {
					return SOLVE_ERR_REDUCE;
				}
			}
		}
	}

	/* Output F is already in the right place. */
	return SOLVE_OK;
}

#if FNDSA_AVX2
TARGET_AVX2
static int
avx2_solve_NTRU_intermediate(unsigned logn_top,
	const int8_t *restrict f, const int8_t *restrict g,
	unsigned depth, uint32_t *restrict tmp)
{
	unsigned logn = logn_top - depth;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	/* slen   size for (f,g) at this level (also size of output F)
	   llen   size for unreduced F at this level
	   dlen   size for input F from deeper level
	   Note: we always have llen >= dlen */
	size_t slen = MAX_BL_SMALL[depth];
	size_t llen = MAX_BL_LARGE[depth];
	size_t dlen = MAX_BL_SMALL[depth + 1];

	/* Fd   F from deeper level (dlen*hn)
	   ft   f from this level (slen*n)  */
	uint32_t *Fd = tmp;
	uint32_t *fgt = Fd + dlen * hn;

	/* Get (f,g) for this level (in RNS+NTT). */
	if (depth < MIN_SAVE_FG[logn_top]) {
		avx2_make_fg_intermediate(logn_top, f, g, depth, fgt);
	} else {
		uint32_t *sav_fg = tmp + FG_SAVE_OFFSET;
		for (unsigned d = MIN_SAVE_FG[logn_top];
			d <= depth; d ++)
		{
			sav_fg -= MAX_BL_SMALL[d] << (logn_top + 1 - d);
		}
		memmove(fgt, sav_fg, 2 * slen * n * sizeof *fgt);
	}

	/* Move buffers so that we have room for the unreduced (F,G) at
	   this level.
	     Ft   F from this level (unreduced) (llen*n)
	     ft   f from this level (slen*n)
	     gt   g from this level (slen*n)
	     Fd   F from deeper level (dlen*hn)  */
	uint32_t *Ft = tmp;
	uint32_t *ft = Ft + llen * n;
	uint32_t *gt = ft + slen * n;
	Fd = gt + slen * n;
	uint32_t *t1 = Fd + dlen * hn;
	memmove(ft, fgt, 2 * n * slen * sizeof *ft);
	memmove(Fd, tmp, hn * dlen * sizeof *tmp);

	/* Convert Fd to RNS, with output temporarily stored in Ft. Fd
	   has degree hn only; we store the values for each modulus p in
	   the _last_ hn slots of the n-word line for that modulus. */
	for (size_t i = 0; i < llen; i ++) {
		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;
		uint32_t Rx = mp_Rx31((unsigned)dlen, p, p0i, R2);
		uint32_t *xt = Ft + i * n + hn;
		if (logn >= 4) {
			__m256i yp = _mm256_set1_epi32(p);
			__m256i yp0i = _mm256_set1_epi32(p0i);
			__m256i yR2 = _mm256_set1_epi32(R2);
			__m256i yRx = _mm256_set1_epi32(Rx);
			for (size_t j = 0; j < hn; j += 8) {
				_mm256_storeu_si256((__m256i *)(xt + j),
					avx2_zint_mod_small_signed_x8(
						Fd + j, dlen, hn,
						yp, yp0i, yR2, yRx));
			}
		} else {
			for (size_t j = 0; j < hn; j ++) {
				xt[j] = zint_mod_small_signed(Fd + j, dlen, hn,
					p, p0i, R2, Rx);
			}
		}
	}

	/* Fd is no longer needed. */
	t1 = Fd;

	/* Compute F (unreduced) modulo sufficiently many small primes.
	   We also un-NTT (f,g) as we go; when slen primes have been
	   processed, we obtain (f,g) in RNS, and we apply the CRT to
	   get (f,g) in plain representation. */
	for (size_t i = 0; i < llen; i ++) {
		/* If we have processed exactly slen primes, then (f,g)
		   are in RNS, and we can rebuild them. */
		if (i == slen) {
			avx2_zint_rebuild_CRT(ft, slen, n, 2, 1, t1);
		}

		uint32_t p = PRIMES[i].p;
		uint32_t p0i = PRIMES[i].p0i;
		uint32_t R2 = PRIMES[i].R2;

		/* Memory layout: we keep Ft, ft and gt; we append:
		     gm    NTT support (n)
		     igm   iNTT support (n)
		     gx    temporary g mod p (NTT) (n)  */
		uint32_t *gm = t1;
		uint32_t *igm = gm + n;
		uint32_t *gx = igm + n;
		avx2_mp_mkgmigm(logn, gm, igm,
			PRIMES[i].g, PRIMES[i].ig, p, p0i);
		if (i < slen) {
			memcpy(gx, gt + i * n, n * sizeof *gx);
			avx2_mp_iNTT(logn, ft + i * n, igm, p, p0i);
			avx2_mp_iNTT(logn, gt + i * n, igm, p, p0i);
		} else {
			uint32_t Rx = mp_Rx31((unsigned)slen, p, p0i, R2);
			if (logn >= 4) {
				__m256i yp = _mm256_set1_epi32(p);
				__m256i yp0i = _mm256_set1_epi32(p0i);
				__m256i yR2 = _mm256_set1_epi32(R2);
				__m256i yRx = _mm256_set1_epi32(Rx);
				for (size_t j = 0; j < n; j += 8) {
					__m256i yd;
					yd = avx2_zint_mod_small_signed_x8(
						gt + j, slen, n,
						yp, yp0i, yR2, yRx);
					_mm256_storeu_si256(
						(__m256i *)(gx + j), yd);
				}
			} else {
				for (size_t j = 0; j < n; j ++) {
					gx[j] = zint_mod_small_signed(
						gt + j, slen, n,
						p, p0i, R2, Rx);
				}
			}
			avx2_mp_NTT(logn, gx, gm, p, p0i);
		}

		/* We have F from deeper level in Ft, in RNS. We apply
		   the NTT modulo p. */
		uint32_t *Fe = Ft + i * n;
		avx2_mp_NTT(logn - 1, Fe + hn, gm, p, p0i);

		/* Compute F (unreduced) modulo p. */
		if (hn >= 4) {
			__m256i yp = _mm256_set1_epi32(p);
			__m256i yp0i = _mm256_set1_epi32(p0i);
			__m256i yR2 = _mm256_set1_epi32(R2);
			for (size_t j = 0; j < hn; j += 4) {
				__m256i yga = _mm256_loadu_si256(
					(__m256i *)(gx + (j << 1)));
				__m256i ygb = _mm256_srli_epi64(yga, 32);
				__m128i xFe = _mm_loadu_si128(
					(__m128i *)(Fe + j + hn));
				__m256i yFp = _mm256_permute4x64_epi64(
					_mm256_castsi128_si256(xFe), 0x50);
				yFp = _mm256_shuffle_epi32(yFp, 0x30);
				yFp = mp_mmul_x4(yFp, yR2, yp, yp0i);
				__m256i yFe0 = mp_mmul_x4(
					ygb, yFp, yp, yp0i);
				__m256i yFe1 = mp_mmul_x4(
					yga, yFp, yp, yp0i);
				_mm256_storeu_si256((__m256i *)(Fe + (j << 1)),
					_mm256_or_si256(yFe0,
						_mm256_slli_epi64(yFe1, 32)));
			}
		} else {
			for (size_t j = 0; j < hn; j ++) {
				uint32_t ga = gx[(j << 1) + 0];
				uint32_t gb = gx[(j << 1) + 1];
				uint32_t mFp = mp_mmul(Fe[j + hn], R2, p, p0i);
				Fe[(j << 1) + 0] = mp_mmul(gb, mFp, p, p0i);
				Fe[(j << 1) + 1] = mp_mmul(ga, mFp, p, p0i);
			}
		}

		/* We want the new F in RNS only (no NTT). */
		avx2_mp_iNTT(logn, Fe, igm, p, p0i);
	}

	/* We no longer need g. */
	t1 = gt;

	/* Edge case: if slen == llen, then we have not rebuilt f
	   into plain representation yet, so we do it now. */
	if (slen == llen) {
		avx2_zint_rebuild_CRT(ft, slen, n, 1, 1, t1);
	}

	/* We now have the unreduced F in RNS. We rebuild its
	   plain representation. */
	avx2_zint_rebuild_CRT(Ft, llen, n, 1, 1, t1);

	/* We now reduce these F with Babai's nearest plane
	   algorithm. The reduction conceptually goes as follows:
	     k <- round((F*adj(f) + G*adj(g))/(f*adj(f) + g*adj(g)))
	     (F, G) <- (F - k*f, G - k*g)
	   We only have F; however, G is such that:
	     f*G - g*F = q
	   hence:
	     G = (q + g*F)/f
	   which we can move into the expression of k, which simplifies into:
	     k = round(F/f + q*adj(g)/(f*(f*adj(f) + g*adj(g))))
	   The second part only depends on f and g; moreover, it is
	   heuristically negligible, i.e. we can compute an approximate
	   value of k as:
	     k = round(F/f)
	   In practice, this approximation is good enough for our purposes,
	   which is to let the algorithm keep going (at the end, a less
	   approximate k is used to finish up the values).

	   We use fixed-point approximations of f and F to get a value k
	   as a small polynomial with scaling; we then apply k on the
	   full-width polynomial. Each iteration "shaves" a few bits off F.

	   We apply the process sufficiently many times to reduce F
	   to the size of f with a reasonable probability of success.
	   Since we want full constant-time processing, the number of
	   iterations and the accessed slots work on some assumptions on
	   the sizes of values (sizes have been measured over many samples,
	   and a margin of 5 times the standard deviation). */

	/* If depth is at least 2, and we will use the NTT to subtract
	   k*f from F, then we will need to convert f to NTT over
	   slen+1 words, which requires an extra word to ft. */
	int use_sub_ntt = (depth > 1 && logn >= MIN_LOGN_FGNTT);
	if (use_sub_ntt) {
		t1 += n;
	}

	/* New layout:
	     Ft    F from this level (unreduced) (llen*n)
	     ft    f from this level (slen*n) (+n if use_sub_ntt)
	     rt3   (n fxr = 2*n) */
	fxr *rt3 = (fxr *)t1;

	/* We consider only the top rlen words of f. */
	size_t rlen = WORD_WIN[depth];
	if (rlen > slen) {
		rlen = slen;
	}
	size_t blen = slen - rlen;
	uint32_t *ftb = ft + blen * n;
	uint32_t scale_fg = 31 * (uint32_t)blen;
	uint32_t scale_FG = 31 * (uint32_t)llen;

	/* Convert f into fixed-point approximations, into rt3. It is scaled
	   down by 2^(scale_fg + scale_x). scale_fg is public (it depends
	   only on the recursion depth), but scale_x comes from a measurement
	   on the actual coefficient values of f and is thus secret.

	   The value scale_x is adjusted so that the largest coefficient is
	   close to, but lower than, some limit t (in absolute value). The
	   limit t is chosen so that f*adj(f) does not overflow, i.e. all
	   coefficients must remain below 2^31.

	   Let n be the degree; we know that n <= 2^10. The squared norm
	   of a polynomial is the sum of the squared norms of the
	   coefficients, with the squared norm of a complex number being
	   the product of that number with its complex conjugate. If all
	   coefficients of f are less than t (in absolute value), then
	   the squared norm of f is less than n*t^2. The squared norm of
	   FFT(f) (f in FFT representation) is exactly n times the
	   squared norm of f, so this leads to n^2*t^2 as a maximum
	   bound. adj(f) has the same norm as f. This implies that each
	   complex coefficient of FFT(f) has a maximum squared norm of
	   n^2*t^2 (with a maximally imbalanced polynomial with all
	   coefficient but one being zero). The computation of f*adj(f)
	   exactly is, in FFT representation, the product of each
	   coefficient with its conjugate; thus, the coefficients of
	   f*adj(f), in FFT representation, are at most n^2*t^2.

	   Since we want the coefficients of f*adj(f) not to exceed
	   2^31, we need n^2*t^2 <= 2^31, i.e. n*t <= 2^15.5. We can adjust t
	   accordingly (called scale_t in the code below). We also need to
	   take care that t must not exceed scale_x. Approximation of f and
	   g are extracted with scale scale_fg + scale_x - scale_t, and
	   later fixed by dividing them by 2^scale_t. */
	uint32_t scale_x = poly_max_bitlength(logn, ftb, rlen);
	uint32_t scale_t = 15 - logn;
	scale_t ^= (scale_t ^ scale_x) & tbmask(scale_x - scale_t);
	uint32_t scdiff = scale_x - scale_t;

	poly_big_to_fixed(logn, rt3, ftb, rlen, scdiff);

	/* rt3 <- adj(f)/(f*adj(f))  (FFT)  */
	avx2_vect_FFT(logn, rt3);
	avx2_vect_inv_mul2e_fft(logn, rt3, scale_t);

	/* New layout:
	     Ft    F from this level (unreduced) (llen*n)
	     ft    f from this level (slen*n) (+n if use_sub_ntt)
	     rt3   (n fxr = 2*n)
	     rt1   (n fxr = 2*n)     |   k    (n)
	                             |   t2   (3*n)  */
	fxr *rt1 = rt3 + n;
	int32_t *k = (int32_t *)rt1;
	uint32_t *t2 = (uint32_t *)(k + n);

	/* If we are going to use poly_sub_scaled_ntt(), then we convert
	   f to the NTT representation. Since poly_sub_scaled_ntt()
	   itself will use more than n*(slen+2) words in t2[], we can do
	   the same here. */
	if (use_sub_ntt) {
		uint32_t *gm = t2;
		uint32_t *tn = gm + n;
		for (size_t i = 0; i <= slen; i ++) {
			uint32_t p = PRIMES[i].p;
			uint32_t p0i = PRIMES[i].p0i;
			uint32_t R2 = PRIMES[i].R2;
			uint32_t Rx = mp_Rx31((unsigned)slen, p, p0i, R2);
			avx2_mp_mkgm(logn, gm, PRIMES[i].g, p, p0i);
			if (logn >= 4) {
				__m256i yp = _mm256_set1_epi32(p);
				__m256i yp0i = _mm256_set1_epi32(p0i);
				__m256i yR2 = _mm256_set1_epi32(R2);
				__m256i yRx = _mm256_set1_epi32(Rx);
				for (size_t j = 0; j < n; j ++) {
					__m256i yd;
					yd = avx2_zint_mod_small_signed_x8(
						ft + j, slen, n,
						yp, yp0i, yR2, yRx);
					_mm256_storeu_si256(
						(__m256i *)(tn + j), yd);
				}
			} else {
				for (size_t j = 0; j < n; j ++) {
					tn[j] = zint_mod_small_signed(
						ft + j, slen, n,
						p, p0i, R2, Rx);
				}
			}
			avx2_mp_NTT(logn, tn, gm, p, p0i);
			tn += n;
		}
		tn = gm + n;
		memmove(ft, tn, (slen + 1) * n * sizeof *tn);
	}

	/* Reduce F repeatedly. */
	size_t FGlen = llen;
	uint32_t reduce_bits;
	switch (logn_top) {
	case 9:  reduce_bits = 9; break;
	case 10: reduce_bits = 8; break;
	default: reduce_bits = 10; break;
	}
	for (;;) {
		/* Convert the current F into fixed-point. We want
		   to apply scaling scale_FG + scale_x. */
		uint32_t tlen, toff;
		DIVREM31(tlen, toff, scale_FG);
		poly_big_to_fixed(logn, rt1,
			Ft + tlen * n, FGlen - tlen, scale_x + toff);

		/* rt1 <- (F*adj(f)) / (f*adj(f)) */
		avx2_vect_FFT(logn, rt1);
		avx2_vect_mul_fft(logn, rt1, rt3);
		avx2_vect_iFFT(logn, rt1);

		/* k <- round(rt1) */
		for (size_t i = 0; i < n; i ++) {
			k[i] = fxr_round(rt1[i]);
		}

		/* f is scaled by scale_fg + scale_x
		   F is scaled by scale_FG + scale_x
		   Thus, k is scaled by scale_FG - scale_fg, which is public. */
		uint32_t scale_k = scale_FG - scale_fg;
		if (depth == 1) {
			avx2_poly_sub_kf_scaled_depth1(logn_top, Ft, FGlen,
				(uint32_t *)k, scale_k, f, t2);
		} else if (use_sub_ntt) {
			avx2_poly_sub_scaled_ntt(logn, Ft, FGlen, ft, slen,
				k, scale_k, t2);
		} else {
			avx2_poly_sub_scaled(logn, Ft, FGlen,
				ft, slen, k, scale_k);
		}

		/* We now assume that F has shrunk by at least
		   reduce_bits. We adjust FGlen accordinly. */
		if (scale_FG <= scale_fg) {
			break;
		}
		if (scale_FG <= (scale_fg + reduce_bits)) {
			scale_FG = scale_fg;
		} else {
			scale_FG -= reduce_bits;
		}
		while (FGlen > slen
			&& 31 * (FGlen - slen) > scale_FG - scale_fg + 30)
		{
			/* We decrement FGlen; when we do so, we check that
			   it does not damage any of the values, i.e. that the
			   removed words are redundant with the remaining
			   words. In practice, this test reliably catches
			   reduction failures early enough. */
			FGlen --;
			uint32_t *xp = &Ft[(FGlen - 1) << logn];
			for (size_t i = 0; i < n; i ++) {
				uint32_t sw = -(xp[i] >> 30) >> 1;
				if (xp[i + n] != sw) {
					return SOLVE_ERR_REDUCE;
				}
			}
		}
	}

	/* Output F is already in the right place. */
	return SOLVE_OK;
}
#endif

/* Solving the NTRU equation, top recursion level. This is a specialized
   variant for solve_NTRU_intermediate() with depth == 0, for lower RAM
   usage and faster operation; it also ensures better precision for the
   reduction. This function returns for F and G, in that order, at the
   start of tmp[].

   Returned value: 0 on success, a negative error code otherwise. */
static int
solve_NTRU_depth0(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	/* At depth 0, all values fit on 30 bits, so we work with a
	   single modulus p. */
	uint32_t p = PRIMES[0].p;
	uint32_t p0i = PRIMES[0].p0i;
	uint32_t R2 = PRIMES[0].R2;

	/* On input, Fd from upper level (hn words) is at the start of
	   tmp[]. */
	uint32_t *t1 = tmp;
	uint32_t *t2 = t1 + n;
	uint32_t *t3 = t2 + n;
	uint32_t *t4 = t3 + n;
	uint32_t *t5 = t4 + n;

	/* Convert Fd to RNS+NTT, into t3. */
	uint32_t *gm = t4;
	mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	poly_mp_set(logn - 1, t1, p);
	mp_NTT(logn - 1, t1, gm, p, p0i);
	memcpy(t3, t1, hn * sizeof *t1);

	/* Compute F (unreduced, RNS+NTT) into t1. */
	poly_mp_set_small(logn, t2, g, p);
	mp_NTT(logn, t2, gm, p, p0i);
	for (size_t i = 0; i < hn; i ++) {
		uint32_t ga = t2[(i << 1) + 0];
		uint32_t gb = t2[(i << 1) + 1];
		uint32_t mF = mp_mmul(t3[i], R2, p, p0i);
		t1[(i << 1) + 0] = mp_mmul(gb, mF, p, p0i);
		t1[(i << 1) + 1] = mp_mmul(ga, mF, p, p0i);
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   g (RNS+NTT)
	     t3   free
	     t4   gm (NTT support)
	     t5   free  */

	/* Convert f to RNS+NTT. Since we are going to divide by f modulo p,
	   we also need to check that f is invertible modulo p (which should
	   almost always be the case in practice).
	     t3 <- f (RNS+NTT)  */
	poly_mp_set_small(logn, t3, f, p);
	mp_NTT(logn, t3, gm, p, p0i);
	for (size_t i = 0; i < n; i ++) {
		if (t3[i] == 0) {
			return SOLVE_ERR_REDUCE;
		}
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   g (RNS+NTT)
	     t3   f (RNS+NTT)
	     t4   free
	     t5   free  */

	/* We want to perform the reduction. Since this is the last one,
	   we want to be precise, i.e. to use the full expression for k:

	     k = round((F*adj(f) + G*adj(g))/(f*adj(f) + g*adj(g)))

	   We do not have G but we know that G = (q + g*F)/f, which we
	   can compute modulo p (the division by f is exact over the
	   integers, hence computing it modulo p yields the correct result,
	   as long as the coefficients of G are in [-p/2,+p/2], which is
	   heuristically the case). We accumulate the numerator and
	   denominator into t2 and t3, respectively. */
	for (size_t i = 0; i < hn; i ++) {
		uint32_t tf0 = t3[i];
		uint32_t tf1 = t3[n - 1 - i];
		uint32_t tg0 = t2[i];
		uint32_t tg1 = t2[n - 1 - i];
		uint32_t tF0 = t1[i];
		uint32_t tF1 = t1[n - 1 - i];
		uint32_t mf0 = mp_mmul(tf0, R2, p, p0i);
		uint32_t mf1 = mp_mmul(tf1, R2, p, p0i);
		uint32_t mg0 = mp_mmul(tg0, R2, p, p0i);
		uint32_t mg1 = mp_mmul(tg1, R2, p, p0i);
		uint32_t tG0 = mp_div(
			mp_add(Q, mp_mmul(mg0, tF0, p, p0i), p), tf0, p);
		uint32_t tG1 = mp_div(
			mp_add(Q, mp_mmul(mg1, tF1, p, p0i), p), tf1, p);
		uint32_t kn0 = mp_add(
			mp_mmul(mf1, tF0, p, p0i),
			mp_mmul(mg1, tG0, p, p0i), p);
		uint32_t kn1 = mp_add(
			mp_mmul(mf0, tF1, p, p0i),
			mp_mmul(mg0, tG1, p, p0i), p);
		uint32_t kd = mp_add(
			mp_mmul(mf0, tf1, p, p0i),
			mp_mmul(mg0, tg1, p, p0i), p);
		t2[i] = kn0;
		t2[n - 1 - i] = kn1;
		t3[i] = kd;
		t3[n - 1 - i] = kd;
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   F*adj(f) + G*adj(g) (RNS+NTT)
	     t3   f*adj(f) + g*adj(g) (RNS+NTT)
	     t4   free
	     t5   free  */

	/* Convert back numerator and denominator to plain integers. */
	mp_mkigm(logn, t4, PRIMES[0].ig, p, p0i);
	mp_iNTT(logn, t2, t4, p, p0i);
	mp_iNTT(logn, t3, t4, p, p0i);
	for (size_t i = 0; i < n; i ++) {
		/* NOTE: no truncature to 31 bits. */
		t2[i] = (uint32_t)mp_norm(t2[i], p);
		t3[i] = (uint32_t)mp_norm(t3[i], p);
	}

#define DOWNSCALE   10
	/* We need to divide t2 by t3, and round the result. We convert
	   them to FFT representation, downscaled by 2^10 (to avoid overflows).
	   We first convert f*adj(f) + g*adj(g), which is self-adjoint;
	   thus, its FFT representation only has half-size. */
	fxr *rt4 = (fxr *)t4;
	for (size_t i = 0; i < n; i ++) {
		uint64_t x = (uint64_t)*(int32_t *)&t3[i] << (32 - DOWNSCALE);
		rt4[i] = fxr_of_scaled32(x);
	}
	vect_FFT(logn, rt4);
	memcpy(rt4 + hn, rt4, hn * sizeof *rt4);
	fxr *rt5 = rt4 + hn;
	fxr *rt3 = (fxr *)t3;
	for (size_t i = 0; i < n; i ++) {
		uint64_t x = (uint64_t)*(int32_t *)&t2[i] << (32 - DOWNSCALE);
		rt3[i] = fxr_of_scaled32(x);
	}
	vect_FFT(logn, rt3);
#undef DOWNSCALE

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   free
	     t3   F*adj(f) + G*adj(g) (FFT) (first half)   <- alias: rt3
	     t4   F*adj(f) + G*adj(g) (FFT) (second half)
	     t5   f*adj(f) + g*adj(g) (FFT) (half-size)    <- alias: rt5  */

	/* Divide F*adj(f) + G*adj(g) by f*adj(f) + g*adj(g), and round the
	   result into t2, with conversion to RNS. */
	vect_div_selfadj_fft(logn, rt3, rt5);
	vect_iFFT(logn, rt3);
	for (size_t i = 0; i < n; i ++) {
		t2[i] = mp_set(fxr_round(rt3[i]), p);
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   k (RNS)
	     t3   free
	     t4   free
	     t5   free  */

	/* Get back f and g, and convert all polynomials to RNS+NTT. */
	gm = t5;
	mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	poly_mp_set_small(logn, t3, f, p);
	poly_mp_set_small(logn, t4, g, p);
	mp_NTT(logn, t2, gm, p, p0i);
	mp_NTT(logn, t3, gm, p, p0i);
	mp_NTT(logn, t4, gm, p, p0i);

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   k (RNS+NTT)
	     t3   f (RNS+NTT)
	     t4   g (RNS+NTT)
	     t5   free  */

	/* Reduce F by subtracting k*F, and recompute the corresponding G
	   with:
	     G = (q + g*F)/f
	   (We did not keep the unreduced G, in order to save RAM.) */
	for (size_t i = 0; i < n; i ++) {
		uint32_t tF = t1[i];
		uint32_t tk = t2[i];
		uint32_t tf = t3[i];
		uint32_t tg = t4[i];
		uint32_t mf = mp_mmul(tf, R2, p, p0i);
		uint32_t mg = mp_mmul(tg, R2, p, p0i);
		tF = mp_sub(tF, mp_mmul(mf, tk, p, p0i), p);
		uint32_t tG = mp_div(
			mp_add(Q, mp_mmul(mg, tF, p, p0i), p), tf, p);
		t1[i] = tF;
		t2[i] = tG;
	}

	/* Convert back F and G into normal representation. */
	mp_mkigm(logn, t3, PRIMES[0].ig, p, p0i);
	mp_iNTT(logn, t1, t3, p, p0i);
	mp_iNTT(logn, t2, t3, p, p0i);
	poly_mp_norm(logn, t1, p);
	poly_mp_norm(logn, t2, p);

	/* By construction, f*G - g*F = q modulo p; if both F and G
	   are in the correct range ([-127,+127]), then this equation
	   will also hold over plain integers:
	     N_inf(f*G - g*F) <= (127^2)*n*2 < 2^25 < p/2
	   Verifying that F and G are in range is done by the caller. */
	return SOLVE_OK;
}

#if FNDSA_AVX2
TARGET_AVX2
static int
avx2_solve_NTRU_depth0(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g,
	uint32_t *restrict tmp)
{
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	/* At depth 0, all values fit on 30 bits, so we work with a
	   single modulus p. */
	uint32_t p = PRIMES[0].p;
	uint32_t p0i = PRIMES[0].p0i;
	uint32_t R2 = PRIMES[0].R2;

	/* On input, Fd from upper level (hn words) is at the start of
	   tmp[]. */
	uint32_t *t1 = tmp;
	uint32_t *t2 = t1 + n;
	uint32_t *t3 = t2 + n;
	uint32_t *t4 = t3 + n;
	uint32_t *t5 = t4 + n;

	/* Convert Fd to RNS+NTT, into t3. */
	uint32_t *gm = t4;
	avx2_mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	avx2_poly_mp_set(logn - 1, t1, p);
	avx2_mp_NTT(logn - 1, t1, gm, p, p0i);
	memcpy(t3, t1, hn * sizeof *t1);

	/* Compute F (unreduced, RNS+NTT) into t1. */
	avx2_poly_mp_set_small(logn, t2, g, p);
	avx2_mp_NTT(logn, t2, gm, p, p0i);
	if (hn >= 4) {
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yp0i = _mm256_set1_epi32(p0i);
		__m256i yR2 = _mm256_set1_epi32(R2);
		for (size_t i = 0; i < hn; i += 4) {
			__m256i yga = _mm256_loadu_si256(
				(__m256i *)(t2 + (i << 1)));
			__m256i ygb = _mm256_srli_epi64(yga, 32);
			__m128i xFd = _mm_loadu_si128((__m128i *)(t3 + i));
			__m256i yFd = _mm256_permute4x64_epi64(
				_mm256_castsi128_si256(xFd), 0x50);
			yFd = _mm256_shuffle_epi32(yFd, 0x30);
			yFd = mp_mmul_x4(yFd, yR2, yp, yp0i);
			__m256i yFe0 = mp_mmul_x4(ygb, yFd, yp, yp0i);
			__m256i yFe1 = mp_mmul_x4(yga, yFd, yp, yp0i);
			_mm256_storeu_si256((__m256i *)(t1 + (i << 1)),
				_mm256_or_si256(yFe0,
					_mm256_slli_epi64(yFe1, 32)));
		}
	} else {
		for (size_t i = 0; i < hn; i ++) {
			uint32_t ga = t2[(i << 1) + 0];
			uint32_t gb = t2[(i << 1) + 1];
			uint32_t mF = mp_mmul(t3[i], R2, p, p0i);
			t1[(i << 1) + 0] = mp_mmul(gb, mF, p, p0i);
			t1[(i << 1) + 1] = mp_mmul(ga, mF, p, p0i);
		}
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   g (RNS+NTT)
	     t3   free
	     t4   gm (NTT support)
	     t5   free  */

	/* Convert f to RNS+NTT. Since we are going to divide by f modulo p,
	   we also need to check that f is invertible modulo p (which should
	   almost always be the case in practice).
	     t3 <- f (RNS+NTT)  */
	avx2_poly_mp_set_small(logn, t3, f, p);
	avx2_mp_NTT(logn, t3, gm, p, p0i);
	for (size_t i = 0; i < n; i ++) {
		if (t3[i] == 0) {
			return SOLVE_ERR_REDUCE;
		}
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   g (RNS+NTT)
	     t3   f (RNS+NTT)
	     t4   free
	     t5   free  */

	/* We want to perform the reduction. Since this is the last one,
	   we want to be precise, i.e. to use the full expression for k:

	     k = round((F*adj(f) + G*adj(g))/(f*adj(f) + g*adj(g)))

	   We do not have G but we know that G = (q + g*F)/f, which we
	   can compute modulo p (the division by f is exact over the
	   integers, hence computing it modulo p yields the correct result,
	   as long as the coefficients of G are in [-p/2,+p/2], which is
	   heuristically the case). We accumulate the numerator and
	   denominator into t2 and t3, respectively. */
	if (hn >= 8) {
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yp0i = _mm256_set1_epi32(p0i);
		__m256i yR2 = _mm256_set1_epi32(R2);
		__m256i yq = _mm256_set1_epi32(Q);
		for (size_t i = 0; i < hn; i += 8) {
			/* Load values by groups of 8. */
			__m256i yf0 = _mm256_loadu_si256(
				(__m256i *)(t3 + i));
			__m256i yf1 = _mm256_loadu_si256(
				(__m256i *)(t3 + (n - 8 - i)));
			__m256i yg0 = _mm256_loadu_si256(
				(__m256i *)(t2 + i));
			__m256i yg1 = _mm256_loadu_si256(
				(__m256i *)(t2 + (n - 8 - i)));
			__m256i yF0 = _mm256_loadu_si256(
				(__m256i *)(t1 + i));
			__m256i yF1 = _mm256_loadu_si256(
				(__m256i *)(t1 + (n - 8 - i)));
			__m256i ymg0 = mp_mmul_x8(yg0, yR2, yp, yp0i);
			__m256i ymg1 = mp_mmul_x8(yg1, yR2, yp, yp0i);

			/* Compute G. */
			__m256i yG0 = mp_div_x8(
				mp_add_x8(mp_mmul_x8(ymg0, yF0, yp, yp0i),
					yq, yp),
				yf0, yp);
			__m256i yG1 = mp_div_x8(
				mp_add_x8(mp_mmul_x8(ymg1, yF1, yp, yp0i),
					yq, yp),
				yf1, yp);

			/* We need adj(f) and adj(g), which entails reversing
			   the order of the values. */
			__m256i yfa1 = _mm256_permute4x64_epi64(yf0, 0x4E);
			yfa1 = _mm256_shuffle_epi32(yfa1, 0x1B);
			__m256i yfa0 = _mm256_permute4x64_epi64(yf1, 0x4E);
			yfa0 = _mm256_shuffle_epi32(yfa0, 0x1B);
			__m256i ymga1 = _mm256_permute4x64_epi64(ymg0, 0x4E);
			ymga1 = _mm256_shuffle_epi32(ymga1, 0x1B);
			__m256i ymga0 = _mm256_permute4x64_epi64(ymg1, 0x4E);
			ymga0 = _mm256_shuffle_epi32(ymga0, 0x1B);

			__m256i ymfa0 = mp_mmul_x8(yfa0, yR2, yp, yp0i);
			__m256i ymfa1 = mp_mmul_x8(yfa1, yR2, yp, yp0i);

			/* kn <- F*adj(f) + G*adj(g) */
			__m256i ykn0 = mp_add_x8(
				mp_mmul_x8(ymfa0, yF0, yp, yp0i),
				mp_mmul_x8(ymga0, yG0, yp, yp0i), yp);
			__m256i ykn1 = mp_add_x8(
				mp_mmul_x8(ymfa1, yF1, yp, yp0i),
				mp_mmul_x8(ymga1, yG1, yp, yp0i), yp);

			/* kd <- f*adj(f) + g*adj(g)
			   kd is self-adjoint, hence we can compute the
			   right half as a swap of the left half. */
			__m256i ykd0 = mp_add_x8(
				mp_mmul_x8(ymfa0, yf0, yp, yp0i),
				mp_mmul_x8(ymga0, yg0, yp, yp0i), yp);
			__m256i ykd1 = _mm256_permute4x64_epi64(ykd0, 0x4E);
			ykd1 = _mm256_shuffle_epi32(ykd1, 0x1B);

			_mm256_storeu_si256((__m256i *)(t2 + i), ykn0);
			_mm256_storeu_si256((__m256i *)(t2 + n - 8 - i), ykn1);
			_mm256_storeu_si256((__m256i *)(t3 + i), ykd0);
			_mm256_storeu_si256((__m256i *)(t3 + n - 8 - i), ykd1);
		}
	} else {
		for (size_t i = 0; i < hn; i ++) {
			uint32_t tf0 = t3[i];
			uint32_t tf1 = t3[n - 1 - i];
			uint32_t tg0 = t2[i];
			uint32_t tg1 = t2[n - 1 - i];
			uint32_t tF0 = t1[i];
			uint32_t tF1 = t1[n - 1 - i];
			uint32_t mf0 = mp_mmul(tf0, R2, p, p0i);
			uint32_t mf1 = mp_mmul(tf1, R2, p, p0i);
			uint32_t mg0 = mp_mmul(tg0, R2, p, p0i);
			uint32_t mg1 = mp_mmul(tg1, R2, p, p0i);
			uint32_t tG0 = mp_div(mp_add(Q,
				mp_mmul(mg0, tF0, p, p0i), p), tf0, p);
			uint32_t tG1 = mp_div(mp_add(Q,
				mp_mmul(mg1, tF1, p, p0i), p), tf1, p);
			uint32_t kn0 = mp_add(
				mp_mmul(mf1, tF0, p, p0i),
				mp_mmul(mg1, tG0, p, p0i), p);
			uint32_t kn1 = mp_add(
				mp_mmul(mf0, tF1, p, p0i),
				mp_mmul(mg0, tG1, p, p0i), p);
			uint32_t kd = mp_add(
				mp_mmul(mf0, tf1, p, p0i),
				mp_mmul(mg0, tg1, p, p0i), p);
			t2[i] = kn0;
			t2[n - 1 - i] = kn1;
			t3[i] = kd;
			t3[n - 1 - i] = kd;
		}
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   F*adj(f) + G*adj(g) (RNS+NTT)
	     t3   f*adj(f) + g*adj(g) (RNS+NTT)
	     t4   free
	     t5   free  */

	/* Convert back numerator and denominator to plain integers. */
	avx2_mp_mkigm(logn, t4, PRIMES[0].ig, p, p0i);
	avx2_mp_iNTT(logn, t2, t4, p, p0i);
	avx2_mp_iNTT(logn, t3, t4, p, p0i);
	if (n >= 8) {
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yhp = _mm256_set1_epi32((p + 1) >> 1);
		for (size_t i = 0; i < n; i += 8) {
			__m256i yn = _mm256_loadu_si256((__m256i *)(t2 + i));
			__m256i yd = _mm256_loadu_si256((__m256i *)(t3 + i));
			yn = mp_norm_x8(yn, yp, yhp);
			yd = mp_norm_x8(yd, yp, yhp);
			_mm256_storeu_si256((__m256i *)(t2 + i), yn);
			_mm256_storeu_si256((__m256i *)(t3 + i), yd);
		}
	} else {
		for (size_t i = 0; i < n; i ++) {
			/* NOTE: no truncature to 31 bits. */
			t2[i] = (uint32_t)mp_norm(t2[i], p);
			t3[i] = (uint32_t)mp_norm(t3[i], p);
		}
	}

#define DOWNSCALE   10
	/* We need to divide t2 by t3, and round the result. We convert
	   them to FFT representation, downscaled by 2^10 (to avoid overflows).
	   We first convert f*adj(f) + g*adj(g), which is self-adjoint;
	   thus, its FFT representation only has half-size. */
	fxr *rt4 = (fxr *)t4;
	for (size_t i = 0; i < n; i ++) {
		uint64_t x = (uint64_t)*(int32_t *)&t3[i] << (32 - DOWNSCALE);
		rt4[i] = fxr_of_scaled32(x);
	}
	avx2_vect_FFT(logn, rt4);
	memcpy(rt4 + hn, rt4, hn * sizeof *rt4);
	fxr *rt5 = rt4 + hn;
	fxr *rt3 = (fxr *)t3;
	for (size_t i = 0; i < n; i ++) {
		uint64_t x = (uint64_t)*(int32_t *)&t2[i] << (32 - DOWNSCALE);
		rt3[i] = fxr_of_scaled32(x);
	}
	avx2_vect_FFT(logn, rt3);
#undef DOWNSCALE

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   free
	     t3   F*adj(f) + G*adj(g) (FFT) (first half)   <- alias: rt3
	     t4   F*adj(f) + G*adj(g) (FFT) (second half)
	     t5   f*adj(f) + g*adj(g) (FFT) (half-size)    <- alias: rt5  */

	/* Divide F*adj(f) + G*adj(g) by f*adj(f) + g*adj(g), and round the
	   result into t2, with conversion to RNS. */
	avx2_vect_div_selfadj_fft(logn, rt3, rt5);
	avx2_vect_iFFT(logn, rt3);
	for (size_t i = 0; i < n; i ++) {
		t2[i] = mp_set(fxr_round(rt3[i]), p);
	}

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   k (RNS)
	     t3   free
	     t4   free
	     t5   free  */

	/* Get back f and g, and convert all polynomials to RNS+NTT. */
	gm = t5;
	avx2_mp_mkgm(logn, gm, PRIMES[0].g, p, p0i);
	avx2_poly_mp_set_small(logn, t3, f, p);
	avx2_poly_mp_set_small(logn, t4, g, p);
	avx2_mp_NTT(logn, t2, gm, p, p0i);
	avx2_mp_NTT(logn, t3, gm, p, p0i);
	avx2_mp_NTT(logn, t4, gm, p, p0i);

	/* Layout:
	     t1   F (unreduced, RNS+NTT)
	     t2   k (RNS+NTT)
	     t3   f (RNS+NTT)
	     t4   g (RNS+NTT)
	     t5   free  */

	/* Reduce F by subtracting k*F, and recompute the corresponding G
	   with:
	     G = (q + g*F)/f
	   (We did not keep the unreduced G, in order to save RAM.) */
	if (n >= 8) {
		__m256i yp = _mm256_set1_epi32(p);
		__m256i yp0i = _mm256_set1_epi32(p0i);
		__m256i yR2 = _mm256_set1_epi32(R2);
		__m256i yq = _mm256_set1_epi32(Q);
		for (size_t i = 0; i < n; i += 8) {
			__m256i yF = _mm256_loadu_si256((__m256i *)(t1 + i));
			__m256i yk = _mm256_loadu_si256((__m256i *)(t2 + i));
			__m256i yf = _mm256_loadu_si256((__m256i *)(t3 + i));
			__m256i yg = _mm256_loadu_si256((__m256i *)(t4 + i));
			__m256i ymf = mp_mmul_x8(yf, yR2, yp, yp0i);
			__m256i ymg = mp_mmul_x8(yg, yR2, yp, yp0i);
			yF = mp_sub_x8(yF,
				mp_mmul_x8(ymf, yk, yp, yp0i), yp);
			__m256i yG = mp_div_x8(
				mp_add_x8(yq,
					mp_mmul_x8(ymg, yF, yp, yp0i), yp),
				yf, yp);
			_mm256_storeu_si256((__m256i *)(t1 + i), yF);
			_mm256_storeu_si256((__m256i *)(t2 + i), yG);
		}
	} else {
		for (size_t i = 0; i < n; i ++) {
			uint32_t tF = t1[i];
			uint32_t tk = t2[i];
			uint32_t tf = t3[i];
			uint32_t tg = t4[i];
			uint32_t mf = mp_mmul(tf, R2, p, p0i);
			uint32_t mg = mp_mmul(tg, R2, p, p0i);
			tF = mp_sub(tF, mp_mmul(mf, tk, p, p0i), p);
			uint32_t tG = mp_div(
				mp_add(Q, mp_mmul(mg, tF, p, p0i), p), tf, p);
			t1[i] = tF;
			t2[i] = tG;
		}
	}

	/* Convert back F and G into normal representation. */
	avx2_mp_mkigm(logn, t3, PRIMES[0].ig, p, p0i);
	avx2_mp_iNTT(logn, t1, t3, p, p0i);
	avx2_mp_iNTT(logn, t2, t3, p, p0i);
	avx2_poly_mp_norm(logn, t1, p);
	avx2_poly_mp_norm(logn, t2, p);

	/* By construction, f*G - g*F = q modulo p; if both F and G
	   are in the correct range ([-127,+127]), then this equation
	   will also hold over plain integers:
	     N_inf(f*G - g*F) <= (127^2)*n*2 < 2^25 < p/2
	   Verifying that F and G are in range is done by the caller. */
	return SOLVE_OK;
}
#endif

/* see kgen_inner.h */
int
solve_NTRU(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp)
{
	size_t n = (size_t)1 << logn;

	int err = solve_NTRU_deepest(logn, f, g, tmp);
	if (err != SOLVE_OK) {
		return 0;
	}
	unsigned depth = logn;
	while (depth -- > 1) {
		err = solve_NTRU_intermediate(logn, f, g, depth, tmp);
		if (err != SOLVE_OK) {
			return 0;
		}
	}
	err = solve_NTRU_depth0(logn, f, g, tmp);
	if (err != SOLVE_OK) {
		return 0;
	}

	/* F and G are at the start of tmp[] (plain, 31 bits per value).
	   We need to convert them to 8-bit representation, and check
	   that they are within the expected range. */
	int8_t *F = (int8_t *)(tmp + 2 * n);
	int8_t *G = F + n;
	int lim = 127;
	if (!poly_big_to_small(logn, F, tmp, lim)) {
		/* SOLVE_ERR_LIMIT */
		return 0;
	}
	if (!poly_big_to_small(logn, G, tmp + n, lim)) {
		/* SOLVE_ERR_LIMIT */
		return 0;
	}
	memmove(tmp, F, 2 * n);

	return 1;
}

#if FNDSA_AVX2
TARGET_AVX2
int
avx2_solve_NTRU(unsigned logn,
	const int8_t *restrict f, const int8_t *restrict g, uint32_t *tmp)
{
	size_t n = (size_t)1 << logn;

	int err = avx2_solve_NTRU_deepest(logn, f, g, tmp);
	if (err != SOLVE_OK) {
		return 0;
	}
	unsigned depth = logn;
	while (depth -- > 1) {
		err = avx2_solve_NTRU_intermediate(logn, f, g, depth, tmp);
		if (err != SOLVE_OK) {
			return 0;
		}
	}
	err = avx2_solve_NTRU_depth0(logn, f, g, tmp);
	if (err != SOLVE_OK) {
		return 0;
	}

	/* F and G are at the start of tmp[] (plain, 31 bits per value).
	   We need to convert them to 8-bit representation, and check
	   that they are within the expected range. */
	int8_t *F = (int8_t *)(tmp + 2 * n);
	int8_t *G = F + n;
	int lim = 127;
	if (!poly_big_to_small(logn, F, tmp, lim)) {
		/* SOLVE_ERR_LIMIT */
		return 0;
	}
	if (!poly_big_to_small(logn, G, tmp + n, lim)) {
		/* SOLVE_ERR_LIMIT */
		return 0;
	}
	memmove(tmp, F, 2 * n);

	return 1;
}
#endif

/* see kgen_inner.h */
int
check_ortho_norm(unsigned logn, const int8_t *f, const int8_t *g, fxr *tmp)
{
	size_t n = (size_t)1 << logn;
	fxr *rt1 = tmp;
	fxr *rt2 = rt1 + n;
	fxr *rt3 = rt2 + n;
	vect_set(logn, rt1, f);
	vect_set(logn, rt2, g);
	vect_FFT(logn, rt1);
	vect_FFT(logn, rt2);
	vect_invnorm_fft(logn, rt3, rt1, rt2, 0);
	vect_adj_fft(logn, rt1);
	vect_adj_fft(logn, rt2);
	vect_mul_realconst(logn, rt1, fxr_of(12289));
	vect_mul_realconst(logn, rt2, fxr_of(12289));
	vect_mul_selfadj_fft(logn, rt1, rt3);
	vect_mul_selfadj_fft(logn, rt2, rt3);
	vect_iFFT(logn, rt1);
	vect_iFFT(logn, rt2);
	fxr sn = fxr_zero;
	for (size_t i = 0; i < n; i ++) {
		sn = fxr_add(sn, fxr_add(fxr_sqr(rt1[i]), fxr_sqr(rt2[i])));
	}
	/* Constant is (0.999*1.17*sqrt(q))^2, scaled up by 2^32 for the
	   fixed-point representation.
	   (Standard says 0.9999, we are slightly more restrictive just to
	   be sure that our keys are always compliant despite using
	   fixed-point approximations for the orthogonalized norm.) */
	return fxr_lt(sn, fxr_of_scaled32(72107278641426));
}

#if FNDSA_AVX2
TARGET_AVX2
int
avx2_check_ortho_norm(unsigned logn, const int8_t *f, const int8_t *g, fxr *tmp)
{
	size_t n = (size_t)1 << logn;
	fxr *rt1 = tmp;
	fxr *rt2 = rt1 + n;
	fxr *rt3 = rt2 + n;
	avx2_vect_set(logn, rt1, f);
	avx2_vect_set(logn, rt2, g);
	avx2_vect_FFT(logn, rt1);
	avx2_vect_FFT(logn, rt2);
	avx2_vect_invnorm_fft(logn, rt3, rt1, rt2, 0);
	avx2_vect_adj_fft(logn, rt1);
	avx2_vect_adj_fft(logn, rt2);
	avx2_vect_mul_realconst(logn, rt1, fxr_of(12289));
	avx2_vect_mul_realconst(logn, rt2, fxr_of(12289));
	avx2_vect_mul_selfadj_fft(logn, rt1, rt3);
	avx2_vect_mul_selfadj_fft(logn, rt2, rt3);
	avx2_vect_iFFT(logn, rt1);
	avx2_vect_iFFT(logn, rt2);
	fxr sn;
	if (logn >= 2) {
		__m256i ysn = _mm256_setzero_si256();
		const __m256i *rp1 = (const __m256i *)rt1;
		const __m256i *rp2 = (const __m256i *)rt2;
		for (size_t i = 0; i < (n >> 2); i ++) {
			__m256i y1 = _mm256_loadu_si256(rp1 + i);
			__m256i y2 = _mm256_loadu_si256(rp2 + i);
			y1 = fxr_sqr_x4(y1);
			y2 = fxr_sqr_x4(y2);
			ysn = _mm256_add_epi64(ysn, _mm256_add_epi64(y1, y2));
		}
		__m128i xsn = _mm_add_epi64(
			_mm256_castsi256_si128(ysn),
			_mm256_extracti128_si256(ysn, 1));
		xsn = _mm_add_epi64(xsn, _mm_bsrli_si128(xsn, 8));
		uint32_t lo = (uint32_t)_mm_cvtsi128_si32(xsn);
		int32_t hi = _mm_cvtsi128_si32(_mm_bsrli_si128(xsn, 4));
		sn = fxr_of_scaled32(((int64_t)hi << 32) | (int64_t)lo);
	} else {
		/* Unused, kept for reference only: logn is the top-level
		   degree, and we enforce logn >= 2 throughout the
		   implementation. */
		sn = fxr_zero;
		for (size_t i = 0; i < n; i ++) {
			sn = fxr_add(sn,
				fxr_add(fxr_sqr(rt1[i]), fxr_sqr(rt2[i])));
		}
	}
	/* Constant is (0.999*1.17*sqrt(q))^2, scaled up by 2^32 for the
	   fixed-point representation.
	   (Standard says 0.9999, we are slightly more restrictive just to
	   be sure that our keys are always compliant despite using
	   fixed-point approximations for the orthogonalized norm.) */
	return fxr_lt(sn, fxr_of_scaled32(72107278641426));
}
#endif
