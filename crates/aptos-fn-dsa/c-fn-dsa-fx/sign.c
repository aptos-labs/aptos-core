/*
 * Top-level signature generation functions.
 */

#include "sign_inner.h"

/* see sign_inner.h */
NOINLINE
int
make_seedbuf(uint8_t *seedbuf, const uint8_t *sign_key,
	const uint8_t *mu, const void *seed, size_t seed_len,
	const void *impl_id)
{
	shake_context sc;

	/* For hedging support, we need the hash of the private key. */
	if (sign_key != NULL) {
		unsigned logn = sign_key[0] & 0x0F;
		unsigned nbits = fg_nbits(logn);
		shake_init(&sc, 256);
		shake_inject(&sc, sign_key + 1, nbits << (logn - 2));
		shake_flip(&sc);
		shake_extract(&sc, seedbuf, SEEDBUF_LEN);
	}

	/* The derived seed is obtained as:
	     SHAKE256(impl_id || SHAKE256(f||g)[40] || mu || seed)[40]
	   We already have SHAKE256(f||g)[40] in seedbuf[].

	   WARNING:
	   --------

	   'impl_id' is an identifier that characterizes the
	   implementation variant: if two implementations do not perform
	   _exactly_ the same floating-point operations in the same
	   order, then they may diverge at some point, even if working
	   on the same private key and same seed. If somebody indeed
	   uses the same private key, same message and same seed in two
	   such implementations, then the divergence may reveal
	   sensitive information on the private key. We want to avoid
	   that, which is why there is an additional 'impl_id'. For the
	   "official" code, impl_id is the empty string; here, we use an
	   arbitrary 8-byte value. If another implementation may futher
	   diverge, then it MUST change the impl_id value here.

	   (Here I am using the implementation date as identifier; the
	   8-byte size is meant to preserve alignment of inputs in SHAKE.) */
	shake_init(&sc, 256);
	if (impl_id != NULL) {
		shake_inject(&sc, impl_id, 8);
	}
	shake_inject(&sc, seedbuf, SEEDBUF_LEN);
	shake_inject(&sc, mu, 64);

	/* We need some entropy. If none was provided, then we use the
	   system RNG. */
	if (seed == NULL) {
		if (!sysrng(seedbuf, SEEDBUF_LEN)) {
			return 0;
		}
		seed = seedbuf;
		seed_len = SEEDBUF_LEN;
	}
	shake_inject(&sc, seed, seed_len);
	shake_flip(&sc);
	shake_extract(&sc, seedbuf, SEEDBUF_LEN);
	return 1;
}

/* Standard signature:
      nonce is included in the signature (at offset 1)
      signature only includes the second vector component
      c is obtained by hashing the nonce and mu */
typedef struct {
	fndsa_variant inner;
	const uint8_t *mu;
} fndsa_variant_std;

static void
make_nonce_std(fndsa_variant *fv,
	unsigned logn, shake_context *sc, uint8_t *sig)
{
	(void)fv;
	(void)logn;
	shake_extract(sc, sig + 1, 40);
}

static void
make_c_std(fndsa_variant *fv, unsigned logn, uint8_t *sig, uint16_t *c)
{
	fndsa_variant_std *fvs = (fndsa_variant_std *)fv;
	hash_to_point(logn, sig + 1, fvs->mu, 0, c);
}

/* KRM uses a specific hash-to-point process (for domain separation with
   the standard mode). */
static void
make_c_krm(fndsa_variant *fv, unsigned logn, uint8_t *sig, uint16_t *c)
{
	fndsa_variant_std *fvs = (fndsa_variant_std *)fv;
	hash_to_point(logn, sig + 1, fvs->mu, 1, c);
}

/* Verified properties at this point:
      degree is acceptable
      encoded signing key has the proper size
      signature buffer is large enough to receive the result
      tmp is large enough (but not necessarily aligned)  */
#if FNDSA_ASM_CORTEXM4
#define sign_step1   fndsa_sign_step1
#else
static
#endif
size_t
sign_step1(unsigned logn, const uint8_t *sign_key, const uint8_t *mu,
	const uint8_t *seed, size_t seed_len, uint8_t *sig, void *tmp)
{
	/* Align tmp to a 32-byte boundary. */
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* We need a signing seed (40 bytes). */
	uint8_t seedbuf[SEEDBUF_LEN];
	if (!make_seedbuf(seedbuf, sign_key,
		mu, seed, seed_len, "20261006"))
	{
		return 0;
	}

#if FNDSA_STATS
	fndsa_stats_start_sign(logn, sign_key, mu, seedbuf);
#endif

	/* Main signing loop. */
	fndsa_variant_std fvs = {
		{ &make_nonce_std, &make_c_std, SIG_TYPE_STD },
		mu
	};
	return sign_core(logn, sign_key + 1, &fvs.inner,
		seedbuf, SEEDBUF_LEN, sig, tmp);

	/* TODO: maybe explicitly overwrite the whole temporary area with
	   zeros? Arguably this is mostly wasted time if the area is
	   allocated on the stack, and if tmp is provided explicitly then
	   it is the responsibility of the caller to do any appropriate
	   zeroizing. */
}

/* Custom wrappers to allocate the temporary buffers on the stack. Several
   wrappers are defined so that stack allocation is not always worst-case. */
#define SIGN_WRAP(sz)   \
	NOINLINE static size_t sign_ ## sz(unsigned logn, \
		const uint8_t *sign_key, const uint8_t *mu, \
		const uint8_t *seed, size_t seed_len, \
		uint8_t *sig) \
	{ \
		uint8_t tmp[(sz) * 28 + 31]; \
		return sign_step1(logn, \
			sign_key, mu, seed, seed_len, sig, tmp); \
	}

SIGN_WRAP(32)
SIGN_WRAP(64)
SIGN_WRAP(128)
SIGN_WRAP(256)
SIGN_WRAP(512)
SIGN_WRAP(1024)

static size_t
sign_wrapper(int weak,
	const uint8_t *sign_key, size_t sign_key_len,
	const uint8_t *ctx, size_t ctx_len,
	const char *id, const uint8_t *hv, size_t hv_len,
	const uint8_t *seed, size_t seed_len,
	uint8_t *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	/* Signing key defines the degree to use. */
	if (sign_key_len == 0) {
		return 0;
	}
	unsigned head = sign_key[0];
	if ((head & 0xF0) != 0x50) {
		return 0;
	}
	unsigned logn = head & 0x0F;
	if (weak) {
		if (logn < 2 || logn > 8) {
			return 0;
		}
	} else {
		if (logn < 9 || logn > 10) {
			return 0;
		}
	}
	if (sign_key_len != FNDSA_SIGN_KEY_SIZE(logn)) {
		return 0;
	}
	if (sig == NULL) {
		return FNDSA_SIGNATURE_SIZE(logn);
	}
	if (max_sig_len < FNDSA_SIGNATURE_SIZE(logn)) {
		return 0;
	}

	/* We have checked that the degree is acceptable, the signing key
	   size is correct, and the signature will fit in the output buffer.
	   We compute the message representative mu. */
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

	if (tmp == NULL) {
		switch (logn) {
		case 6:
			return sign_64(logn,
				sign_key, mu, seed, seed_len, sig);
		case 7:
			return sign_128(logn,
				sign_key, mu, seed, seed_len, sig);
		case 8:
			return sign_256(logn,
				sign_key, mu, seed, seed_len, sig);
		case 9:
			return sign_512(logn,
				sign_key, mu, seed, seed_len, sig);
		case 10:
			return sign_1024(logn,
				sign_key, mu, seed, seed_len, sig);
		default:
			return sign_32(logn,
				sign_key, mu, seed, seed_len, sig);
		}
	} else {
		if (tmp_len < (((size_t)28 << logn) + 31)) {
			return 0;
		}
		return sign_step1(logn,
			sign_key, mu, seed, seed_len, sig, tmp);
	}
}

static size_t
sign_krm(const uint8_t *sign_key, size_t sign_key_len,
	const uint8_t *ctx, size_t ctx_len,
	const char *id, const uint8_t *hv, size_t hv_len,
	const uint8_t *seed, size_t seed_len,
	uint8_t *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	/* Signing key defines the degree to use. */
	if (sign_key_len == 0) {
		return 0;
	}
	unsigned head = sign_key[0];
	if ((head & 0xF0) != 0x50) {
		return 0;
	}
	unsigned logn = head & 0x0F;
	if (logn < 9 || logn > 10) {
		return 0;
	}
	if (sign_key_len != FNDSA_SIGN_KEY_SIZE(logn)) {
		return 0;
	}
	if (sig == NULL) {
		return FNDSA_SIGNATURE_KRM_SIZE(logn);
	}
	if (max_sig_len < FNDSA_SIGNATURE_KRM_SIZE(logn)) {
		return 0;
	}

	/* We have checked that the degree is acceptable, the signing key
	   size is correct, and the signature will fit in the output buffer.
	   We compute the message representative mu. */
	uint8_t mu[64];
	if (id != NULL && *(const uint8_t *)id == 0xFE) {
		/* External mu mode. */
		if (hv_len != 64) {
			return 0;
		}
		memcpy(mu, hv, hv_len);
	} else {
		/* In KRM, the verifying key is not included in the hash. */
		if (!fndsa_compute_mu(mu, NULL, ctx, ctx_len, id, hv, hv_len)) {
			return 0;
		}
	}

	/* Check tmp size */
	if (tmp_len < (((size_t)28 << logn) + 31)) {
		return 0;
	}

	/* Align tmp to a 32-byte boundary. */
	tmp = (void *)(((uintptr_t)tmp + 31) & ~(uintptr_t)31);

	/* We need a signing seed (40 bytes). */
	uint8_t seedbuf[SEEDBUF_LEN];
	if (!make_seedbuf(seedbuf, sign_key,
		mu, seed, seed_len, "20261007"))
	{
		return 0;
	}

#if FNDSA_STATS
	fndsa_stats_start_sign(logn, sign_key, mu, seedbuf);
#endif

	/* KRM uses the same nonce as in standard mode, but with
	   a slightly different hashing process. */
	fndsa_variant_std fvs = {
		{ &make_nonce_std, &make_c_krm, SIG_TYPE_KRM },
		mu
	};
	return sign_core(logn, sign_key + 1, &fvs.inner,
		seedbuf, SEEDBUF_LEN, sig, tmp);
}

/* see fndsa.h */
size_t
fndsa_sign(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len)
{
	return sign_wrapper(0, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		NULL, 0, sig, max_sig_len, NULL, 0);
}

/* see fndsa.h */
size_t
fndsa_sign_seeded(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len)
{
	return sign_wrapper(0, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		seed, seed_len, sig, max_sig_len, NULL, 0);
}

/* see fndsa.h */
size_t
fndsa_sign_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_wrapper(0, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		NULL, 0, sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_wrapper(0, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		seed, seed_len, sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_weak(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len)
{
	return sign_wrapper(1, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		NULL, 0, sig, max_sig_len, NULL, 0);
}

/* see fndsa.h */
size_t
fndsa_sign_weak_seeded(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len)
{
	return sign_wrapper(1, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		seed, seed_len, sig, max_sig_len, NULL, 0);
}

/* see fndsa.h */
size_t
fndsa_sign_weak_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_wrapper(1, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		NULL, 0, sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_weak_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_wrapper(1, sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		seed, seed_len, sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_krm_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_krm(sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		NULL, 0, sig, max_sig_len, tmp, tmp_len);
}

/* see fndsa.h */
size_t
fndsa_sign_krm_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len)
{
	return sign_krm(sign_key, sign_key_len,
		ctx, ctx_len, id, hv, hv_len,
		seed, seed_len, sig, max_sig_len, tmp, tmp_len);
}
