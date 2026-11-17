/*
 * Signature verification, with message recovery.
 */

#include "inner.h"

/*
 * Inner verification function; it enforces a specific degree range.
 */
static size_t
inner_verify_mrm(unsigned logn_min, unsigned logn_max,
	const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *embedded, size_t embedded_max_len,
	void *tmp, size_t tmp_len)
{
	/* Get header bytes for signature, check the signature type,
	   degree, and size. */
	if (sig_len == 0) {
		return 0;
	}
	const uint8_t *sigbuf = (const uint8_t *)sig;
	unsigned hb = sigbuf[0];
	if ((hb >> 4) != SIG_TYPE_MRM) {
		return 0;
	}
	unsigned logn = hb & 0x0F;
	if (logn < logn_min || logn > logn_max) {
		return 0;
	}
	if (sig_len != FNDSA_SIGNATURE_MRM_SIZE(logn)) {
		return 0;
	}

	/* Check verifying key size and degree. */
	if (vrfy_key_len != FNDSA_VRFY_KEY_SIZE(logn)) {
		return 0;
	}
	const uint8_t *vkbuf = (const uint8_t *)vrfy_key;
	if (vkbuf[0] != logn) {
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
	uint16_t *t0 = (uint16_t *)tmp;
	uint16_t *t1 = t0 + n;

	/* t1 <- s1
	   This also checks that s1 complies with the infinity norm bound. */
	size_t sx_len = FNDSA_SIGNATURE_SIZE(logn) - 41;
	if (!comp_decode(logn, sigbuf + 1 + sx_len, sx_len, (int16_t *)t1)) {
		return 0;
	}
	uint32_t norm1 = mqpoly_sqnorm_signed(logn, t1);
	mqpoly_signed_to_int(logn, t1);

	/* t0 <- h */
	if (mqpoly_decode(logn, vkbuf + 1, t0) != vrfy_key_len - 1) {
		return 0;
	}

	/* t1 <- s1*h */
	mqpoly_ext_to_int(logn, t0);
	mqpoly_int_to_ntt(logn, t1);
	mqpoly_mul_ntt(logn, t1, t0);
	mqpoly_ntt_to_int(logn, t1);

	/* t0 <- s0
	   This also checks that s0 complies with the infinity norm bound. */
	if (!comp_decode(logn, sigbuf + 1, sx_len, (int16_t *)t0)) {
		return 0;
	}
	uint32_t norm0 = mqpoly_sqnorm_signed(logn, t0);
	mqpoly_signed_to_int(logn, t0);

	/* Check L2 norm of (s0, s1). */
	if (norm1 != 0 && norm0 >= -norm1) {
		return 0;
	}
	if (!mqpoly_sqnorm_is_acceptable(logn, norm0 + norm1)) {
		return 0;
	}

	/* t0 <- c = s0 + s1*h (external format) */
	mqpoly_add(logn, t0, t1);
	mqpoly_int_to_ext(logn, t0);

	/* t1 <- H2(c1) (starting at offset lambda) */
	mrm_H2(logn, t0, t1);

	/* t0 <- c1 || nonce || m_r = c - H2(c1) */
	memset(t1, 0, MRM_LAMBDA(logn) * sizeof(uint16_t));
	mqpoly_ext_to_int(logn, t0);
	mqpoly_ext_to_int(logn, t1);
	mqpoly_sub(logn, t0, t1);
	mqpoly_int_to_ext(logn, t0);
	/* Since t1[0..lambda-1] was cleared, t0[0..lambda-1] still
	   contains the received c1. */

	/* Find out the actual embedded message length by locating the
	   trailing '1' bit (and removing it). */
	int embedded_len = mrm_get_embedded_length(logn, t0);
	if (embedded_len < 0 || (size_t)embedded_len > embedded_max_len) {
		/* Invalid padding, or embedded part too large for the
		   provided buffer. */
		return -1;
	}

	/* We can now decode the embedded part. */
	bits_writer bw = { embedded, (size_t)embedded_len, 0, 0, 1 };
	size_t num_163 = MRM_NUM163(logn);
	size_t num_27 = MRM_NUM27(logn);
	size_t j = MRM_LAMBDA(logn) + MRM_GAMMA(logn);
	while (num_163 -- > 0) {
		if (!bits_q12_to_163(&t0[j], &bw)) {
			return 0;
		}
		j += 12;
	}
	while (num_27 -- > 0) {
		if (!bits_q2_to_27(&t0[j], &bw)) {
			return 0;
		}
		j += 2;
	}
	/* mrm_get_embedded_length() validated that the embedded part was
	   validly encoded, and the padding bit was removed when decoding,
	   so no further encoded-related check remains. */

	/* Recompute c1 into t1. */
	mrm_H1(logn, &t0[MRM_LAMBDA(logn)], mu, embedded, embedded_len, t1);

	/* Signature is valid only if this recomputed c1 matches the
	   value found in the rebuilt c polynomial. */
	if (memcmp(t0, t1, MRM_LAMBDA(logn) * sizeof(uint16_t)) != 0) {
		return -1;
	}

	/* Return embedded part length. */
	return embedded_len;
}

/* see fndsa.h */
int
fndsa_verify_mrm_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *embedded, size_t embedded_len,
	void *tmp, size_t tmp_len)
{
	return inner_verify_mrm(9, 10, sig, sig_len, vrfy_key, vrfy_key_len,
		ctx, ctx_len, id, hv, hv_len, embedded, embedded_len,
		tmp, tmp_len);
}
