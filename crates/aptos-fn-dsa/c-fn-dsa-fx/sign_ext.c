/*
 * Core signature generation.
 */

#include "sign_inner.h"

/* TODO: possibly merge with same array in sign_sampler.c?
   Note: making it local might help the compiler infer things
   about it. */
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

static const rx RX_Q = { 0, 0, 12289 };

/* see sign_inner.h */
TARGET_SSE2 TARGET_NEON
void *
expand_skey(int weak, const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_max_len)
{
	if (sign_key_len < 1) {
		return NULL;
	}
	unsigned head = *(const uint8_t *)sign_key;
	if ((head & 0xF0) != 0x50) {
		return NULL;
	}
	unsigned logn = head & 0x0F;
	if (weak) {
		if (logn < 2 || logn > 8) {
			return NULL;
		}
	} else {
		if (logn < 9 || logn > 10) {
			return NULL;
		}
	}
	if (sign_key_len != FNDSA_SIGN_KEY_SIZE(logn)) {
		return NULL;
	}
	const uint8_t *sign_key_fgF = (const uint8_t *)sign_key + 1;

	/* Expanded key format:
	     logn                  1 byte
	     h                     2*n bytes    (NTT, internal)
	     f                     n bytes      (int8)
	     F                     n bytes      (int8)
	     L10 (root)            12*n bytes   (FFT)
	     Tree leaves (left)    12*(n/2) bytes
	     Tree leaves (right)   12*(n/2) bytes
	     Inner tree nodes      12*(logn - 1)*(n/4) bytes  (FFT)
	     hashed_vkey           64 bytes
	     hashed_fg             40 bytes
	   Total: (3*logn + 25)*n + 105 bytes */
	size_t n = (size_t)1 << logn;
	size_t hn = n >> 1;

	/* Ensure 32-byte alignment, accounting for an extra byte for the
	   header (logn). */
	uintptr_t p1 = (uintptr_t)ekey;
	uintptr_t p2 = (p1 + 32) & ~(uintptr_t)31;
	size_t ekey_len = (size_t)(p2 - p1) + (size_t)(3 * logn + 25) * n + 104;
	if (ekey_max_len < ekey_len) {
		return NULL;
	}
	ekey = (uint8_t *)p2 - 1;
	*(uint8_t *)ekey = logn;

	/* buf points to the start of the aligned region. */
	uint8_t *buf = (uint8_t *)p2;

	/* We define 9 temporary buffers (t0 to t8) of size 2*n each, and
	   one buffer (t9) of size 4*n, starting at the same address as L10:
	     L10 area: t0 to t5
	     Tree leaves (left): t6 to t8
	     Tree leaves (right): t9
	   We first decode and compute f, g, F, and G, both modulo q and
	   modulo q'; while doing that, we also compute h (the public key)
	   and store it in NTT format. Ultimately, we'll want
	   f*adj(f) + g*adj(g) and F*adj(f) + G*adj(g) in t8 and t9,
	   respectively. */

	uint16_t *t0 = (uint16_t *)(buf + 4 * n);
	uint16_t *t1 = t0 + n;
	uint16_t *t2 = t1 + n;
	uint16_t *t3 = t2 + n;
	uint16_t *t4 = t3 + n;
	uint16_t *t5 = t4 + n;
	uint16_t *t6 = t5 + n;
	uint16_t *t7 = t6 + n;
	int16_t *t8 = (int16_t *)(t7 + n);
	int32_t *t9 = (int32_t *)(t8 + n);
	uint16_t *h = (uint16_t *)buf;
	int8_t *f = (int8_t *)(h + n);
	int8_t *F = f + n;

	/* Get hashed verify key. */
	uint8_t *hashed_vkey = buf + ((size_t)(3 * logn + 25) << logn);
	unsigned nbits = fg_nbits(logn);
	size_t flen = (nbits << logn) >> 3;
	memcpy(buf + ((size_t)(3 * logn + 25) << logn),
		sign_key_fgF + (flen << 1) + n, 64);

	/* Get hashed signing key (f and g).
	   This is used for randomness hedging. */
	uint8_t *hashed_fg = hashed_vkey + 64;
	shake_context sc;
	shake_init(&sc, 256);
	shake_inject(&sc, sign_key_fgF, flen << 1);
	shake_flip(&sc);
	shake_extract(&sc, hashed_fg, 40);

	/* t0 <- f mod q (NTT)
	   t4 <- f mod q' (NTT) */
	(void)trim_i8_decode(logn, sign_key_fgF, f, nbits);
	uint32_t sqnf = mqpoly_sqnorm_small(logn, f);
	mqpoly_small_to_int(logn, f, t0);
	mqpoly_int_to_ntt(logn, t0);
	mq2poly_small_to_mod(logn, f, t4);
	mq2poly_NTT(logn, t4);

	/* t1 <- g mod q (NTT)
	   t5 <- g mod q' (NTT)
	   We temporarily use the F area to store the decoded g.
	   We also complete the check on ||(g,-f)||. */
	(void)trim_i8_decode(logn, sign_key_fgF + flen, F, nbits);
	uint32_t sqng = mqpoly_sqnorm_small(logn, F);
	if (sqnf + sqng > 16822) {
		return NULL;
	}
	mqpoly_small_to_int(logn, F, t1);
	mqpoly_int_to_ntt(logn, t1);
	mq2poly_small_to_mod(logn, F, t5);
	mq2poly_NTT(logn, t5);

	/* t2 <- F mod q (NTT)
	   t6 <- F mod q' (NTT) */
	memcpy(F, sign_key_fgF + (flen << 1), n);
	mqpoly_small_to_int(logn, F, t2);
	mqpoly_int_to_ntt(logn, t2);
	mq2poly_small_to_mod(logn, F, t6);
	mq2poly_NTT(logn, t6);

	/* t3 <- G mod q (NTT)
	   t7 <- G mod q' (NTT)
	   h <- g/f mod q (NTT) */
	memcpy(h, t0, n * sizeof(uint16_t));    /* h <- f mod q */
	mqpoly_inv_ntt(logn, h, t7);            /* h <- 1/f mod q */
	mqpoly_mul_ntt(logn, h, t1);            /* h <- g/f mod q */
	memcpy(t3, h, n * sizeof(uint16_t));    /* t3 <- g/f mod q */
	mqpoly_mul_ntt(logn, t3, t2);           /* t3 <- g*F/f = G mod q */
	memcpy(t7, t3, n * sizeof(uint16_t));
	mqpoly_ntt_to_int(logn, t7);
	mqpoly_int_to_signed(logn, t7);         /* t7 <- G (normalized) */
	mq2poly_signed_to_mod(logn, t7);
	mq2poly_NTT(logn, t7);

	/* t2 <- F*adj(f) + G*adj(g) mod q (normalized) */
	mqpoly_muladj_add_muladj(logn, t2, t0, t3, t1);
	mqpoly_ntt_to_int(logn, t2);
	mqpoly_int_to_signed(logn, t2);

	/* t7 <- F*adj(f) + G*adj(g) mod q' (normalized)
	   t6 is consumed */
	mq2poly_muladj_ntt(logn, t6, t4);
	mq2poly_muladj_ntt(logn, t7, t5);
	mq2poly_add(logn, t7, t6);
	mq2poly_iNTT(logn, t7);
	mq2poly_mod_to_signed(logn, t7);

	/* t9 <- F*adj(f) + G*adj(g)  (int32) */
	mq2mqpoly_CRT(logn, t9, (int16_t *)t2, (int16_t *)t7);

	/* t4 <- f*adj(f) + g*adj(g) mod q' (normalized) */
	mq2poly_muladj_x2_ntt(logn, t4, t5);
	mq2poly_iNTT(logn, t4);
	mq2poly_mod_to_signed(logn, t4);

	/* t0 <- f*adj(f) + g*adj(g) mod q (normalized) */
	mqpoly_muladj_x2_ntt(logn, t0, t1);
	mqpoly_ntt_to_int(logn, t0);
	mqpoly_int_to_signed(logn, t0);

	/* t8 <- f*adj(f) + g*adj(g)  (int16) */
	memcpy(t8, t4, n * sizeof(int16_t));
	mq2mqpoly_adjust_save(logn, t8, (int16_t *)t0, (uint8_t *)t4);

	/* At this point:
	     h, f and F are set
	     t8 contains f*adj(f) + g*adj(g)  (int16)
	     t9 contains F*adj(f) + G*adj(g)  (int32) */
	rx *xL10 = (rx *)t0;
	rx *xleaves_left = xL10 + n;
	rx *xleaves_right = xleaves_left + hn;
	rx *xnodes = xleaves_right + hn;

	/* L10 <- (F*adj(f) + G*adj(g))/(f*adj(f) + g*adj(g))  (FFT)
	   leaves_left <- f*adj(f) + g*adj(g)  (FFT, self-adj) */
	xpoly_FFT_int32(logn, xL10, t9);
	xpoly_FFT_int16_selfadj(logn, xleaves_left, t8);
	xpoly_div_selfadj_spec_fft(logn, xL10, xleaves_left);

	/* Compute the inner node values. At each layer, we have k
	   self-adjoint polynomials (degree n/k), stored consecutively
	   in xleaves_left. */
	rx *xout = xnodes;
	rx *a0 = xleaves_right;
	rx *b1 = a0 + (n >> 2);
	for (unsigned lm = logn; lm >= 2; lm --) {
		/* We have k self-adjoint polynomials, of degree m;
		   each uses m/2 elements. Each must be split and
		   LDL-decomposed. */
		size_t k = (size_t)1 << (logn - lm);
		size_t m = (size_t)1 << lm;
		size_t hm = m >> 1;
		size_t qm = m >> 2;
		rx *xin = xleaves_left;
		for (size_t j = 0; j < k; j ++) {
			xpoly_split_selfadj_fft(lm, a0, b1, xin);
			memcpy(xin, a0, qm * sizeof(rx));
			xpoly_LDL_fft(lm - 1, xin + qm, xout, a0, b1);
			xin += hm;
			xout += qm;
		}
	}

	/* xleaves_left now contains the raw leaves, we must normalize
	   them and also compute xleaves_right through symplecticity. */
	for (size_t j = 0; j < hn; j ++) {
		rx y;
		rx_sqrt_spec(y, xleaves_left[j]);
		rx_mul(xleaves_left[j], y, RX_INV_SIGMA[logn]);
		rx_div_spec(y, RX_Q, y);
		rx_mul(xleaves_right[hn - 1 - j], y, RX_INV_SIGMA[logn]);
	}

	return ekey;
}

/* see sign_inner.h */
TARGET_SSE2 TARGET_NEON
size_t
sign_core_expkey(unsigned logn, const void *ekbuf, const uint8_t *mu,
	const uint8_t *seed, size_t seed_len, uint8_t *sig, void *tmp)
{
	/* Output value is 0 on error, or the signature length on success. */
	size_t ret = 0;
	size_t n = (size_t)1 << logn;

	/* Expanded key format (after the header byte):
	     logn                  1 byte
	     h                     2*n bytes    (NTT, internal)
	     f                     n bytes      (int8)
	     F                     n bytes      (int8)
	     L10 (root)            12*n bytes   (FFT)
	     Tree leaves (left)    12*(n/2) bytes
	     Tree leaves (right)   12*(n/2) bytes
	     Inner tree nodes      12*(logn - 1)*(n/4) bytes  (FFT)
	     hashed_vkey           64 bytes
	     hashed_fg             40 bytes  */
	const uint16_t *h = (const uint16_t *)ekbuf;
	const int8_t *f = (const int8_t *)(h + n);
	const int8_t *F = f + n;
	const rx *L10 = (const rx *)(F + n);
	const rx *leaves_left = L10 + n;
	const rx *leaves_right = leaves_left + (n >> 1);
	const rx *nodes = leaves_right + (n >> 1);

	/* The loop contents usually run one time; restart probability
	   is about 1/2450 at n=512, 1/820 at n=1024. Thus, probability
	   of needing more than 27 iterations is less than 2^(-256).
	   We force an exit after 27 iterations; repeated failures are
	   conceptually feasible if the private key has invalid contents. */
	for (uint8_t counter = 0; counter < 27; counter ++) {
		/* Initialize a SHAKE context which will be used throughout
		   for getting the 40-byte nonce, then the sampling.
		   The nonce is stored directly in the signature output
		   buffer (sig + 1). */
		sampler_state ss;
		ss.logn = logn;
		shake_init(&ss.pc, 256);
		shake_inject(&ss.pc, seed, seed_len);
		shake_inject(&ss.pc, &counter, 1);
		shake_flip(&ss.pc);
		shake_extract(&ss.pc, sig + 1, 40);

#if FNDSA_STATS
		fndsa_stats_start_iteration(counter, sig + 1);
#endif

		rx *xt0 = (rx *)tmp;
		rx *xt1 = xt0 + n;
		uint16_t *uz0 = (uint16_t *)(xt1 + n);
		uint16_t *uz1 = uz0 + n;
		uint16_t *cf = uz1 + n;
		uint16_t *ncF = cf + n;
		uint16_t *c = ncF + n;

		/* c <- hashed input mod q  (NTT) */
		hash_to_point(logn, sig + 1, mu, 0, c);
		mqpoly_ext_to_int(logn, c);
		mqpoly_int_to_ntt(logn, c);

		/* uz0 <- -c*F mod q  (normalized) */
		mqpoly_small_to_int(logn, F, uz0);
		mqpoly_neg(logn, uz0);
		mqpoly_int_to_ntt(logn, uz0);
		mqpoly_mul_ntt(logn, uz0, c);
		mqpoly_ntt_to_int(logn, uz0);
		mqpoly_int_to_signed(logn, uz0);

		/* uz1 <- c*f mod q  (normalized) */
		mqpoly_small_to_int(logn, f, uz1);
		mqpoly_int_to_ntt(logn, uz1);
		mqpoly_mul_ntt(logn, uz1, c);
		mqpoly_ntt_to_int(logn, uz1);
		mqpoly_int_to_signed(logn, uz1);

		/* cf <- c*f mod q  (normalized) */
		memcpy(cf, uz1, n * sizeof(uint16_t));

		/* ncF <- -c*F mod q  (normalized) */
		memcpy(ncF, uz0, n * sizeof(uint16_t));

		/* ffSampling on t1 */
		ss.dz = (int16_t *)uz1;
		x_ffsamp_expkey_fft_right(&ss, nodes, leaves_right, xt0);

		/* xt0 <- xt0 * L10 */
		xpoly_mul_fft(logn, xt0, L10);

		/* ffSampling on t1 */
		ss.dz = (int16_t *)uz0;
		x_ffsamp_expkey_fft_left(&ss, nodes, leaves_left, xt0);

		/* Build signature: s1 = f*(z0 + r0) + F*(z1 + r1)
		   with:
		      c*f = (c*f mod q) + q*r1
		      -c*F = (-c*F mod q) + q*r0
		   This simplifies to:
		      s1 = f*(z0 + (-c*F)/q - (-c*F mod q)/q)
		         + F*(z1 + c*f/q - (c*f mod q)/q)
		      s1 = f*(z0 + (c*F mod q)/q) + F*(z1 - (c*f mod q)/q)
		   Since s1 is integral, we can do these computations
		   modulo q'.
		   We obtained z0 and z1 in uz0 and uz1, respectively. */
		uint16_t *tmp0 = (uint16_t *)xt0;
		uint16_t *tmp1 = tmp0 + n;

		/* cf <- (c*f mod q)/q mod q' */
		mq2poly_signed_to_mod(logn, cf);
		mq2poly_mulconst(logn, cf, MQ2_IQ);

		/* ncF <- (-c*F mod q)/q mod q' */
		mq2poly_signed_to_mod(logn, ncF);
		mq2poly_mulconst(logn, ncF, MQ2_IQ);

		/* uz0 <- z0 - (-c*F mod q)/q mod q'  (NTT) */
		mq2poly_signed_to_mod(logn, uz0);
		mq2poly_sub(logn, uz0, ncF);
		mq2poly_NTT(logn, uz0);

		/* uz1 <- z1 - (c*f mod q)/q mod q'  (NTT) */
		mq2poly_signed_to_mod(logn, uz1);
		mq2poly_sub(logn, uz1, cf);
		mq2poly_NTT(logn, uz1);

		/* tmp1 <- s1 = f*(z0 + (c*F mod q)/q)
		              + F*(z1 - (c*f mod q)/q)  (normalized) */
		mq2poly_small_to_mod(logn, f, tmp0);
		mq2poly_NTT(logn, tmp0);
		mq2poly_mul_ntt(logn, tmp0, uz0);
		mq2poly_small_to_mod(logn, F, tmp1);
		mq2poly_NTT(logn, tmp1);
		mq2poly_mul_ntt(logn, tmp1, uz1);
		mq2poly_add(logn, tmp1, tmp0);
		mq2poly_iNTT(logn, tmp1);
		mq2poly_mod_to_signed(logn, tmp1);

		/* tmp0 <- -s0 = s1*h - c mod q  (normalized) */
		memcpy(tmp0, tmp1, n * sizeof(uint16_t));
		mqpoly_signed_to_int(logn, tmp0);
		mqpoly_int_to_ntt(logn, tmp0);
		mqpoly_mul_ntt(logn, tmp0, h);
		mqpoly_sub(logn, tmp0, c);
		mqpoly_ntt_to_int(logn, tmp0);
		mqpoly_int_to_signed(logn, tmp0);

		/* We have the putative signature (s0, s1) in (tmp0, tmp1).
		   This signature is acceptable if the L2 and infinity
		   norms of (s0, s1) are low engouh, and s1 can be encoded
		   in the allocated output size. */
		uint32_t sqn0 = mqpoly_sqnorm_binf_signed(
			logn, (int16_t *)tmp0);
		uint32_t sqn1 = mqpoly_sqnorm_binf_signed(
			logn, (int16_t *)tmp1);
		uint32_t sqn = (sqn0 + sqn1) | ((sqn0 | sqn1) & 0x80000000);
		if (!mqpoly_sqnorm_is_acceptable(logn, sqn)) {
			continue;
		}

		size_t sig_len = FNDSA_SIGNATURE_SIZE(logn);
		if (comp_encode(logn,
			(int16_t *)tmp1, sig + 41, sig_len - 41))
		{
			/* Success! */
			sig[0] = 0x30 + logn;
			ret = sig_len;
			break;
		}
	}
	return ret;
}

/* see fndsa.h */
void *
fndsa_expand_signkey(const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_len)
{
	return expand_skey(0, sign_key, sign_key_len, ekey, ekey_len);
}

/* see fndsa.h */
void *
fndsa_expand_signkey_weak(const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_len)
{
	return expand_skey(1, sign_key, sign_key_len, ekey, ekey_len);
}

/* see fndsa.h */
size_t
fndsa_signexp_temp(const void *exp_key,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return fndsa_signexp_seeded_temp(exp_key,
		ctx, ctx_len, id, hv, hv_len, NULL, 0,
		sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_signexp_seeded_temp(const void *exp_key,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	const uint8_t *ekbuf = (const uint8_t *)exp_key;
	unsigned logn = *ekbuf ++;
	if (sig == NULL) {
		return FNDSA_SIGNATURE_SIZE(logn);
	}
	if (max_sig_len < FNDSA_SIGNATURE_SIZE(logn)) {
		return 0;
	}

	/* Expanded key contains the hashed verifying and signing keys. */
	const uint8_t *hashed_vkey = (const uint8_t *)ekbuf
		+ ((size_t)(3 * logn + 25) << logn);
	const uint8_t *hashed_fg = hashed_vkey + 64;

	/* We compute the message representative mu. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		/* External mu mode. */
		if (hv_len != 64) {
			return 0;
		}
		memcpy(mu, hv, hv_len);
	} else {
		if (!fndsa_compute_mu(mu, hashed_vkey,
			ctx, ctx_len, id, hv, hv_len))
		{
			return 0;
		}
	}

	/* Check temporary buffer length, and align tmp to a
	   32-byte boundary. */
	if (tmp_len < (((size_t)34 << logn) + 31)) {
		return 0;
	}
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* We need a signing seed (40 bytes). */
	uint8_t seedbuf[SEEDBUF_LEN];
	memcpy(seedbuf, hashed_fg, SEEDBUF_LEN);
	if (!make_seedbuf(seedbuf, NULL,
		mu, seed, seed_len, (void *)"20261008"))
	{
		return 0;
	}

	/* Main signing loop. */
	return sign_core_expkey(logn, ekbuf, mu,
		seedbuf, SEEDBUF_LEN, sig, tmp);
}
