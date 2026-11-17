/*
 * Message recovery signature generation.
 */

#include "sign_inner.h"

/* Rehash the seed along with some extra data. This is used to ensure
   the seed derandomization in MRM also covers the embedded message part. */
NOINLINE
static void
rehash_seed(uint8_t *seedbuf, const uint8_t *extra, size_t extra_len)
{
	shake_context sc;
	shake_init(&sc, 256);
	shake_inject(&sc, seedbuf, SEEDBUF_LEN);
	shake_inject(&sc, extra, extra_len);
	shake_flip(&sc);
	shake_extract(&sc, seedbuf, SEEDBUF_LEN);
}

/* Message recover mode:
   nonce is in (Z_q)^gamma
   c is obtained as the concatenation c1 || c2, with:
      c1 <- H1(nonce, mu, embedded)
      c2 <- H2(c1) + (nonce || embedded)
   gamma = 15 for n=512, 24 for n=1024
   nonce has size gamma elements of Z_q
   H1 output has size lambda elements of Z_q
   lambda = 19 for n=512, 38 for n=1024 */

typedef struct {
	fndsa_variant inner;
	const uint8_t *embedded;
	size_t embedded_len;
	const uint8_t *mu;
} fndsa_variant_mrm;

static inline uint16_t *
nonce_ptr(uint8_t *sig)
{
	/* TODO: enforce stronger alignment? */
	uintptr_t p = (uintptr_t)(sig + 1);
	return (uint16_t *)((p + 1) & ~(uintptr_t)1);
}

static void
make_nonce_mrm(fndsa_variant *fv,
	unsigned logn, shake_context *sc, uint8_t *sig)
{
	(void)fv;
	(void)mrm_make_nonce(logn, sc, nonce_ptr(sig));
}

static void
make_c_mrm(fndsa_variant *fv, unsigned logn, uint8_t *sig, uint16_t *c)
{
	fndsa_variant_mrm *fvm = (fndsa_variant_mrm *)fv;
	mrm_hash_to_point(logn, nonce_ptr(sig),
		fvm->mu, fvm->embedded, fvm->embedded_len, c);
}

static size_t
sign_mrm(const void *sign_key, size_t sign_key_len,
	const void *embedded, size_t embedded_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	/* Get degree from the signing key. */
	if (sign_key_len == 0) {
		return 0;
	}
	unsigned head = ((const uint8_t *)sign_key)[0];
	if ((head & 0xF0) != 0x50) {
		return 0;
	}
	unsigned logn = head & 0x0F;
	if (logn < 9 || logn > 10) {
		return 0;
	}

	/* Check signing key size, signature size, and embedded part size. */
	if (sign_key_len != FNDSA_SIGN_KEY_SIZE(logn)) {
		return 0;
	}
	if (max_sig_len < FNDSA_SIGNATURE_MRM_SIZE(logn)) {
		return 0;
	}
	if (embedded_len > FNDSA_MAX_MRM_SIZE(logn)) {
		return 0;
	}

	/* Get the message representative mu for the non-embedded part. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		/* External mu mode. */
		if (hv_len != 64) {
			return 0;
		}
		memcpy(mu, hv, hv_len);
	} else {
		if (!fndsa_compute_mu(mu,
			fndsa_hashed_vrfykey_from_signkey(
				sign_key, sign_key_len),
			ctx, ctx_len, id, hv, hv_len))
		{
			return 0;
		}
	}

	/* Check tmp size */
	if (tmp_len < FNDSA_SIGN_TMP_SIZE(logn)) {
		return 0;
	}

	/* Align tmp to a 32-byte boundary. */
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* Get the signing seed (40 bytes). */
	uint8_t seedbuf[SEEDBUF_LEN];
	if (!make_seedbuf(seedbuf, sign_key,
		mu, seed, seed_len, "20261009"))
	{
		return 0;
	}
	/* Also make the seed depend on the embedded part. */
	rehash_seed(seedbuf, embedded, embedded_len);

#if FNDSA_STATS
	fndsa_starts_start_sign(logn, sign_key, mu, seedbuf);
#endif

	fndsa_variant_mrm fvm = {
		{ &make_nonce_mrm, &make_c_mrm, SIG_TYPE_MRM },
		embedded, embedded_len, mu
	};
	return sign_core(logn, (const uint8_t *)sign_key + 1, &fvm.inner,
		seedbuf, SEEDBUF_LEN, sig, tmp);
}

/* see fndsa.h */
size_t
fndsa_sign_mrm_temp(const void *sign_key, size_t sign_key_len,
	const void *embedded, size_t embedded_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_mrm(sign_key, sign_key_len,
		embedded, embedded_len,
		ctx, ctx_len, id, hv, hv_len, NULL, 0,
		sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_mrm_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *embedded, size_t embedded_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_mrm(sign_key, sign_key_len,
		embedded, embedded_len,
		ctx, ctx_len, id, hv, hv_len, seed, seed_len,
		sig, max_sig_len, tmp, tmp_len);
}
