/*
 * Utility functions.
 */

#include "inner.h"

/* see inner.h */
void
hash_to_point(unsigned logn,
        const uint8_t *nonce, const uint8_t *mu, int krm, uint16_t *c)
{
	shake_context sc;
	shake_init(&sc, 256);
	shake_inject(&sc, nonce, 40);
	shake_inject(&sc, mu, 64);
	if (krm) {
		uint8_t x = 1;
		shake_inject(&sc, &x, 1);
	}
	shake_flip(&sc);

	size_t n = (size_t)1 << logn;
	size_t i = 0;
#if FNDSA_ASM_CORTEXM4
	uint8_t *sbuf = (uint8_t *)(void *)&sc;
	size_t j = 136;
#else
	uint8_t sbuf[136];
	size_t j = sizeof sbuf;
#endif
	while (i < n) {
		if (j == 136) {
#if FNDSA_ASM_CORTEXM4
			shake_extract(&sc, NULL, j);
#else
			shake_extract(&sc, sbuf, j);
#endif
			j = 0;
		}
		unsigned w = sbuf[j] | ((unsigned)sbuf[j + 1] << 8);
		j += 2;
		if (w < 61445) {
			while (w >= 12289) {
				w -= 12289;
			}
			c[i ++] = w;
		}
	}
}

#if FNDSA_AVX2
#if defined __GNUC__ || defined __clang__
#include <cpuid.h>
__attribute__((target("xsave")))
int
has_avx2(void)
{
	/* __get_cpuid_count() includes a check that CPUID is callable,
	   and that the requested leaf number is available. */
	unsigned eax, ebx, ecx, edx;
	if (__get_cpuid_count(7, 0, &eax, &ebx, &ecx, &edx)) {
		/* Check AVX2 support by the hardware. */
		if ((ebx & (1 << 5)) != 0) {
			/* Also check that YMM registers have not been
			   disabled by the OS. */
			return (_xgetbv(0) & 0x06) == 0x06;
		}
	}
	return 0;
}
#elif _MSC_VER
int
has_avx2(void)
{
	int rr[4];
	/* Check that CPUID leaf 7 is accessible. */
	__cpuid(rr, 0);
	if (rr[0] < 7) {
		return 0;
	}
	/* Check that the hardware supports AVX2. */
	__cpuidex(rr, 7, 0);
	if ((rr[1] & (1 << 5)) == 0) {
		return 0;
	}
	/* Check that the YMM registers have not been disabled by the OS. */
	return (_xgetbv(0) & 0x06) == 0x06;
}
#else
#error Missing has_avx2() implementation (not GCC/Clang/MSVC)
#endif
#endif

/* see fndsa.h */
void
fndsa_hashed_vrfykey_from_vrfykey(void *hashed_vkey,
	const void *vrfy_key, size_t vrfy_key_len)
{
	shake_context sc;
	shake_init(&sc, 256);
	shake_inject(&sc, vrfy_key, vrfy_key_len);
	shake_flip(&sc);
	shake_extract(&sc, hashed_vkey, 64);
}

/* see fndsa.h */
int
fndsa_compute_mu(void *mu, const void *hashed_vkey,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len)
{
	if ((ctx == NULL && ctx_len != 0) || ctx_len > 255) {
		return 0;
	}
	if (id == NULL) {
		id = FNDSA_HASH_ID_RAW;
	}

	shake_context sc;
	shake_init(&sc, 256);
	if (hashed_vkey == NULL) {
		static const uint8_t zz[64] = { 0 };
		hashed_vkey = &zz;
	}
	shake_inject(&sc, hashed_vkey, 64);
	const uint8_t *id_u = (const uint8_t *)id;
	size_t id_len;

	/* Header byte: 0x00 for raw, 0x01 for pre-hashed. */
	uint8_t xb;
	switch (id_u[0]) {
	case 0x00:
		/* Raw message. */
		xb = 0;
		shake_inject(&sc, &xb, 1);
		id_len = 0;
		break;
	case 0x06:
		/* Pre-hashed message. id is a DER-encoded OID; first
		   byte is the tag (0x06), second byte should be the
		   length (we tolerate only lengths up to 127, which is
		   more than enough for normal OIDs). */
		id_len = id_u[1];
		if (id_len > 127) {
			return 0;
		}
		id_len += 2;  /* for tag and length */
		xb = 1;
		shake_inject(&sc, &xb, 1);
		break;
	default:
		/* Other values are invalid, including the "external mu"
		   identifier, which cannot be used for this function. */
		return 0;
	}

	xb = (uint8_t)ctx_len;
	shake_inject(&sc, &xb, 1);
	shake_inject(&sc, ctx, ctx_len);
	shake_inject(&sc, id, id_len);
	shake_inject(&sc, hv, hv_len);
	shake_flip(&sc);
	shake_extract(&sc, mu, 64);
	return 1;
}

/* see fndsa.h */
int
fndsa_compute_mu_start(const void *hashed_vkey,
	const void *ctx, size_t ctx_len,
	void (*shake_cb)(void *state, const uint8_t *data, size_t data_len),
	void *shake_cb_state)
{
	if ((ctx == NULL && ctx_len != 0) || ctx_len > 255) {
		return 0;
	}
	if (hashed_vkey != NULL) {
		shake_cb(shake_cb_state, hashed_vkey, 64);
	}
	uint8_t xb = 0;
	shake_cb(shake_cb_state, &xb, 1);
	xb = (uint8_t)ctx_len;
	shake_cb(shake_cb_state, &xb, 1);
	if (ctx_len > 0) {
		shake_cb(shake_cb_state, ctx, ctx_len);
	}
	return 1;
}
