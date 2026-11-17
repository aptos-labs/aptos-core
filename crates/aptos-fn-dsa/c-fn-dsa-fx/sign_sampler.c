/*
 * Gaussian sampling.
 */

#include "sign_inner.h"

/* 1/(2*(1.8205^2)) */
static const rx RX_INV_2SQRSIGMA0 = { 125273110, 647960451, 0 };

/* 1/q */
static const rx RX_IQ = { 3827689952, 349496, 0 };

#if !FNDSA_ASM_CORTEXM4
/* q^2 */
static const rx RX_QQ = { 0, 0, 151019521 };
#endif

/* For logn = 1 to 10, n = 2^logn:
      q = 12289
      gs_norm = (117/100)*sqrt(q)
      bitsec = max(2, n/4)
      eps = 1/sqrt(bitsec*2^64)
      smoothz2n = sqrt(log(4*n*(1 + 1/eps))/pi)/sqrt(2*pi)
      sigma = smoothz2n*gs_norm
      sigma_min = sigma/gs_norm = smoothz2n
   We store precomputed values for 1/sigma and for sigma_min, indexed
   by logn. */
static const rx RX_INV_SIGMA[] = {
	{ 0, 0, 0 },                    /* unused */
	{ 3795937681,   29658807, 0 },  /* 0.0069054793295940881448815 */
	{ 1221258620,   29249701, 0 },  /* 0.0068102267767177965870212 */
	{  166159018,   28857070, 0 },  /* 0.0067188101910722700669046 */
	{ 1018012490,   28296685, 0 },  /* 0.0065883354370073655839959 */
	{ 2540388031,   27767728, 0 },  /* 0.0064651781207602891112483 */
	{  749324652,   27267368, 0 },  /* 0.0063486788828078985751774 */
	{ 3857871696,   26793116, 0 },  /* 0.0062382586529084365148390 */
	{ 1458413264,   26342780, 0 },  /* 0.0061334065020930252434828 */
	{ 3863772309,   25914413, 0 },  /* 0.0060336696681577231819900 */
	{ 1662103065,   25506287, 0 },  /* 0.0059386453095331151138571 */
};
static const rx RX_SIGMA_MIN[] = {
	{ 0, 0, 0 },                    /* unused */
	{ 1164490260,  500400228, 1 },  /* 1.1165085072329102588735 */
	{ 3630327625,  567471562, 1 },  /* 1.1321247692325272405527 */
	{ 2814877969,  633630234, 1 },  /* 1.1475285353733668684329 */
	{ 2947697067,  731235700, 1 },  /* 1.1702540788534828939955 */
	{ 3794493109,  826981503, 1 },  /* 1.1925466358390344010919 */
	{ 1568421115,  920970055, 1 },  /* 1.2144300507766139921094 */
	{  472004899, 1013294698, 1 },  /* 1.2359260567719808789999 */
	{ 3409574745, 1104040792, 1 },  /* 1.2570545284063214162514 */
	{ 4154935963, 1193286641, 1 },  /* 1.2778336969128335860106 */
	{  209075216, 1281104281, 1 },  /* 1.2982803343442918539777 */
};

/* log(2) */
static const rx RX_LOG2     = { 3520035244, 2977044471, 0 };

/* 1/log(2) */
static const rx RX_INV_LOG2 = { 3090145655, 1901360722, 1 };

/* We access the PRNG through macros so that they can be overridden at
   compile-time by some internal test routine. */
#ifndef prng_init
#define prng_init(pc, seed, seed_len)   do { \
		shake_init(pc, 256); \
		shake_inject(pc, seed, seed_len); \
		shake_flip(pc); \
	} while (0)
#define prng_next_u8    shake_next_u8
#define prng_next_u16   shake_next_u16
#define prng_next_u64   shake_next_u64
#endif

/* fndsa_gaussian0_helper() performs the sampling for the half-Gaussian,
   using (lo >> 1) | (hi << 63) as look-up value (79 bits; low bit of lo
   is ignored). */
#if FNDSA_ASM_CORTEXM4
int32_t fndsa_gaussian0_helper(uint64_t lo, uint32_t hi);
#else
static inline int32_t
fndsa_gaussian0_helper(uint64_t lo, uint32_t hi)
{
	/* Values from Table 5 (Distribution for BaseSampler), split into
	   three chunks of 31, 24 and 24 bits, in high-to-low order,
	   respectively. */
	static const uint32_t GAUSS0[][3] = {
		{ 1375468055,  6936092,  9176346 },
		{  711562636,  1023934, 15582455 },
		{  289335016,  4826132, 14746371 },
		{   90749601, 12313548, 10843417 },
		{   21676598,  5732767,  9419414 },
		{    3908963, 11171514,  6206010 },
		{     529006, 11146683, 11891888 },
		{      53503, 15610582, 11661180 },
		{       4032,  7095613, 11671091 },
		{        225, 16701698,   407118 },
		{          9,  6787688,  1638204 },
		{          0,  4870085,  8687822 },
		{          0,   111406, 13946073 },
		{          0,     1887, 12017202 },
		{          0,       23, 11452285 },
		{          0,        0,  3689579 },
		{          0,        0,    25354 },
		{          0,        0,      129 }
	};

	/* Split the 79-bit value into three words of 24, 24 and 31 bits
	   (in low-to-high order). */
	uint32_t v0 = ((uint32_t)lo >> 1) & 0x00FFFFFF;
	uint32_t v1 = (uint32_t)(lo >> 25) & 0x00FFFFFF;
	uint32_t v2 = (uint32_t)(lo >> 49) | (hi << 15);

	/* Sampled value is z such that v0..v2 is lower than the first
	   z elements of the table. */
	int32_t z = 0;
	for (size_t i = 0; i < (sizeof GAUSS0) / sizeof(GAUSS0[0]); i ++) {
		uint32_t cc;
		cc = (v0 - GAUSS0[i][2]) >> 31;
		cc = (v1 - GAUSS0[i][1] - cc) >> 31;
		cc = (v2 - GAUSS0[i][0] - cc) >> 31;
		z += (int32_t)cc;
	}
	return z;
}
#endif

#define GAUSSIAN(ss, z0, b)   do { \
		uint64_t lo = prng_next_u64(&ss->pc); \
		uint32_t hi = prng_next_u16(&ss->pc); \
		(b) = (uint32_t)lo & 1; \
		(z0) = fndsa_gaussian0_helper(lo, hi); \
	} while (0)

/* ====================== FIXED-POINT IMPLEMENTATION ====================== */

/* Compute ccs*exp(-x)*2^63, rounded to an integer.
   Assumptions:
      0 <= x < log(2)
      0 <= ccs < 1
   x and ccs are provided as trunc(x*2^64) and trunc(ccs*2^64).
   Returned value is in [0,2^63]. */
static inline uint64_t
rx_expm_p63(uint64_t x, uint64_t ccs)
{
	/* The polynomial approximation of exp(-x) is from FACCT:
	      https://eprint.iacr.org/2018/1234
	   Specifically, the values are extracted from the implementation
	   referenced by the FACCT paper, available at:
	      https://github.com/raykzhao/gaussian  */
	static const uint64_t EXPM_COEFFS[] = {
		0x00000004741183A3,
		0x00000036548CFC06,
		0x0000024FDCBF140A,
		0x0000171D939DE045,
		0x0000D00CF58F6F84,
		0x000680681CF796E3,
		0x002D82D8305B0FEA,
		0x011111110E066FD0,
		0x0555555555070F00,
		0x155555555581FF00,
		0x400000000002B400,
		0x7FFFFFFFFFFF4800,
		0x8000000000000000
	};

	/* TODO: maybe use 64x64->128 multiplications if available? It
	   is a bit tricky to decide, because the plain code is used for
	   unknown architectures, and we do not know if the larger
	   multiplication is constant-time (it often happens that it
	   is not). */

	uint64_t y = EXPM_COEFFS[0];
	uint64_t z = x;
	uint32_t z0 = (uint32_t)z, z1 = (uint32_t)(z >> 32);
#if FNDSA_ASM_CORTEXM4
#pragma GCC unroll 12
#endif
	for (size_t i = 1; i < (sizeof EXPM_COEFFS) / sizeof(uint64_t); i ++) {
		uint32_t y0 = (uint32_t)y, y1 = (uint32_t)(y >> 32);
#if FNDSA_ASM_CORTEXM4
		uint32_t tt, r0, r1;
		__asm__(
			"umull	%0, %2, %3, %5\n\t"
			"umull	%0, %1, %3, %6\n\t"
			"umaal	%2, %0, %4, %5\n\t"
			"umaal	%0, %1, %4, %6\n\t"
			: "=&r" (r0), "=&r" (r1), "=&r" (tt)
			: "r" (y0), "r" (y1), "r" (z0), "r" (z1));
		y = EXPM_COEFFS[i] - ((uint64_t)r0 | ((uint64_t)r1 << 32));
#else
		uint64_t f = (uint64_t)z0 * (uint64_t)y0;
		uint64_t a = (uint64_t)z0 * (uint64_t)y1 + (f >> 32);
		uint64_t b = (uint64_t)z1 * (uint64_t)y0;
		uint64_t c = (a >> 32) + (b >> 32)
			+ (((uint64_t)(uint32_t)a
			  + (uint64_t)(uint32_t)b) >> 32)
			+ (uint64_t)z1 * (uint64_t)y1;
		y = EXPM_COEFFS[i] - c;
#endif
	}

	/* The scaling factor must be applied at the end. Since y is now
	   in fixed-point notation, we have to convert the factor to the
	   same format, and we do an extra integer multiplication. */
	/* sigma_min is not tight, so ccs < 1 and w is not truncated to 0 */
	uint64_t w = ccs;
	uint32_t w0 = (uint32_t)w, w1 = (uint32_t)(w >> 32);
	uint32_t y0 = (uint32_t)y, y1 = (uint32_t)(y >> 32);
#if FNDSA_ASM_CORTEXM4
	uint32_t tt, r0, r1;
	__asm__(
		"umull	%0, %2, %3, %5\n\t"
		"umull	%0, %1, %3, %6\n\t"
		"umaal	%2, %0, %4, %5\n\t"
		"umaal	%0, %1, %4, %6\n\t"
		: "=&r" (r0), "=&r" (r1), "=&r" (tt)
		: "r" (y0), "r" (y1), "r" (w0), "r" (w1));
	y = (uint64_t)r0 | ((uint64_t)r1 << 32);
#else
	uint64_t f = (uint64_t)w0 * (uint64_t)y0;
	uint64_t a = (uint64_t)w0 * (uint64_t)y1 + (f >> 32);
	uint64_t b = (uint64_t)w1 * (uint64_t)y0;
	y = (a >> 32) + (b >> 32)
		+ (((uint64_t)(uint32_t)a + (uint64_t)(uint32_t)b) >> 32)
		+ (uint64_t)w1 * (uint64_t)y1;
#endif
	return y;
}

/* Sample a bit with probability ccs*exp(-x) (for x >= 0). It is assumed
   that 0 <= ccs < 1, and ccs is provided as trunc(ccs*2^64). */
static inline int
rx_ber_exp(sampler_state *ss, rx x, uint64_t ccs)
{
	/* Reduce x modulo log(2): x = s*log(2) + r, with s an integer,
	   and 0 <= r < log(2). */
	rx tx;
	rx_mul(tx, x, RX_INV_LOG2);
	int32_t si = rx_floor(tx);
	rx_of32(tx, si);
	rx_mul(tx, tx, RX_LOG2);
	rx r;
	rx_sub(r, x, tx);

	/* If s >= 64, sigma = 1.2, r = 0 and b = 1, then we get s >= 64
	   if the half-Gaussian produced z >= 13, which happens with
	   probability about 2^(-32). When s >= 64, ber_exp() will return
	   true with probability less than 2^(-64), so we can simply
	   saturate s at 63 (the bias introduced here is lower than 2^(-96),
	   and would require about 2^192 samplings to be detectable, which
	   is way beyond the formal bound of 2^64 signatures with the
	   same key. */
	uint32_t s = (uint32_t)si;
	s |= (uint32_t)(63 - s) >> 26;
	s &= 63;

	/* Compute ccs*exp(-x). Since x = s*log(2) + r, we compute
	   ccs*exp(-r)/2^s. We know that 0 <= r < log(2), so we can
	   use expm_p63(), which yields a result scaled by 63 bits. We
	   scale it up 1 bit further, then right-shift by s bits.

	   We subtract 1 to make sure that the value fits on 64 bits
	   (i.e. if r = 0 then we may get 2^64 and we prefer 2^64-1
	   in that case, to avoid the overflow). The bias is negligible
	   since expm_p63() has precision only 51 bits or so. */
	uint64_t z = rx_ursh((rx_expm_p63(rx_ufrac(r), ccs) << 1) - 1, s);

	/* Sample a bit. We lazily compare the value z with a uniform 64-bit
	   integer, consuming only as many bytes as necessary. Since the PRNG
	   is cryptographically strong, we leak no information from the
	   conditional jumps below. */
	for (int i = 56; i >= 0; i -= 8) {
		unsigned w = prng_next_u8(&ss->pc);
		unsigned bz = (unsigned)(z >> i) & 0xFF;
		if (w != bz) {
			return w < bz;
		}
	}
	return 0;
}

/* see sign_inner.h */
int32_t
x_sampler_next(sampler_state *ss, const rx mu, const rx isigma)
{
	/* Split center mu into s + r, for an integer s, and 0 <= r < 1. */
	int32_t s = rx_floor(mu);
	rx r;
	rx_of32(r, s);
	rx_sub(r, mu, r);

	/* dss = 1/(2*sigma^2) = 0.5*(isigma^2)  */
	rx dss;
	rx_sqr(dss, isigma);
	rx_half(dss, dss);

	/* ccs = sigma_min / sigma = sigma_min * isigma  */
	rx tccs;
	rx_mul(tccs, isigma, RX_SIGMA_MIN[ss->logn]);
	uint64_t ccs = rx_ufrac(tccs);

	/* We sample on centre r. */
	for (;;) {
		/* Sample z for a Gaussian distribution (non-negative only),
		   then get a random bit b to turn the sampling into a
		   bimodal distribution (we use z+1 if b = 1, or -z
		   otherwise). */
		int32_t z0, b;
		GAUSSIAN(ss, z0, b);
		int32_t z = b + ((b << 1) - 1) * z0;

		/* Rejection sampling. We want a Gaussian centred on r,
		   but we sampled against a bimodal distribution (with
		   "centres" at 0 and 1). However, we know that z is
		   always in the range where our sampling distribution is
		   greater than the Gaussian distribution, so rejection works.

		   We got z from distribution:
		      G(z) = exp(-((z-b)^2)/(2*sigma0^2))
		   We target distribution:
		      S(z) = exp(-((z-r)^2)/(2*signa^2))
		   Rejection sampling works by keeping the value z with
		   probability S(z)/G(z), and starting again otherwise.
		   This requires S(z) <= G(z), which is the case here.
		   Thus, we simply need to keep our z with probability:
		      P = exp(-x)
		   where:
		      x = ((z-r)^2)/(2*sigma^2) - ((z-b)^2)/(2*sigma0^2)
		   Here, we scale up the Bernouilli distribution, which
		   makes rejection more probable, but also makes the
		   rejection rate sufficiently decorrelated from the Gaussian
		   centre and standard deviation, so that measurement of the
		   rejection rate does not leak enough usable information
		   to attackers (which is how the implementation can claim
		   to be "constant-time").  */
		rx x;
		rx_of32(x, z);
		rx_sub(x, x, r);
		rx_sqr(x, x);
		rx_mul(x, x, dss);
		rx y;
		rx_of32(y, z0 * z0);
		rx_mul(y, y, RX_INV_2SQRSIGMA0);
		rx_sub(x, x, y);
		if (rx_ber_exp(ss, x, ccs)) {
#if FNDSA_STATS
			fndsa_stats_sample(mu, isigma, (int32_t)s + z);
#endif
			return (int32_t)s + z;
		}
	}
}

/* On Arm Cortex M4, this function must be callable externally so that it
   can be invoked by the assembly implementation of ffsamp_fft_inner(). */
#if FNDSA_ASM_CORTEXM4
#define x_ffsamp_fft_deepest   fndsa_x_ffsamp_fft_deepest
NOINLINE void
#else
NOINLINE TARGET_SSE2 TARGET_NEON static void
#endif
x_ffsamp_fft_deepest(sampler_state *ss, unsigned flags, rx *tmp)
{
	/* n = 2
	   t = t0 + t1*X; FFT is t0(i) = t0 + t1*i (already split).
	   a = a0  (since a is self-adjoint)  */
	rx leaf;
	rx_sqrt_spec(leaf, tmp[2]);
	rx_mul(leaf, leaf, RX_INV_SIGMA[ss->logn]);

	/* Elements of dz[] are explored by pairs; base offset
	   is the bit-reversal of a decrementing counter. */
	unsigned j = -- ss->off;
	j = ((j & 0x1C0) >> 6) | (j & 0x038) | ((j & 0x007) << 6);
	j = ((j & 0x124) >> 2) | (j & 0x092) | ((j & 0x049) << 2);
	j >>= (10 - ss->logn);
	unsigned j2 = j + (1u << (ss->logn - 1));

	rx t0;
	rx_of32(t0, ss->dz[j]);
	rx_mul(t0, t0, RX_IQ);
	if (!(flags & FLAG_INIT0)) {
		rx_add(t0, t0, tmp[0]);
	}
	int32_t z0 = x_sampler_next(ss, t0, leaf);
	ss->dz[j] = (int16_t)z0;
	if (!(flags & FLAG_NOOUT)) {
		rx rz;
		rx_of32(rz, z0);
		rx_sub(tmp[0], t0, rz);
	}

	rx t1;
	rx_of32(t1, ss->dz[j2]);
	rx_mul(t1, t1, RX_IQ);
	if (!(flags & FLAG_INIT0)) {
		rx_add(t1, t1, tmp[1]);
	}
	int32_t z1 = x_sampler_next(ss, t1, leaf);
	ss->dz[j2] = (int16_t)z1;
	if (!(flags & FLAG_NOOUT)) {
		rx rz;
		rx_of32(rz, z1);
		rx_sub(tmp[1], t1, rz);
	}
}

#if FNDSA_ASM_CORTEXM4
#define x_ffsamp_fft_left_inner   fndsa_x_ffsamp_fft_left_inner
void x_ffsamp_fft_left_inner(sampler_state *ss,
	unsigned logn, unsigned flags, rx *tmp);
#else
/* logn    current degree
   init0   non-zero if t == 0 on input
   noout   non-zero if FFT output is not needed
   tmp     work area

   tmp contents on input:
      t    FFT (12*n bytes)
      a    FFT, self-adjoint (6*n bytes)
   tmp contents on output:
      u    FFT (12*n bytes)   (u = t - z, with z the sampled vector)

   If init0 is set, then t is not set on input, and it is implicitly zero.
   If noout is set, then u is not written on output. */
TARGET_SSE2 TARGET_NEON static void
x_ffsamp_fft_left_inner(sampler_state *ss,
	unsigned logn, unsigned flags, rx *tmp)
{
	/* Input:
	      t           polynomial, FFT, 8*n bytes
	      a           self-adjoint polynomial, FFT, 4*n bytes
	   Output:
	      u = t - z   polynomial, FFT, 8*n bytes
	   Algorithm:
	      (t0, t1) <- split(t)
	      (a0, a1) <- split(a)  (a0 is self-adj, a1 is X-adj)
	      L10 <- adj(a1)/a0  (1/X-adj)
	      b <- a0 - a1*adj(a1)/a0  (self-adj)
	      keep t0 (6*n), a0 (3*n), L10/(X+1) (3*n)
	      u1 <- recursive-call(t1, b)    (u1 = t1 - z1)
	      t0' <- t0 + u1*L10
	      keep t0 - t0' (6*n), u1 (6*n)
	      v0 <- recursive-call(t0, a0)    (v0 = t0' - z0)
	      u0 <- v0 + (t0 - t0')
	      u <- merge(u0, u1)  */

	if (logn == 1) {
		x_ffsamp_fft_deepest(ss, flags, tmp);
		return;
	}

	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	size_t qn = n >> 2;

	uint8_t *btmp = (uint8_t *)tmp;
	rx *tmp0 = (rx *)btmp;
	rx *tmp6 = (rx *)(btmp + 6 * n);
	rx *tmp9 = (rx *)(btmp + 9 * n);
	rx *tmp12 = (rx *)(btmp + 12 * n);
	rx *tmp18 = (rx *)(btmp + 18 * n);
	rx *tmp21 = (rx *)(btmp + 21 * n);

	/* (tmp18, tmp21) <- (a0, a1) = split(a)  (self-adj) */
	xpoly_split_selfadj_fft(logn, tmp18, tmp21, tmp12);

	/* (tmp0, tmp12) <- (t0, t1) = split(t) */
	if (!(flags & FLAG_INIT0)) {
		xpoly_split_fft(logn, tmp0, tmp12);
	}
	memcpy(tmp6, tmp18, qn * sizeof(rx));

	/* At this point:
	      tmp0    t0  (6*n)  (unset if init0)
	      tmp6    a0  (3*n)
	      tmp9    free  (3*n)
	      tmp12   t1  (6*n)  (unset if init0)
	      tmp18   free  (3*n)
	      tmp21   b1  (3*n)  */

	/* tmp9 <- (X+1)*L10
	   tmp18 <- a0 - a1*adj(a1)/a0 */
	xpoly_LDL_fft(logn - 1, tmp18, tmp9, tmp6, tmp21);

	/* First recursive call. */
	x_ffsamp_fft_left_inner(ss, logn - 1, flags & ~FLAG_NOOUT, tmp12);

	/* At this point:
	      tmp0    t0  (6*n)  (unset if init0)
	      tmp6    a0  (3*n)
	      tmp9    L10/(X+1)  (3*n)
	      tmp12   u1  (6*n)
	      tmp18   free  (6*n) */

	/* tmp18 <- a0
	   tmp21 <- L10/(X+1) */
	memcpy(tmp18, tmp6, hn * sizeof(rx));

	/* tmp6  <- u1 */
	memcpy(tmp6, tmp12, hn * sizeof(rx));

	/* tmp12 <- t0 */
	if (!(flags & FLAG_INIT0)) {
		memcpy(tmp12, tmp0, hn * sizeof(rx));
	}

	/* At this point:
	      tmp0    free  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   t0  (6*n)  (unset if init0)
	      tmp18   a0  (3*n)
	      tmp21   L10/(X+1)  (3*n) */

	/* tmp0 <- t0' - t0 = u1*L10 */
	xpoly_mul_iXadj_fft(logn - 1, tmp0, tmp6, tmp21);

	/* tmp12 <- t0' */
	if (flags & FLAG_INIT0) {
		memcpy(tmp12, tmp0, hn * sizeof(rx));
	} else {
		xpoly_add(logn - 1, tmp12, tmp0);
	}

	/* At this point:
	      tmp0    t0' - t0  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   t0'  (6*n)
	      tmp18   a0  (3*n)  */

	/* Second recursive call. */
	x_ffsamp_fft_left_inner(ss, logn - 1, flags & ~FLAG_INIT0, tmp12);

	/* At this point:
	      tmp0    t0' - t0  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   v0  (6*n)
	      tmp18   free  (6*n)  */

	if (!(flags & FLAG_NOOUT)) {
		/* tmp12  <- u0 = v0 - (t0' - t0) */
		xpoly_sub(logn - 1, tmp12, tmp0);

		/* tmp18 <- u1 */
		memcpy(tmp18, tmp6, hn * sizeof(rx));

		/* tmp0  <- u = merge(u0, u1) */
		xpoly_merge_fft(logn, tmp0, tmp12, tmp18);
	}
}
#endif

/* see sign_inner.h */
void
x_ffsamp_fft_left(sampler_state *ss, unsigned flags, rx *tmp)
{
	ss->off = 1u << (ss->logn - 1);
	x_ffsamp_fft_left_inner(ss, ss->logn, flags, tmp);
}

#if FNDSA_ASM_CORTEXM4
#define x_ffsamp_fft_right_inner   fndsa_x_ffsamp_fft_right_inner
void x_ffsamp_fft_right_inner(sampler_state *ss,
	unsigned logn, unsigned flags, rx *tmp);
#else
/* logn    current degree
   init0   non-zero if t == 0 on input
   noout   non-zero if FFT output is not needed
   tmp     work area

   tmp contents on input:
      t    FFT (12*n bytes)
      a    FFT, self-adjoint (6*n bytes)
   tmp contents on output:
      u    FFT (12*n bytes)   (u = t - z, with z the sampled vector)

   If init0 is set, then t is not set on input, and it is implicitly zero.
   If noout is set, then u is not written on output. */
TARGET_SSE2 TARGET_NEON static void
x_ffsamp_fft_right_inner(sampler_state *ss,
	unsigned logn, unsigned flags, rx *tmp)
{
	/* Input:
	      t           polynomial, FFT, 12*n bytes
	      a           self-adjoint polynomial, FFT, 6*n bytes
	   Output:
	      u = t - z   polynomial, FFT, 8*n bytes
	   Algorithm:
	      (t0, t1) <- split(t)
	      (a0, a1) <- split(a)  (a0 is self-adj, a1 is X-adj)
	      L10 <- -adj(a1)/a0  (1/X-adj)
	      b <- a0 - a1*adj(a1)/a0  (self-adj)
	      keep t0 (6*n), b (3*n), L10/(X+1) (3*n)
	      u1 <- recursive-call(t1, a0)    (u1 = t1 - z1)
	      t0' <- t0 + u1*L10
	      keep t0 - t0' (6*n), u1 (6*n)
	      v0 <- recursive-call(t0, b)     (v0 = t0' - z0)
	      u0 <- v0 + (t0 - t0')
	      u <- merge(u0, u1)  */

	if (logn == 1) {
		/* This is the right-subtree exploration, thus we are
		   working with the symplectic tree; we must adjust
		   the leaf here. */
		rx_div_spec(tmp[2], RX_QQ, tmp[2]);
		x_ffsamp_fft_deepest(ss, flags, tmp);
		return;
	}

	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	uint8_t *btmp = (uint8_t *)tmp;
	rx *tmp0 = (rx *)btmp;
	rx *tmp6 = (rx *)(btmp + 6 * n);
	rx *tmp9 = (rx *)(btmp + 9 * n);
	rx *tmp12 = (rx *)(btmp + 12 * n);
	rx *tmp18 = (rx *)(btmp + 18 * n);
	rx *tmp21 = (rx *)(btmp + 21 * n);

	/* (tmp18, tmp21) <- (a0, a1) = split(a)  (self-adj) */
	xpoly_split_selfadj_fft(logn, tmp18, tmp21, tmp12);

	/* (tmp0, tmp12) <- (t0, t1) = split(t) */
	if (!(flags & FLAG_INIT0)) {
		xpoly_split_fft(logn, tmp0, tmp12);
	}

	/* At this point:
	      tmp0    t0  (6*n)  (unset if init0)
	      tmp6    free  (3*n)
	      tmp9    free  (3*n)
	      tmp12   t1  (6*n)  (unset if init0)
	      tmp18   a0  (3*n)
	      tmp21   b1  (3*n)  */

	/* tmp9 <- L10/(X+1)  (including symplectic sign adjustement)
	   tmp6 <- a0 - a1*adj(a1)/a0 */
	xpoly_LDL_fft(logn - 1, tmp6, tmp9, tmp18, tmp21);
	xpoly_neg(logn - 2, tmp9);

	/* First recursive call. */
	x_ffsamp_fft_right_inner(ss, logn - 1, flags & ~FLAG_NOOUT, tmp12);

	/* At this point:
	      tmp0    t0  (6*n)  (unset if init0)
	      tmp6    a0 - a1*adj(a1)/a0  (3*n)
	      tmp9    L10/(X+1)  (3*n)
	      tmp12   u1  (6*n)
	      tmp18   free  (6*n) */

	/* tmp18 <- a0 - a1*adj(a1)/a0
	   tmp21 <- L10/(X+1) */
	memcpy(tmp18, tmp6, hn * sizeof(rx));

	/* tmp6  <- u1 */
	memcpy(tmp6, tmp12, hn * sizeof(rx));

	/* tmp12  <- t0 */
	if (!(flags & FLAG_INIT0)) {
		memcpy(tmp12, tmp0, hn * sizeof(rx));
	}

	/* At this point:
	      tmp0    free  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   t0  (6*n)  (unset if init0)
	      tmp18   a0 - a1*adj(a1)/a0  (3*n)
	      tmp21   L10/(X+1)  (3*n) */

	/* tmp0 <- t0' - t0 = u1*L10 */
	xpoly_mul_iXadj_fft(logn - 1, tmp0, tmp6, tmp21);

	/* tmp12 <- t0' */
	if (flags & FLAG_INIT0) {
		memcpy(tmp12, tmp0, hn * sizeof(rx));
	} else {
		xpoly_add(logn - 1, tmp12, tmp0);
	}

	/* At this point:
	      tmp0    t0' - t0  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   t0'  (6*n)
	      tmp18   a0 - a1*adj(a1)/a0  (3*n)  */

	/* Second recursive call. */
	x_ffsamp_fft_right_inner(ss, logn - 1, flags & ~FLAG_INIT0, tmp12);

	/* At this point:
	      tmp0    t0' - t0  (6*n)
	      tmp6    u1  (6*n)
	      tmp12   v0  (6*n)
	      tmp18   free  (6*n)  */

	if (!(flags & FLAG_NOOUT)) {
		/* tmp12  <- u0 = v0 - (t0' - t0) */
		xpoly_sub(logn - 1, tmp12, tmp0);

		/* tmp18 <- u1 */
		memcpy(tmp18, tmp6, hn * sizeof(rx));

		/* tmp0  <- u = merge(u0, u1) */
		xpoly_merge_fft(logn, tmp0, tmp12, tmp18);
	}
}
#endif

/* see sign_inner.h */
void
x_ffsamp_fft_right(sampler_state *ss, unsigned flags, rx *tmp)
{
	ss->off = 1u << (ss->logn - 1);
	x_ffsamp_fft_right_inner(ss, ss->logn, flags, tmp);
}

static void
x_ffsamp_expkey_fft_deepest(sampler_state *ss, const rx leaf, rx *tmp)
{
	/* Elements of dz[] are explored by pairs; base offset
	   is the bit-reversal of a decrementing counter. */
	unsigned j = -- ss->off;
	j = ((j & 0x1C0) >> 6) | (j & 0x038) | ((j & 0x007) << 6);
	j = ((j & 0x124) >> 2) | (j & 0x092) | ((j & 0x049) << 2);
	j >>= (10 - ss->logn);
	unsigned j2 = j + (1u << (ss->logn - 1));

	rx rz;
	rx t0;
	rx_of32(t0, ss->dz[j]);
	rx_mul(t0, t0, RX_IQ);
	rx_add(t0, t0, tmp[0]);
	int32_t z0 = x_sampler_next(ss, t0, leaf);
	ss->dz[j] = (int16_t)z0;
	rx_of32(rz, z0);
	rx_sub(tmp[0], t0, rz);

	rx t1;
	rx_of32(t1, ss->dz[j2]);
	rx_mul(t1, t1, RX_IQ);
	rx_add(t1, t1, tmp[1]);
	int32_t z1 = x_sampler_next(ss, t1, leaf);
	ss->dz[j2] = (int16_t)z1;
	rx_of32(rz, z1);
	rx_sub(tmp[1], t1, rz);
}

/* see sign_inner.h */
void
x_ffsamp_expkey_fft_left(sampler_state *ss,
	const rx *xnodes, const rx *xleaves, rx *tmp)
{
	unsigned logn = ss->logn;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	size_t qn = n >> 2;
	ss->off = hn;

	/* Let k be the level, starting at logn (top-level) down to 2
	   (lowest level, just above the leaves). At level k:
	      input is polynomial t, of degree 2^k
	      input splits into two polynomials of degree 2^(k-1)
	      node value has degree 2^(k-1) and uses 2^(k-2) elements
	   Tree is explored right-to-left, hence each layer in inner nodes
	   is used in descending address order. cur[k-2] keeps track of
	   the current node at level k. */
	const rx *cur[9];
	for (unsigned k = logn; k >= 2; k --) {
		xnodes += qn;
		cur[k - 2] = xnodes;
	}
	xleaves += hn;

	/* Throughout the walk, we update k (current level) and tmp (pointer
	   to values and temporary space). */

	/* Downward-right move:
	      tmp[] contains t, which has degree 2^k
	      split (t0, t1) <- t1
	      reserve 2^k elements in tmp (t0 uses the first half)
	      recursive call on t1 and degree k-1 */
#define DOWN_RIGHT   do { \
		xpoly_split_fft(k, tmp, tmp + ((size_t)1 << k)); \
		tmp += (size_t)1 << k; \
		k --; \
	} while (0)

	/* Upward-left+downward-left move:
	      tmp[] contains u1, degree 2^k (returned from recursive call)
	      ww <- u1*T.value
	      t0' <- t0 + u1*T.value
	      keep u1 and t0' - t0
	      recursive call on t0' */
#define UP_DOWN_LEFT   do { \
		rx *t0 = tmp - ((size_t)2 << k); \
		rx *ww = tmp - ((size_t)1 << k); \
		const rx *L10 = (cur[k - 1] -= (size_t)1 << (k - 1)); \
		memcpy(ww, tmp, sizeof(rx) << k); \
		memcpy(tmp, t0, sizeof(rx) << k); \
		xpoly_mul_iXadj_fft(k, t0, ww, L10); \
		xpoly_add(k, tmp, t0); \
	} while (0)

	/* Upward-right move:
	      tmp[] contains v0, degree 2^k (returned from recursive call)
	      move up (increment k, move back tmp)
	      u0 <- v0 - (t0' - t0)
	      u <- merge(u0, u1) */
#define UP_RIGHT   do { \
		rx *v0 = tmp; \
		rx *ww = tmp + ((size_t)1 << k); \
		rx *u1 = tmp - ((size_t)1 << k); \
		tmp = u1 - ((size_t)1 << k); \
		xpoly_sub(k, v0, tmp); \
		memcpy(ww, u1, sizeof(rx) << k); \
		k ++; \
		xpoly_merge_fft(k, tmp, v0, ww); \
	} while (0)

	unsigned k = logn;

	for (size_t j = 0; j < hn; j ++) {
		if (j == 0) {
			/* First leaf: descend the right-most part of
			   the tree. */
			while (k > 1) {
				DOWN_RIGHT;
			}
		} else {
			/* Let s = number of trailing zeros in j.
			   We do s up-right, then one up-down-left, then
			   s down-right. */
			for (unsigned z = j; (z & 1) == 0; z >>= 1) {
				UP_RIGHT;
			}
			UP_DOWN_LEFT;
			for (unsigned z = j; (z & 1) == 0; z >>= 1) {
				DOWN_RIGHT;
			}
		}
		x_ffsamp_expkey_fft_deepest(ss, *-- xleaves, tmp);
	}

	/* When exploring the left sub-tree, we do not need the final
	   output, so we do not have to compute the final merges. */

#undef DOWN_RIGHT
#undef UP_DOWN_LEFT
#undef UP_RIGHT
}

/* see sign_inner.h */
void
x_ffsamp_expkey_fft_right(sampler_state *ss,
	const rx *xnodes, const rx *xleaves, rx *tmp)
{
	unsigned logn = ss->logn;
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;
	size_t qn = n >> 2;
	ss->off = hn;

	/* Let k be the level, starting at logn (top-level) down to 2
	   (lowest level, just above the leaves). At level k:
	      input is polynomial t, of degree 2^k
	      input splits into two polynomials of degree 2^(k-1)
	      node value has degree 2^(k-1) and uses 2^(k-2) elements
	   Tree is explored right-to-left, but stored tree is mirrored
	   (through symplecticity), hence each layer in inner nodes
	   is used in ascending address order. cur[k-2] keeps track of
	   the current node at level k.
	   Leaves are still used in descending address order. */
	const rx *cur[9];
	for (unsigned k = logn; k >= 2; k --) {
		cur[k - 2] = xnodes;
		xnodes += qn;
	}
	xleaves += hn;

	/* When exploring the right sub-tree, input is zero. We skip
	   the unneeded initial splits, but we should still initialize
	   the space with zeros. */
	memset(tmp, 0, sizeof(rx) << (logn + 1));

	/* Throughout the walk, we update k (current level) and tmp (pointer
	   to values and temporary space). */

	/* Downward-right move:
	      tmp[] contains t, which has degree 2^k
	      split (t0, t1) <- t1
	      reserve 2^k elements in tmp (t0 uses the first half)
	      recursive call on t1 and degree k-1 */
#define DOWN_RIGHT   do { \
		xpoly_split_fft(k, tmp, tmp + ((size_t)1 << k)); \
		tmp += (size_t)1 << k; \
		k --; \
	} while (0)

	/* Upward-left+downward-left move:
	      tmp[] contains u1, degree 2^k (returned from recursive call)
	      ww <- u1*T.value
	      t0' <- t0 - u1*T.value    (symplectic sign adjustment)
	      keep u1 and t0 - t0'
	      recursive call on t0' */
#define UP_DOWN_LEFT   do { \
		rx *t0 = tmp - ((size_t)2 << k); \
		rx *ww = tmp - ((size_t)1 << k); \
		const rx *L10 = cur[k - 1]; \
		cur[k - 1] += (size_t)1 << (k - 1); \
		memcpy(ww, tmp, sizeof(rx) << k); \
		memcpy(tmp, t0, sizeof(rx) << k); \
		xpoly_mul_iXadj_fft(k, t0, ww, L10); \
		xpoly_sub(k, tmp, t0); \
	} while (0)

	/* Upward-right move:
	      tmp[] contains v0, degree 2^k (returned from recursive call)
	      move up (increment k, move back tmp)
	      u0 <- v0 + (t0 - t0')
	      u <- merge(u0, u1) */
#define UP_RIGHT   do { \
		rx *v0 = tmp; \
		rx *ww = tmp + ((size_t)1 << k); \
		rx *u1 = tmp - ((size_t)1 << k); \
		tmp = u1 - ((size_t)1 << k); \
		xpoly_add(k, v0, tmp); \
		memcpy(ww, u1, sizeof(rx) << k); \
		k ++; \
		xpoly_merge_fft(k, tmp, v0, ww); \
	} while (0)

	/* We start at the leaf level (we virtually descend from the root
	   by walking the rightmost path, but the input is zero so we
	   can skip all the splits). */
	unsigned k = 1;
	tmp += ((size_t)1 << (logn + 1)) - 4;

	for (size_t j = 0; j < hn; j ++) {
		if (j != 0) {
			/* Let s = number of trailing zeros in j.
			   We do s up-right, then one up-down-left, then
			   s down-right. */
			for (unsigned z = j; (z & 1) == 0; z >>= 1) {
				UP_RIGHT;
			}
			UP_DOWN_LEFT;
			for (unsigned z = j; (z & 1) == 0; z >>= 1) {
				DOWN_RIGHT;
			}
		}
		x_ffsamp_expkey_fft_deepest(ss, *-- xleaves, tmp);
	}

	/* Walk the leftmost path. */
	while (k != logn) {
		UP_RIGHT;
	}

#undef DOWN_RIGHT
#undef UP_DOWN_LEFT
#undef UP_RIGHT
}
