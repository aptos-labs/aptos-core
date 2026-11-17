#ifndef FNDSA_H__
#define FNDSA_H__

#include <stddef.h>
#include <stdint.h>

/*
 * FN-DSA is parameterized by a degree ('n'), which is a power of two.
 * Standard degrees are 512 (security level I) and 1024 (security level
 * V). All APIs here specify the degree logarithmically as the 'logn'
 * parameter, an integer with a value from 2 to 10 (inclusive). Standard
 * degrees correspond to logn = 9 or 10; the two macros below make that
 * convention explicit. Lower degrees (logn = 2 to 8) are for tests and
 * research only, since they do not provide sufficient security.
 *
 * Key and signature sizes (in bytes) are shown below:
 *
 *  logn    n     sign-key  vrfy-key  signature  security
 * ------------------------------------------------------------------
 *    9    512      1345       897       666     level I (~128 bits)
 *   10   1024      2369      1793      1280     level V (~256 bits)
 *
 *    2      4        77         8        47     none
 *    3      8        89        15        52     none
 *    4     16       113        29        63     none
 *    5     32       161        57        82     none
 *    6     64       241       113       122     none
 *    7    128       417       225       200     very weak
 *    8    256       705       449       356     presumed weak
 *
 * The private key, used to generate signatures, is hereafter called the
 * "signing key". The public key, used to verify signatures, is the
 * "verifying key".
 */

#define FNDSA_LOGN_512     9
#define FNDSA_LOGN_1024   10

/*
 * Signing (private) key size, in bytes, for 2 <= logn <= 10.
 */
#define FNDSA_SIGN_KEY_SIZE(logn)   \
	(65u + ((12u - ((logn) >= 6) - ((logn) >= 8) - ((logn) >= 10)) \
	<< ((logn) - 2)))

/*
 * Verifying (public) key size, in bytes, for 2 <= logn <= 10.
 */
#define FNDSA_VRFY_KEY_SIZE(logn)   (1u + (7u << ((logn) - 2)))

/*
 * Signature size, in bytes, for 2 <= logn <= 10.
 */
#define FNDSA_SIGNATURE_SIZE(logn) \
	(44u + 3 * (256u >> (10 - (logn))) + 2 * (128u >> (10 - (logn))) \
	+ 3 * (64u >> (10 - (logn))) + 2 * (16u >> (10 - (logn))) \
	- 2 * (2u >> (10 - (logn))) - 8 * (1u >> (10 - (logn))))

/*
 * Signature size, in bytes, for the message recovery variant.
 */
#define FNDSA_SIGNATURE_MRM_SIZE(logn) \
	(1u + 2 * (FNDSA_SIGNATURE_SIZE(logn) - 41u))

/*
 * Signature size, in bytes, for the key recovery variant.
 */
#define FNDSA_SIGNATURE_KRM_SIZE(logn) \
	(41u + 2 * (FNDSA_SIGNATURE_SIZE(logn) - 41u))

/*
 * Each message to sign, or to verify, is provided as a triplet of three
 * values:
 *  - A domain separation context, which is an arbitrary binary string of
 *    length at most 255 bytes. The context pointer can be NULL if the
 *    length is zero.
 *  - An identifier for the hash function used to pre-hash the message.
 *  - The pre-hashed message.
 * The caller is responsible for pre-hashing the message; the signature
 * generation and verification work on the hash value only. A special
 * identifier is defined for a "raw" message, which is not pre-hashed:
 * the message itself is provided as "hash value".
 *
 * The hash identifier is normally a DER-encoded ASN.1 OID for the hash
 * function; it is provided in this API under a character string type,
 * though its length is actually inferred from its ASN.1 header. The
 * special identifier for a raw message is an empty string.
 */

/* Hash function identifier: none (raw message) */
#define FNDSA_HASH_ID_RAW        ""

/* Hash function identifier: SHA-256 */
#define FNDSA_HASH_ID_SHA256     "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x01"

/* Hash function identifier: SHA-384 */
#define FNDSA_HASH_ID_SHA384     "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x02"

/* Hash function identifier: SHA-512 */
#define FNDSA_HASH_ID_SHA512     "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x03"

/* Hash function identifier: SHA-512-256 */
#define FNDSA_HASH_ID_SHA512_256 "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x06"

/* Hash function identifier: SHA3-256 */
#define FNDSA_HASH_ID_SHA3_256   "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x08"

/* Hash function identifier: SHA3-384 */
#define FNDSA_HASH_ID_SHA3_384   "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x09"

/* Hash function identifier: SHA3-512 */
#define FNDSA_HASH_ID_SHA3_512   "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x0A"

/* Hash function identifier: SHAKE128 */
#define FNDSA_HASH_ID_SHAKE128   "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x0B"

/* Hash function identifier: SHAKE256 */
#define FNDSA_HASH_ID_SHAKE256   "\x06\x09\x60\x86\x48\x01\x65\x03\x04\x02\x0C"

/* Hash function identifier: external mu
   The provided message value is really the 64-byte "mu" value. The function
   fndsa_compute_mu() can help with computing that value. */
#define FNDSA_HASH_ID_EXTMU      "\xFE"

/*
 * Generate new key pair. The operating system's RNG is used as source of
 * entropy. This function is supported on Windows and on various Unix-like
 * systems (including Linux and macOS). An error (0) is reported if the RNG
 * fails; otherwise, the function succeeds and returns 1. On error, the
 * two output buffers are filled with zeros, which are not valid encoding
 * of keys.
 *
 * The new signing (private) and verifying (public) keys are written,
 * encoded, into the two provided buffers. The keys' respective sizes
 * can be obtained from the FNDSA_SIGN_KEY_SIZE and FNDSA_VRFY_KEY_SIZE
 * macros.
 *
 * Logarithmic degree MUST be between 2 and 10. Only values 9 and 10 are
 * standard.
 */
int fndsa_keygen(unsigned logn, void *sign_key, void *vrfy_key);

/*
 * This is similar to fndsa_keygen(), except that the provided 'tmp'
 * area is used for temporary buffers, with size tmp_len bytes, instead
 * of using the stack. This function is provided for the benefit of
 * small embedded system with limited RAM space. The size of the required
 * temporary area depends on the requested degree:
 *
 *    logn   min tmp_len   security
 *   -----------------------------------------
 *      9      11295       standard (level I)
 *     10      22559       standard (level V)
 *
 *      2        119       none
 *      3        207       none
 *      4        383       none
 *      5        735       none
 *      6       1439       none
 *      7       2847       very weak
 *      8       5663       presumed weak
 *
 * (Formula is: 22*n+31 bytes, for degree n = 2^logn)
 * If the temporary area is too short then an error is returned (0).
 *
 * The tmp area MUST NOT overlap with the destination buffers sign_key
 * and vrfy_key.
 */
int fndsa_keygen_temp(unsigned logn, void *sign_key, void *vrfy_key,
	void *tmp, size_t tmp_len);

#define FNDSA_KEYGEN_TMP_SIZE(logn)   ((22u << (logn)) + 31)

/*
 * Generate a new key pair from an initial seed. The process is
 * deterministic for a given seed value. The seed MUST have been generated
 * with enough entropy (i.e. 256 bits or so). By design, this function
 * never fails.
 *
 * The new signing (private) and verifying (public) keys are written,
 * encoded, into the two provided buffers. The keys' respective sizes
 * can be obtained from the FNDSA_SIGN_KEY_SIZE and FNDSA_VRFY_KEY_SIZE
 * macros.
 *
 * Logarithmic degree MUST be between 2 and 10. Only values 9 and 10 are
 * standard.
 *
 * Though the process of deriving the key pair from the seed is
 * deterministic, it is not fully specified, and may differ between
 * implementations. It may also change without notice between versions
 * of the code. Thus, applications MUST NOT store seeds instead of
 * encoded private keys.
 */
void fndsa_keygen_seeded(unsigned logn, const void *seed, size_t seed_len,
	void *sign_key, void *vrfy_key);

/*
 * This is similar to fndsa_keygen_seeded(), except that an explicit
 * temporary area is provided, to avoid bulky stack allocation. See
 * fndsa_keygen_temp() for guidance on the temporary area size. If the
 * temporary area is too small then the function fails and returns 0;
 * otherwise, it generates a key pair and returns 1.
 */
int fndsa_keygen_seeded_temp(unsigned logn,
	const void *seed, size_t seed_len, void *sign_key, void *vrfy_key,
	void *tmp, size_t tmp_len);

/*
 * Get the hashed verifying key from the signing key. This function
 * returns a pointer within the encoded signing key. If the source
 * signing key does not have a valid length, then NULL is returned. The
 * hashed verifying key has length 64 bytes.
 */
static const uint8_t *
fndsa_hashed_vrfykey_from_signkey(const void *sign_key, size_t sign_key_len)
{
	if (sign_key_len < 65) {
		return NULL;
	}
	return (const uint8_t *)sign_key + sign_key_len - 64;
}

/*
 * Get the hashed verifying key from the verifying key. This function
 * stores the hash value (64 bytes) into the provided hashed_vkey[]
 * buffer.
 */
void fndsa_hashed_vrfykey_from_vrfykey(void *hashed_vkey,
	const void *vrfy_key, size_t vrfy_key_len);

/*
 * Compute the message representative mu (64 bytes), given the hashed
 * verifying key (hashed_vkey, 64 bytes), context string (ctx, size ctx_len
 * up to 255 bytes) and raw or pre-hashed message (hv, size hv_len).
 *
 * Rules:
 *  - hashed_vkey has size 64 bytes. See fndsa_hashed_vrfykey_from_signkey()
 *    and fndsa_hashed_vrfykey_from_vrfykey() to get that value.
 *    SPECIAL: for key recovery mode (KRM), hashed_vkey may be NULL; in
 *    that case, signatures are still safe but no longer provide full BUFF
 *    protections. Signer and verifier must agree on whether the hashed
 *    verifying key is included or not. For standard mode, hashed_vkey MUST
 *    NOT be NULL.
 *  - ctx can be NULL if ctx_len is zero. ctx_len MUST be between 0 and 255,
 *    inclusive.
 *  - If id is FNDSA_HASH_ID_RAW, then the raw message itself is provided
 *    in hv (size hv_len). Otherwise, hv should point to a hash value, and
 *    id should be the ASN.1 DER-encoded OBJECT IDENTIFIER that identifies
 *    the hash function; the various FNDSA_HASH_ID_* constants contain the
 *    identifiers for some standard hash functions.
 *  - The mu[] array may overlap the other arrays (hashed_vkey, ctx, hv).
 *
 * Returned value is 1 on success, 0 on error. An error is reported if the
 * context string is too large, or if the hash identifier is syntactically
 * incorrect.
 */
int fndsa_compute_mu(void *mu, const void *hashed_vkey,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len);

/*
 * Helper for a streamed variant of fndsa_compute_mu(); it may be useful
 * when processing large messages (obtained by chunks) but using FN-DSA
 * without pre-hashing. This function injects the hashed verifying key
 * and context string into the provided shake_cb() callback (which
 * supposedly implements SHAKE256, as per the FN-DSA specification).
 * shake_cb() receives as parameters, in that order: a pointer to some
 * state value (shake_cb_state), a pointer to some more input bytes, and
 * the number of input bytes. The caller would afterwards continue the
 * SHAKE256 computation by injecting the message itself; the SHAKE256
 * output (64 bytes) will be the message representative mu.
 *
 * As in fndsa_compute_mu(), for key recovery mode, hashed_vkey may be
 * set to NULL.
 *
 * Returned value is 1 on success, 0 on error. An error is reported if the
 * context string is too large.
 */
int fndsa_compute_mu_start(const void *hashed_vkey,
	const void *ctx, size_t ctx_len,
	void (*shake_cb)(void *state, const uint8_t *data, size_t data_len),
	void *shake_cb_state);

/*
 * Sign a message.
 *    sign_key, sign_key_len   signing key (encoded)
 *    ctx, ctx_len             domain separation context (at most 255 bytes)
 *    id                       identifier for the pre-hash function
 *    hv, hv_len               pre-hashed message (or raw message)
 *    sig, sig_max_len         output buffer for the signature
 * This function returns 0 on error; error conditions include: signing key
 * cannot be decoded; signing key is for a weak degree (i.e. not 512 nor
 * 1024); system random generator failed to provide the requested random
 * bytes; output buffer is not large enough for the signature.
 *
 * If id is FNDSA_HASH_ID_EXTMU, then ctx and ctx_len are ignored (and
 * can be zero), hv_len shall be exactly 64, and hv shall point to the
 * "message representative" mu of size 64 bytes (see fndsa_compute_mu()
 * for details on how to obtain that value). Otherwise, ctx, ctx_len,
 * id, hv and hv_len shall follow the rules documented in
 * fndsa_compute_mu().
 *
 * If sig is NULL, then the signature is not generated, but the signing key
 * is still decoded; returned value is then the signature length (in bytes),
 * or 0 on error (signing key cannot be decoded or uses a weak degree). If
 * sig is not NULL, then sig_max_len should be the maximum buffer size for
 * the signature; an error (0) is returned if that length is lower than the
 * signature size implied by the degree of the signing key. If the output
 * buffer is large enough, and the signature generation succeeds (i.e. the
 * system random generator does not fail), then the signature is written
 * in sig[] and the signature length (in bytes) is returned.
 */
size_t fndsa_sign(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len);

/*
 * This is similar to fndsa_sign(), except that instead of using the
 * system random generator, all random number generation is performed
 * from an internal deterministic PRNG seeded with the provided seed
 * value (of length seed_len bytes). It is the responsibility of the
 * caller to provide a seed with sufficient entropy (preferably 320 bits
 * or more, i.e. a seed of at least 40 bytes). The FN-DSA standard uses
 * 40-byte seeds.
 *
 * This function is provided mostly for testing purposes,
 * and for bare-metal systems with no OS-provided random generator.
 */
size_t fndsa_sign_seeded(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len);

/*
 * This is similar to fndsa_sign(), except that temporary storage will
 * use the provided tmp[] area (of size tmp_len bytes) instead of stack
 * buffers. This function is provided for the benefit of small embedded
 * systems that have only small stacks. Temporary area sizes are as
 * follows:
 *
 *    logn   min tmp_len   security
 *   -----------------------------------------
 *      9      14367       standard (level I)
 *     10      28703       standard (level V)
 *
 *      2        143       none
 *      3        255       none
 *      4        479       none
 *      5        927       none
 *      6       1823       none
 *      7       3615       very weak
 *      8       7199       presumed weak
 *
 * (Formula is: 28*n+31 bytes, for degree n = 2^logn)
 *
 * An undersized temporary area triggers an error (returned value is zero).
 */
size_t fndsa_sign_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

#define FNDSA_SIGN_TMP_SIZE(logn)   ((28u << (logn)) + 31)

/*
 * Similar to fndsa_sign_seeded(), but with an explicitly provided temporary
 * area as in fndsa_sign_temp().
 */
size_t fndsa_sign_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

/*
 * The fndsa_sign_*() functions declared above require the signing key
 * degree to be secure (512 or 1024). The fndsa_sign_weak_*() functions
 * below are similar, but require the signing key degree to be weak
 * (4 to 256). These functions are meant for tests and research only.
 */
size_t fndsa_sign_weak(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len);
size_t fndsa_sign_weak_seeded(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len);
size_t fndsa_sign_weak_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);
size_t fndsa_sign_weak_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

/*
 * Expand a private key. The expanded key can then be used with
 * fndsa_signexp_temp() or fndsa_signexp_seeded_temp(). Expanded keys use
 * more RAM but improve performance, especially when generating several
 * signatures with the same signing key.
 *
 * A buffer is provided to receive the expanded key. The returned value
 * is the entry pointer to the expanded key within that buffer; this is
 * the pointer that will be provided to fndsa_signexp_temp(). The
 * buffer MUST NOT be released or moved as long as the expanded key is
 * in use.
 *
 * Expanded key size is (3*logn + 25)*n + 136 bytes, for degree n = 2^logn.
 *
 *    logn   min ekey_len  security
 *   -----------------------------------------
 *      9      26760       standard (level I)
 *     10      56456       standard (level V)
 *
 *      2        260       none
 *      3        408       none
 *      4        728       none
 *      5       1416       none
 *      6       2888       none
 *      7       6024       very weak
 *      8      12680       presumed weak
 *
 * NULL is returned on error. An error is reported if the provided signing
 * key has an invalid format, or uses an unsupported or insecure degree
 * (this function accepts only logn = 9 or 10), or if the output buffer
 * is too small.
 */
void *fndsa_expand_signkey(const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_len);

#define FNDSA_EXP_KEY_SIZE(logn)   (((3u * (logn) + 25) << (logn)) + 136)

/*
 * This function is identical to fndsa_expand_signkey(), except that
 * it expects the signing key to have a "weak" degree (logn = 2 to 8)
 * and rejects other degrees.
 */
void *fndsa_expand_signkey_weak(const void *sign_key, size_t sign_key_len,
	void *ekey, size_t ekey_len);

/*
 * Generate a signature using an expanded key. This is similar to the
 * fndsa_sign_temp() function, except that:
 *  - An expanded private key (as obtained from fndsa_expand_signkey())
 *    is used, and is provided as a single pointer (no length).
 *  - The RAM usage (size of the temporary buffer) is higher.
 *  - There is no weak/non-weak separation, since it has already been
 *    enforced at key expansion.
 *
 * Temporary area size is 34*n + 31 bytes, for degree n = 2^logn.
 *
 *    logn   min tmp_len   security
 *   -----------------------------------------
 *      9      17439       standard (level I)
 *     10      34847       standard (level V)
 *
 *      2        167       none
 *      3        303       none
 *      4        575       none
 *      5       1119       none
 *      6       2207       none
 *      7       4383       very weak
 *      8       8735       presumed weak
 *
 * Returned value is the signature size (in bytes), or 0 on error.
 */
size_t fndsa_signexp_temp(const void *exp_key,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

#define FNDSA_SIGNEXP_TMP_SIZE(logn)   ((34u << (logn)) + 31)

/*
 * Like fndsa_signexp_temp(), but with an explicit seed value.
 * Note that, by design, fndsa_signexp_seeded_temp() does NOT return
 * the same signature value as fndsa_sign_seeded_temp(), even if using
 * the same private key, input message, and seed value.
 */
size_t fndsa_signexp_seeded_temp(const void *exp_key,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

/* TODO: add an API for deriving the public key from the private key?
   The code is mostly already there. */

/*
 * Verify a signature.
 *    sig, sig_len             signature
 *    vrfy_key, vrfy_key_len   verifying key (encoded)
 *    ctx, ctx_len             domain separation context (at most 255 bytes)
 *    id                       identifier for the pre-hash function
 *    hv, hv_len               pre-hashed message (or raw message)
 * Returned value is 1 if the signature is valid, 0 otherwise. Decoding
 * errors on the verifying key also imply a returned value of 0.
 *
 * If id is FNDSA_HASH_ID_EXTMU, then ctx and ctx_len are ignored (and
 * can be zero), hv_len shall be exactly 64, and hv shall point to the
 * "message representative" mu of size 64 bytes (see fndsa_compute_mu()
 * for details on how to obtain that value). Otherwise, ctx, ctx_len,
 * id, hv and hv_len shall follow the rules documented in
 * fndsa_compute_mu().
 *
 * This function accepts only the standard degrees (512 or 1024).
 */
int fndsa_verify(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len);

/*
 * Verify a signature (weak version).
 *
 * This function is similar to fndsa_verify(), except that it accepts only
 * keys and signatures with the weak degrees (4 to 256).
 */
int fndsa_verify_weak(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len);

/*
 * The fndsa_verify_temp() and fndsa_verify_weak_temp() functions are
 * similar to fndsa_verify() and fndsa_verify_weak(), respectively,
 * except that temporary storage will use the provided tmp[] area (of
 * size tmp_len bytes) instead of stack buffers. These functions are
 * provided for the benefit of small embedded systems that have only
 * small stacks. Temporary area sizes are as follows:
 *
 *    logn   min tmp_len   security
 *   -----------------------------------------
 *      9       2079       standard (level I)
 *     10       4127       standard (level V)
 *
 *      2         47       none
 *      3         63       none
 *      4         95       none
 *      5        159       none
 *      6        287       none
 *      7        543       very weak
 *      8       1055       presumed weak
 *
 * (Formula is: 4*n+31 bytes, for degree n = 2^logn)
 *
 * An undersized temporary area triggers an error (returned value is zero).
 */
int fndsa_verify_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len);
int fndsa_verify_weak_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *tmp, size_t tmp_len);

#define FNDSA_VERIFY_TMP_SIZE(logn)   ((4u << (logn)) + 31)

/* ====================================================================== */
/*
 * Key recovery mode (KRM).
 *
 * In this mode, the signature includes both vector components. The
 * verifying key is not provided to the verifying function; instead,
 * the matching verifying key is recomputed from the signature and
 * message. It is up to the caller to then check that the reconstructed
 * verifying key is the intended one.
 *
 * In general, the hashed verifying key is NOT included in the
 * computation of the message representative mu. This removes some of
 * the BUFF properties of the scheme and makes output signatures
 * fully incompatible with standard mode; however, it enables some
 * applications in which the verifier cannot know the hash of the public
 * key before performing the verification.
 *
 * KRM users in contexts where the hashed verifying key _is_ known in
 * advance to verifiers can use fndsa_compute_mu() with an explicit
 * non-NULL hashed_vkey parameter, and inject the resulting mu value
 * with FNDSA_HASH_ID_EXTMU.
 */

/*
 * Generate a signature in key recovery mode. This operates like
 * fndsa_sign_temp(), except that the output signature is larger
 * (see the FNDSA_SIGNATURE_KRM_SIZE macro). Degree must be 512
 * or 1024. The needed temporary area size is the same as in
 * fndsa_sign_temp().
 */
size_t fndsa_sign_krm_temp(const void *sign_key, size_t sign_key_len,
        const void *ctx, size_t ctx_len,
        const char *id, const void *hv, size_t hv_len,
        void *sig, size_t max_sig_len, void *tmp, size_t tmp_len);

/*
 * Generate a signature in key recovery mode, and explicit seeding. This
 * operates like fndsa_sign_seeded_temp(), except that the output signature
 * is larger (see the FNDSA_SIGNATURE_KRM_SIZE macro). Degree must be 512
 * or 1024. The needed temporary area size is the same as in
 * fndsa_sign_seeded_temp().
 */
size_t fndsa_sign_krm_seeded_temp(const void *sign_key, size_t sign_key_len,
        const void *ctx, size_t ctx_len,
        const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
        void *sig, size_t max_sig_len, void *tmp, size_t tmp_len);

/*
 * Verify a signature in key recovery mode.
 *    sig, sig_len                 signature
 *    ctx, ctx_len                 domain separation context (at most 255 bytes)
 *    id                           identifier for the pre-hash function
 *    hv, hv_len                   pre-hashed message (or raw message)
 *    vrfy_key, max_vrfy_key_len   output for verifying key
 *    tmp, tmp_len                 temporary area
 *
 * In KRM, the verifying key is not an input but an output. The encoded
 * verifying key is recomputed from the signature and the message. The
 * reconstructed verifying key is written into the vrfy_key[] buffer,
 * whose maximum size is max_vrfy_key_len; the actual encoded key length
 * (in bytes) is returned. On error, 0 is returned. An error is reported if
 * the source signature is incorrectly encoded, or uses an unsupported
 * degree (only degrees n = 512 and 1024 are supported), or if the output
 * buffer is not large enough to receive the reconstructed verifying key.
 *
 * The temporary area (tmp, size tmp_len) has the same requirements as
 * fndsa_verify_temp().
 *
 * IMPORTANT: the caller MUST verify in some way that the reconstructed
 * verifying key is correct. In general, if the signature is correctly
 * encoded, then the function always succeeds at reconstructing a key;
 * no security is achieved if the reconstructed key is not then validated
 * in some way.
 */
size_t fndsa_verify_krm_temp(const void *sig, size_t sig_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *vrfy_key, size_t max_vrfy_key_len,
	void *tmp, size_t tmp_len);

/* ====================================================================== */
/*
 * Message recovery mode (MRM).
 *
 * In this mode, the signature includes both vector components, but not
 * the nonce. The verifying key is provided to the verifying function.
 * The nonce, and a part of the message, is encoded into the signature.
 *
 * The message consists of two elements:
 *  - A limited-size string which is included in the signature; when
 *    verifying the signature, that part is extracted from the signature
 *    and returned to the caller.
 *  - An arbitrary-length string (like in normal signatures).
 * The maximum size of the part which can be encoded in the signature
 * depends on the degree; see the FNDSA_MAX_MRM_SIZE macro. Signer and
 * verifier must agree on the size of that part.
 *
 * The arbitrary-length part of the input is internally turned into a
 * message representative mu, with the same conventions as in the
 * standard signatures.
 *
 * MRM is defined only for the standard degrees (n = 512 and n = 1024).
 */

/*
 * Get the maximum size of the message part which is embedded inside the
 * signature. This macro assumed that logn is equal to 9 or 10 (for
 * degrees n = 512 and 1024, respectively).
 */
#define FNDSA_MAX_MRM_SIZE(logn)   ((822u << ((logn) - 9)) - 11u)

/*
 * Generate an MRM signature.
 * The embedded part as size embedded_len bytes; if that size exceeds
 * the embeddable size for the used key degree, an error will be
 * reported. The other parameters otherwise have the same semantics
 * as in fndsa_sign_temp(). The temporary size requirements (tmp/tmp_len)
 * are the same as for normal signatures.
 */
size_t fndsa_sign_mrm_temp(const void *sign_key, size_t sign_key_len,
	const void *embedded, size_t embedded_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

/*
 * Generate an MRM signature, with an explicit seed.
 * This is the seeded, deterministic variant of fndsa_sign_mrm_temp().
 */
size_t fndsa_sign_mrm_seeded_temp(const void *sign_key, size_t sign_key_len,
	const void *embedded, size_t embedded_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	const void *seed, size_t seed_len,
	void *sig, size_t max_sig_len,
	void *tmp, size_t tmp_len);

/*
 * Verify a signature with message recovery.
 *
 * This is similar to fndsa_verify_temp(), except that the function
 * expects an MRM signature, and the embedded part of the message is
 * extracted and written into the provided embedded[] buffer; the
 * size of that buffer (embedded_max_len) is provided as input. See
 * the FNDSA_MAX_MRM_SIZE() macro for the maximum size an embedded part
 * may have (depending on the used degree).
 *
 * If the signature is valid and the embedded part fits in the provided
 * output buffer, then this function returns the size of the embedded
 * part as found in the signature (in bytes). Otherwise, this function
 * returns -1.
 *
 * WARNING: contrary to other verification functions, this one returns
 * -1 on error; 0 is a success (corresponding to an empty embedded part).
 */
int fndsa_verify_mrm_temp(const void *sig, size_t sig_len,
	const void *vrfy_key, size_t vrfy_key_len,
	const void *ctx, size_t ctx_len,
	const char *id, const void *hv, size_t hv_len,
	void *embedded, size_t embedded_max_len,
	void *tmp, size_t tmp_len);

/* ====================================================================== */

#endif
