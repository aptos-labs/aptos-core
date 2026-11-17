/*
 * Signature verification.
 */

#include "inner.h"

/*
 * Inner verification function; it enforces a specific degree range.
 */
static int
inner_verify(unsigned logn_min, unsigned logn_max,
	const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len)
{
	/* Get header bytes for key and signature, check that they relate
	   to the same degree, and that it is acceptable. */
	if (sig_len == 0 || vrfy_key_len == 0) {
		return 0;
	}
	const uint8_t *sigbuf = (const uint8_t *)sig;
	const uint8_t *vkbuf = (const uint8_t *)vrfy_key;
	unsigned logn = vkbuf[0];
	if (logn < logn_min || logn > logn_max || sigbuf[0] != 0x30 + logn) {
		return 0;
	}

	/* Keys and signatures have known fixed sizes. */
	if (sig_len != FNDSA_SIGNATURE_SIZE(logn)
		|| vrfy_key_len != FNDSA_VRFY_KEY_SIZE(logn))
	{
		return 0;
	}

	/* Check that temporary area is large enough. */
	size_t n = (size_t)1 << logn;
	if (tmp_len < (n * 4 + 31)) {
		return 0;
	}
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* Get message representative mu. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		if (hv_len != sizeof mu) {
			return 0;
		}
		memcpy(mu, hv, sizeof mu);
	} else {
		fndsa_hashed_vrfykey_from_vrfykey(mu, vrfy_key, vrfy_key_len);
		if (!fndsa_compute_mu(mu, mu, ctx, ctx_len, id, hv, hv_len)) {
			return 0;
		}
	}

	/* Get two buffers of n 16-bit values from the temporary area. */
	uint16_t *t1 = (uint16_t *)tmp;
	uint16_t *t2 = t1 + n;

	/* t1 <- h (verifying key, decoded); h is in ntt representation. */
	if (mqpoly_decode(logn, vkbuf + 1, t1) != vrfy_key_len - 1) {
		return 0;
	}
	mqpoly_ext_to_int(logn, t1);

	/* t2 <- s2 (signature, decoded, converted to ntt)
	   Also get the squared norm of s2. */
	if (!comp_decode(logn, sigbuf + 41, sig_len - 41, (int16_t *)t2)) {
		/* Note: comp_decode() checks the L-infinity norm of s2. */
		return 0;
	}
	uint32_t norm2 = mqpoly_sqnorm_signed(logn, t2);
	mqpoly_signed_to_int(logn, t2);
	mqpoly_int_to_ntt(logn, t2);

	/* t2 <- s2*h (converted to int) */
	mqpoly_mul_ntt(logn, t2, t1);
	mqpoly_ntt_to_int(logn, t2);

	/* Hash message into polynomial c (into t1, converted to int) */
	hash_to_point(logn, sigbuf + 1, mu, 0, t1);
	mqpoly_ext_to_int(logn, t1);

	/* t1 <- s1 = c - s2*h (converted to ext), and compute its norm;
	   this also verifies the L-infinity norm of s1. */
	mqpoly_sub(logn, t1, t2);
	mqpoly_int_to_ext(logn, t1);
	uint32_t norm1 = mqpoly_sqnorm_binf_ext(logn, t1);

	/* Signature is valid if the total squared norm of (s1,s2) is
	   small enough. Beware overflows. */
	if (norm2 != 0 && norm1 >= -norm2) {
		return 0;
	}
	return mqpoly_sqnorm_is_acceptable(logn, norm1 + norm2);
}

#if FNDSA_AVX2
TARGET_AVX2
static int
avx2_inner_verify(unsigned logn_min, unsigned logn_max,
	const void *sig, size_t sig_len,
        const void *vrfy_key, size_t vrfy_key_len,
        const void *ctx, size_t ctx_len,
        const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len)
{
	/* Get header bytes for key and signature, check that they relate
	   to the same degree, and that it is acceptable. */
	if (sig_len == 0 || vrfy_key_len == 0) {
		return 0;
	}
	const uint8_t *sigbuf = (const uint8_t *)sig;
	const uint8_t *vkbuf = (const uint8_t *)vrfy_key;
	unsigned logn = vkbuf[0];
	if (logn < logn_min || logn > logn_max || sigbuf[0] != 0x30 + logn) {
		return 0;
	}

	/* Keys and signatures have known fixed sizes. */
	if (sig_len != FNDSA_SIGNATURE_SIZE(logn)
		|| vrfy_key_len != FNDSA_VRFY_KEY_SIZE(logn))
	{
		return 0;
	}

	/* Check that temporary area is large enough. */
	size_t n = (size_t)1 << logn;
	if (tmp_len < (n * 4 + 31)) {
		return 0;
	}
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* Get message representative mu. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		if (hv_len != sizeof mu) {
			return 0;
		}
		memcpy(mu, hv, sizeof mu);
	} else {
		fndsa_hashed_vrfykey_from_vrfykey(mu, vrfy_key, vrfy_key_len);
		if (!fndsa_compute_mu(mu, mu, ctx, ctx_len, id, hv, hv_len)) {
			return 0;
		}
	}

	/* Get two buffers of n 16-bit values from the temporary area. */
	uint16_t *t1 = (uint16_t *)tmp;
	uint16_t *t2 = t1 + n;

	/* t1 <- h (verifying key, decoded); h is in ntt representation. */
	if (mqpoly_decode(logn, vkbuf + 1, t1) != vrfy_key_len - 1) {
		return 0;
	}
	avx2_mqpoly_ext_to_int(logn, t1);

	/* t2 <- s2 (signature, decoded, converted to ntt)
	   Also get the squared norm of s2. */
	if (!comp_decode(logn, sigbuf + 41, sig_len - 41, (int16_t *)t2)) {
		/* Note: comp_decode() checks the L-infinity norm of s2. */
		return 0;
	}
	uint32_t norm2 = avx2_mqpoly_sqnorm_signed(logn, t2);
	avx2_mqpoly_signed_to_int(logn, t2);
	avx2_mqpoly_int_to_ntt(logn, t2);

	/* t2 <- s2*h (converted to int) */
	avx2_mqpoly_mul_ntt(logn, t2, t1);
	avx2_mqpoly_ntt_to_int(logn, t2);

	/* Hash message into polynomial c (into t1, converted to int) */
	hash_to_point(logn, sigbuf + 1, mu, 0, t1);
	avx2_mqpoly_ext_to_int(logn, t1);

	/* t1 <- s1 = c - s2*h (converted to ext), and compute its norm. */
	avx2_mqpoly_sub(logn, t1, t2);
	avx2_mqpoly_int_to_ext(logn, t1);
	uint32_t norm1 = avx2_mqpoly_sqnorm_binf_ext(logn, t1);

	/* Signature is valid if the total squared norm of (s1,s2) is
	   small enough. Beware overflows. */
	if (norm2 != 0 && norm1 >= -norm2) {
		return 0;
	}
	return mqpoly_sqnorm_is_acceptable(logn, norm1 + norm2);
}
#endif

/* see fndsa.h */
int
fndsa_verify(const void *sig, size_t sig_len,
        const void *vrfy_key, size_t vrfy_key_len,
        const void *ctx, size_t ctx_len,
        const char *id, const void *hv, size_t hv_len)
{
	uint8_t tmp[4 * 1024 + 31];
#if FNDSA_AVX2
	if (has_avx2()) {
		return avx2_inner_verify(9, 10,
			sig, sig_len, vrfy_key, vrfy_key_len,
			ctx, ctx_len, id, hv, hv_len, tmp, sizeof tmp);
	}
#endif
	return inner_verify(9, 10,
		sig, sig_len, vrfy_key, vrfy_key_len,
		ctx, ctx_len, id, hv, hv_len, tmp, sizeof tmp);
}

/* see fndsa.h */
int
fndsa_verify_weak(const void *sig, size_t sig_len,
        const void *vrfy_key, size_t vrfy_key_len,
        const void *ctx, size_t ctx_len,
        const char *id, const void *hv, size_t hv_len)
{
	uint8_t tmp[4 * 256 + 31];
#if FNDSA_AVX2
	if (has_avx2()) {
		return avx2_inner_verify(2, 8,
			sig, sig_len, vrfy_key, vrfy_key_len,
			ctx, ctx_len, id, hv, hv_len, tmp, sizeof tmp);
	}
#endif
	return inner_verify(2, 8,
		sig, sig_len, vrfy_key, vrfy_key_len,
		ctx, ctx_len, id, hv, hv_len, tmp, sizeof tmp);
}

/* see fndsa.h */
int
fndsa_verify_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len)
{
#if FNDSA_AVX2
	if (has_avx2()) {
		return avx2_inner_verify(9, 10,
			sig, sig_len, vrfy_key, vrfy_key_len,
			ctx, ctx_len, id, hv, hv_len, tmp, tmp_len);
	}
#endif
	return inner_verify(9, 10,
		sig, sig_len, vrfy_key, vrfy_key_len,
		ctx, ctx_len, id, hv, hv_len, tmp, tmp_len);
}

/* see fndsa.h */
int
fndsa_verify_weak_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len)
{
#if FNDSA_AVX2
	if (has_avx2()) {
		return avx2_inner_verify(2, 8,
			sig, sig_len, vrfy_key, vrfy_key_len,
			ctx, ctx_len, id, hv, hv_len, tmp, tmp_len);
	}
#endif
	return inner_verify(2, 8,
		sig, sig_len, vrfy_key, vrfy_key_len,
		ctx, ctx_len, id, hv, hv_len, tmp, tmp_len);
}

/*
 * Inner verification function; it enforces a specific degree range.
 */
static size_t
inner_verify_krm(unsigned logn_min, unsigned logn_max,
	const void *sig, size_t sig_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *vrfy_key, size_t max_vrfy_key_len,
	void *tmp, size_t tmp_len)
{
	/* Get header bytes for signature, check the signature type,
	   degree, and size. */
	if (sig_len == 0) {
		return 0;
	}
	const uint8_t *sigbuf = (const uint8_t *)sig;
	unsigned hb = sigbuf[0];
	if ((hb >> 4) != SIG_TYPE_KRM) {
		return 0;
	}
	unsigned logn = hb & 0x0F;
	if (logn < logn_min || logn > logn_max) {
		return 0;
	}
	if (sig_len != FNDSA_SIGNATURE_KRM_SIZE(logn)) {
		return 0;
	}

	/* Check that the output buffer is large enough. */
	if (max_vrfy_key_len < FNDSA_VRFY_KEY_SIZE(logn)) {
		return 0;
	}

	/* Check that temporary area is large enough. */
	size_t n = (size_t)1 << logn;
	if (tmp_len < (n * 4 + 31)) {
		return 0;
	}
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* Get message representative mu. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		if (hv_len != sizeof mu) {
			return 0;
		}
		memcpy(mu, hv, sizeof mu);
	} else {
		/* hashed_vkey is NULL by default for KRM. */
		if (!fndsa_compute_mu(mu, NULL, ctx, ctx_len, id, hv, hv_len)) {
			return 0;
		}
	}

	/* Get two buffers of n 16-bit values from the temporary area. */
	uint16_t *t0 = (uint16_t *)tmp;
	uint16_t *t1 = t0 + n;

	/* t1 <- s0 (first signature component).
	   This also checks that it complies with the infinity norm bound. */
	size_t sx_len = FNDSA_SIGNATURE_SIZE(logn) - 41;
	if (!comp_decode(logn, sigbuf + 41, sx_len, (int16_t *)t1)) {
		return 0;
	}
	uint32_t norm0 = mqpoly_sqnorm_signed(logn, t1);

	/* t0 <- c - s0  (mod q) */
	mqpoly_signed_to_int(logn, t1);
	hash_to_point(logn, sigbuf + 1, mu, 1, t0);
	mqpoly_ext_to_int(logn, t0);
	mqpoly_sub(logn, t0, t1);

	/* t1 <- s1 (second signature component).
	   This also checks that it complies with the infinity norm bound. */
	if (!comp_decode(logn, sigbuf + 41 + sx_len, sx_len, (int16_t *)t1)) {
		return 0;
	}
	uint32_t norm1 = mqpoly_sqnorm_signed(logn, t1);

	/* If the L2 norm of (s0,s1) is inacceptable, reject the signature. */
	if (norm1 != 0 && norm0 >= -norm1) {
		return 0;
	}
	if (!mqpoly_sqnorm_is_acceptable(logn, norm0 + norm1)) {
		return 0;
	}

	/* From this point, there is no possible error condition; we will
	   build and return a public key. */

	/* t0 <- h = (c - s0)/s1 mod q (ntt) */
	mqpoly_int_to_ntt(logn, t0);
	mqpoly_signed_to_int(logn, t1);
	mqpoly_int_to_ntt(logn, t1);

	/* Slightly atrocious hack to be able to keep within RAM requirements.
	   The division function normally uses an extra temporary buffer,
	   which we do not have here (the extra buffer powers the use of
	   the Montgomery trick to speed up divisions). However, we can
	   use the output buffer to the verifying key as temporary; it
	   is large enough for a half-size polynomial. Since the division
	   is coefficient-wise, this still work if we pretend that each
	   polynomial is two concatenated half-size polynomials. */
	uintptr_t p = (uintptr_t)vrfy_key;
	unsigned aa = (logn < 5) ? ((1u << logn) - 1) : 31;
	p = (p + aa) & ~(uintptr_t)aa;
	uint16_t *t2 = (uint16_t *)p;
	size_t hn = n >> 1;
	mqpoly_div_ntt(logn - 1, t0, t1, t2);
	mqpoly_div_ntt(logn - 1, t0 + hn, t1 + hn, t2);

	/* Encode h into output, and return. */
	mqpoly_int_to_ext(logn, t0);
	uint8_t *vkbuf = (uint8_t *)vrfy_key;
	vkbuf[0] = logn;
	return 1 + mqpoly_encode(logn, t0, vkbuf + 1);
}

/* see fndsa.h */
size_t
fndsa_verify_krm_temp(const void *sig, size_t sig_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *vrfy_key, size_t max_vrfy_key_len,
	void *tmp, size_t tmp_len)
{
	return inner_verify_krm(9, 10, sig, sig_len,
		ctx, ctx_len, id, hv, hv_len,
		vrfy_key, max_vrfy_key_len, tmp, tmp_len);
}
