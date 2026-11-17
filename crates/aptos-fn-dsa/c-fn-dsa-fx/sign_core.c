/*
 * Core signature generation.
 */

#include "sign_inner.h"

/* see sign_inner.h */
TARGET_SSE2 TARGET_NEON
size_t
sign_core(unsigned logn,
	const uint8_t *sign_key_fgF, fndsa_variant *fv,
	const uint8_t *seed, size_t seed_len, uint8_t *sig, void *tmp)
{
	/* Output value is 0 on error, or the signature length on success. */
	size_t ret = 0;
	size_t n = (size_t)1 << logn;
	unsigned nbits = fg_nbits(logn);

	/* Storage format of F is already an array of int8_t.
	   Note: we do not check that all coefficients of F[] are in
	   [-127, +127]: the private key is assumed to be correct, and
	   our API contract does not involve checking it. It is not actually
	   a problem if a coefficient of F happens to be equal to -128;
	   at worst, an invalid private key may lead to repeated failure
	   to sign, which will exit after 27 attempts. */
	size_t flen = (nbits << logn) >> 3;
	int8_t *F = (int8_t *)(sign_key_fgF + (flen << 1));

	/* We will use the output buffer (sig[]) to store some values:
	    - The nonce (40 bytes) goes directly into sig + 1. This is
	      where it will appear in the final signature anyway.
	    - When computing f*adj(f) + g*adj(g), the value is obtained
	      modulo q', then adjusted by recomputing it modulo q. Whether
	      an adjustment to any individual value was needed is stored
	      in a bitfield, that starts at sig + 41. These bits allow
	      recomputing f*adj(f) + g*adj(g) later on by working only
	      modulo q'. The signature buffer is always large enough for
	      that bitfield. */

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
		fv->make_nonce(fv, logn, &ss.pc, sig);

#if FNDSA_STATS
		fndsa_stats_start_iteration(counter, sig + 1);
#endif

		uint16_t *tmp0 = (uint16_t *)tmp;
		uint16_t *tmp2 = tmp0 + n;
		uint16_t *tmp4 = tmp0 + 2 * n;
		uint16_t *tmp6 = tmp0 + 3 * n;
		uint16_t *tmp12 = tmp0 + 6 * n;
		uint16_t *tmp18 = tmp0 + 9 * n;
		uint16_t *tmp20 = tmp0 + 10 * n;
		uint16_t *tmp22 = tmp0 + 11 * n;
		uint16_t *tmp24 = tmp0 + 12 * n;
		uint16_t *tmp26 = tmp0 + 13 * n;

		/* Step 1
		   Compute f*adj(f) + g*adj(g) (FFT) and
		   c*f mod q (normalized) */

		/* tmp4  <- f  (n bytes, into first half of tmp4) */
		(void)trim_i8_decode(logn, sign_key_fgF, (int8_t *)tmp4, nbits);
		uint32_t sqnf = mqpoly_sqnorm_small(logn, (int8_t *)tmp4);

		/* tmp0  <- f mod q'  (NTT mod q') */
		mq2poly_small_to_mod(logn, (int8_t *)tmp4, tmp0);
		mq2poly_NTT(logn, tmp0);

		/* tmp4  <- g  (n bytes, into second half of tmp4) */
		(void)trim_i8_decode(logn,
			sign_key_fgF + flen, (int8_t *)tmp4 + n, nbits);
		uint32_t sqng = mqpoly_sqnorm_small(logn, (int8_t *)tmp4 + n);

		/* If ||(g,-f)|| > 1.17*sqrt(q) then the private key is
		   invalid, and that could lead us to incorrect computations
		   for f*adj(f) + g*adj(g), which could conceptually lead
		   to a quasi-infinite loop in SamplerZ. We avoid this issue
		   by checking the norm here. */
		if (sqnf + sqng > 16822) {
			return 0;
		}

		/* tmp2  <- g mod q'  (NTT mod q') */
		mq2poly_small_to_mod(logn, (int8_t *)tmp4 + n, tmp2);
		mq2poly_NTT(logn, tmp2);

		/* tmp0  <- f*adj(f) + g*adj(g) mod q' (normalized) */
		mq2poly_muladj_x2_ntt(logn, tmp0, tmp2);
		mq2poly_iNTT(logn, tmp0);
		mq2poly_mod_to_signed(logn, tmp0);

		/* tmp26 <- f mod q  (NTT mod q) */
		mqpoly_small_to_int(logn, (int8_t *)tmp4, tmp26);
		mqpoly_int_to_ntt(logn, tmp26);

		/* tmp2  <- g mod q  (NTT mod q) */
		mqpoly_small_to_int(logn, (int8_t *)tmp4 + n, tmp2);
		mqpoly_int_to_ntt(logn, tmp2);

		/* tmp2  <- f*adj(f) + g*adj(g) mod q (normalized) */
		mqpoly_muladj_x2_ntt(logn, tmp2, tmp26);
		mqpoly_ntt_to_int(logn, tmp2);
		mqpoly_int_to_signed(logn, tmp2);

		/* tmp0 and tmp2 contain f*adj(f) + g*adj(g) mod q' and
		   mod q, respectively (both normalized). We adjust the
		   value in tmp0 so that it matches the value in tmp2.
		   The adjustment bits are saved in sig + 41. */
		mq2mqpoly_adjust_save(logn - 1,
			(int16_t *)tmp0, (int16_t *)tmp2, sig + 41);

		/* tmp12 <- f*adj(f) + g*adj(g)  (FFT, self-adj) */
		xpoly_FFT_int16_selfadj(logn, (rx *)tmp12, (int16_t *)tmp0);

		/* tmp24 <- c*f mod q  (normalized) */
		fv->make_c(fv, logn, sig, tmp24);
#if FNDSA_STATS
		fndsa_stats_hash_to_point(logn, tmp24);
#endif
		mqpoly_ext_to_int(logn, tmp24);
		mqpoly_int_to_ntt(logn, tmp24);
		mqpoly_mul_ntt(logn, tmp24, tmp26);
		mqpoly_ntt_to_int(logn, tmp24);
		mqpoly_int_to_signed(logn, tmp24);

		/* Step 2
		   ffSampling on t1 */
		ss.dz = (int16_t *)tmp24;
		x_ffsamp_fft_right(&ss, FLAG_INIT0, tmp);

		/* Step 3
		   We have t1 - z1 (FFT) in tmp0. */

		/* tmp20 <- f mod q  (NTT mod q)
		   Value is already in tmp26. */
		memcpy(tmp20, tmp26, n * sizeof(uint16_t));

		/* tmp26 <- 1/f mod q  (NTT mod q) */
		mqpoly_inv_ntt(logn, tmp26, tmp18);

		/* tmp22 <- g mod q  (NTT mod q) */
		(void)trim_i8_decode(logn,
			sign_key_fgF + flen, (int8_t *)tmp18, nbits);
		mqpoly_small_to_int(logn, (int8_t *)tmp18, tmp22);
		mqpoly_int_to_ntt(logn, tmp22);

		/* tmp12 <- F mod q  (NTT mod q) */
		mqpoly_small_to_int(logn, F, tmp12);
		mqpoly_int_to_ntt(logn, tmp12);

		/* tmp18 <- G = (q + g*F)/f mod q  (NTT mod q) */
		memcpy(tmp18, tmp22, n * sizeof(uint16_t));
		mqpoly_mul_ntt(logn, tmp18, tmp12);
		mqpoly_mul_ntt(logn, tmp18, tmp26);

		/* tmp12 <- F*adj(f) + G*adj(g) mod q  (normalized) */
		mqpoly_muladj_add_muladj(logn, tmp12, tmp20, tmp18, tmp22);
		mqpoly_ntt_to_int(logn, tmp12);
		mqpoly_int_to_signed(logn, tmp12);

		/* tmp18 <- G  (normalized) */
		mqpoly_ntt_to_int(logn, tmp18);
		mqpoly_int_to_signed(logn, tmp18);

		/* tmp18 <- G mod q'  (NTT mod q') */
		mq2poly_signed_to_mod(logn, tmp18);
		mq2poly_NTT(logn, tmp18);

		/* tmp22 <- g mod q'  (NTT mod q') */
		(void)trim_i8_decode(logn,
			sign_key_fgF + flen, (int8_t *)tmp26, nbits);
		mq2poly_small_to_mod(logn, (int8_t *)tmp26, tmp22);
		mq2poly_NTT(logn, tmp22);

		/* tmp18 <- G*adj(g) mod q'  (NTT mod q') */
		mq2poly_muladj_ntt(logn, tmp18, tmp22);

		/* tmp20 <- f mod q'  (NTT mod q') */
		(void)trim_i8_decode(logn,
			sign_key_fgF, (int8_t *)tmp26, nbits);
		mq2poly_small_to_mod(logn, (int8_t *)tmp26, tmp20);
		mq2poly_NTT(logn, tmp20);

		/* tmp26 <- f*adj(f) + g*adj(g) mod q'  (normalized) */
		memcpy(tmp26, tmp20, n * sizeof(uint16_t));
		mq2poly_muladj_x2_ntt(logn, tmp26, tmp22);
		mq2poly_iNTT(logn, tmp26);
		mq2poly_mod_to_signed(logn, tmp26);

		/* Adjust value in tmp26 to contain the true value of
		   f*adj(f) + g*adj(g). We reuse the adjustment bits
		   that were previously saved in sig + 41. */
		mq2mqpoly_adjust(logn - 1, (int16_t *)tmp26, sig + 41);

		/* tmp22 <- F mod q'  (NTT mod q') */
		mq2poly_small_to_mod(logn, F, tmp22);
		mq2poly_NTT(logn, tmp22);

		/* tmp18 <- F*adj(f) + G*adj(g) mod q'  (normalized) */
		mq2poly_muladj_ntt(logn, tmp22, tmp20);
		mq2poly_add(logn, tmp18, tmp22);
		mq2poly_iNTT(logn, tmp18);
		mq2poly_mod_to_signed(logn, tmp18);

		/* tmp20 <- F*adj(f) + G*adj(g)  (int32) */
		mq2mqpoly_CRT(logn, (int32_t *)tmp20,
			(int16_t *)tmp12, (int16_t *)tmp18);

		/* We now have:
		     tmp20    F*adj(f) + G*adj(g)  (int32)
		     tmp26    f*adj(f) + g*adj(g)  (int16)
		   We want to convert both to FFT, then multiply t1-z1
		   (currently in tmp0, size 12*n bytes) with
		   F*adj(f) + G*adj(g), and divide it by f*adj(f) + g*adj(g).
		   At the end, we _also_ want to have f*adj(f) + g*adj(g)
		   (in FFT format) in tmp12. The simplest way would be:
		     - tmp12 <- FFT(F*adj(f) + G*adj(g) to FFT) (size: 12*n)
		     - tmp0 <- tmp0 * tmp12
		     - tmp12 <- FFT(f*adj(f) + g*adj(g) to FFT) (size: 6*n)
		     - tmp0 <- tmp0 / tmp12
		   However, this makes the values in tmp0 after the
		   multiplication by tmp12 larger than they could/should be,
		   and it is detrimental to the range analysis. We thus
		   want to do the division first. But since the FFT
		   representation of F*adj(f) + G*adj(g) is large (12*n bytes),
		   we do not have the room to keep FFT(f*adj(f) + g*adj(g))
		   along, hence we must compute it twice:
		     - tmp12 <- FFT(f*adj(f) + g*adj(g) to FFT) (size: 6*n)
		     - tmp0 <- tmp0 / tmp12
		     - tmp12 <- FFT(F*adj(f) + G*adj(g) to FFT) (size: 12*n)
		     - tmp0 <- tmp0 * tmp12
		     - tmp12 <- FFT(f*adj(f) + g*adj(g) to FFT) (size: 6*n)
		   The overhead of this extra FFT is not large in practice
		   (less than 70k cycles on Arm Cortex M4). */

		/* tmp12 <- f*adj(f) + g*adj(g)  (FFT, self-adj) */
		xpoly_FFT_int16_selfadj(logn, (rx *)tmp12, (int16_t *)tmp26);

		/* tmp0  <- (t1 - z1)/(f*adj(f) + g*adj(g))  (FFT) */
		xpoly_div_selfadj_spec_fft(logn, (rx *)tmp0, (rx *)tmp12);

		/* tmp12 <- F*adj(f) + G*adj(g)  (FFT) */
		xpoly_FFT_int32(logn, (rx *)tmp12, (int32_t *)tmp20);

		/* tmp0  <- (t1 - z1)*(F*adj(f) + G*adj(g))
		            /(f*adj(f) + g*adj(g))  (FFT) */
		xpoly_mul_fft(logn, (rx *)tmp0, (rx *)tmp12);

		/* tmp12 <- f*adj(f) + g*adj(g)  (FFT, self-adj) */
		xpoly_FFT_int16_selfadj(logn, (rx *)tmp12, (int16_t *)tmp26);

		/* tmp20 <- c  (NTT mod q) */
		fv->make_c(fv, logn, sig, tmp20);
		mqpoly_ext_to_int(logn, tmp20);
		mqpoly_int_to_ntt(logn, tmp20);

		/* tmp26 <- -F  (NTT mod q) */
		mqpoly_small_to_int(logn, F, tmp26);
		mqpoly_int_to_ntt(logn, tmp26);
		mqpoly_neg(logn, tmp26);

		/* tmp26 <- -c*F mod q  (normalized) */
		mqpoly_mul_ntt(logn, tmp26, tmp20);
		mqpoly_ntt_to_int(logn, tmp26);
		mqpoly_int_to_signed(logn, tmp26);

		/* Step 4
		   ffSampling on t0' */
		ss.dz = (int16_t *)tmp26;
		x_ffsamp_fft_left(&ss, FLAG_NOOUT, tmp);

		/* Step 5
		   Build signature: s1 = f*(z0 + r0) + F*(z1 + r1)
		   with:
		      c*f = (c*f mod q) + q*r1
		      -c*F = (-c*F mod q) + q*r0
		   This simplifies to:
		      s1 = f*(z0 + (-c*F)/q - (-c*F mod q)/q)
		         + F*(z1 + c*f/q - (c*f mod q)/q)
		      s1 = f*(z0 + (c*F mod q)/q) + F*(z1 - (c*f mod q)/q)
		   Since s1 is integral, we can do these computations
		   modulo q'.
		   We obtained z0 and z1 in tmp26 and tmp24, respectively. */

		/* tmp2  <- c mod q  (NTT mod q) */
		fv->make_c(fv, logn, sig, tmp2);
		mqpoly_ext_to_int(logn, tmp2);
		mqpoly_int_to_ntt(logn, tmp2);

		/* tmp4  <- f mod q  (NTT mod q)
		   (f[] into tmp20) */
		(void)trim_i8_decode(logn,
			sign_key_fgF, (int8_t *)tmp20, nbits);
		mqpoly_small_to_int(logn, (int8_t *)tmp20, tmp4);
		mqpoly_int_to_ntt(logn, tmp4);

		/* tmp6  <- F mod q  (NTT mod q) */
		mqpoly_small_to_int(logn, F, tmp6);
		mqpoly_int_to_ntt(logn, tmp6);

		/* tmp12 <- c*f mod q  (normalized) */
		memcpy(tmp12, tmp2, n * sizeof(uint16_t));
		mqpoly_mul_ntt(logn, tmp12, tmp4);
		mqpoly_ntt_to_int(logn, tmp12);
		mqpoly_int_to_signed(logn, tmp12);

		/* tmp6  <- c*F mod q  (normalized) */
		mqpoly_mul_ntt(logn, tmp6, tmp2);
		mqpoly_ntt_to_int(logn, tmp6);
		mqpoly_int_to_signed(logn, tmp6);

		/* tmp6  <- z0 + (c*F mod q)/q  (NTT mod q') */
		mq2poly_signed_to_mod(logn, tmp6);
		mq2poly_mulconst(logn, tmp6, MQ2_IQ);
		mq2poly_signed_to_mod(logn, tmp26);
		mq2poly_add(logn, tmp6, tmp26);
		mq2poly_NTT(logn, tmp6);

		/* tmp24 <- z1 - (c*f mod q)/q  (NTT mod q') */
		mq2poly_signed_to_mod(logn, tmp12);
		mq2poly_mulconst(logn, tmp12, MQ2_IQ);
		mq2poly_signed_to_mod(logn, tmp24);
		mq2poly_sub(logn, tmp24, tmp12);
		mq2poly_NTT(logn, tmp24);

		/* tmp0  <- f mod q'  (NTT mod q') */
		mq2poly_small_to_mod(logn, (int8_t *)tmp20, tmp0);
		mq2poly_NTT(logn, tmp0);

		/* tmp18 <- F mod q'  (NTT mod q') */
		mq2poly_small_to_mod(logn, F, tmp18);
		mq2poly_NTT(logn, tmp18);

		/* tmp6  <- s1 = f*(z0 + (c*F mod q)/q)
		               + F*(z1 - (c*f mod q)/q)  (normalized) */
		mq2poly_mul_ntt(logn, tmp6, tmp0);
		mq2poly_mul_ntt(logn, tmp24, tmp18);
		mq2poly_add(logn, tmp6, tmp24);
		mq2poly_iNTT(logn, tmp6);
		mq2poly_mod_to_signed(logn, tmp6);

		/* At this point:
		   tmp2    c mod q  (NTT mod q)
		   tmp4    f mod q  (NTT mod q)
		   tmp6    s1  (normalized) */

		/* tmp0  <- g mod q  (NTT mod q)
		   (g[] into tmp20) */
		(void)trim_i8_decode(logn,
			sign_key_fgF + flen, (int8_t *)tmp20, nbits);
		mqpoly_small_to_int(logn, (int8_t *)tmp20, tmp0);
		mqpoly_int_to_ntt(logn, tmp0);

		/* tmp0  <- h = g/f mod q  (NTT mod q) */
		mqpoly_div_ntt(logn, tmp0, tmp4, tmp20);

		/* tmp4  <- s1 mod q  (NTT mod q) */
		memcpy(tmp4, tmp6, n * sizeof(uint16_t));
		mqpoly_signed_to_int(logn, tmp4);
		mqpoly_int_to_ntt(logn, tmp4);

		/* For key recovery modes, s1 must be invertible modulo q. */
		if (fv->sig_type == SIG_TYPE_KRM) {
			if (!mqpoly_is_invertible_ntt(logn, tmp4)) {
				continue;
			}
		}

		/* tmp2  <- s0 = c - s1*h mod q  (normalized) */
		mqpoly_mul_ntt(logn, tmp4, tmp0);
		mqpoly_sub(logn, tmp2, tmp4);
		mqpoly_ntt_to_int(logn, tmp2);
		mqpoly_int_to_signed(logn, tmp2);

		/* We have the putative signature (s0, s1) in (tmp2, tmp6).
		   This signature is acceptable if the L2 and infinity
		   norms of (s0, s1) are low enough, and s1 can be encoded
		   in the allocated output size. */
		uint32_t sqn0 = mqpoly_sqnorm_binf_signed(
			logn, (int16_t *)tmp2);
		uint32_t sqn1 = mqpoly_sqnorm_binf_signed(
			logn, (int16_t *)tmp6);
		uint32_t sqn = (sqn0 + sqn1) | ((sqn0 | sqn1) & 0x80000000);
		if (!mqpoly_sqnorm_is_acceptable(logn, sqn)) {
			continue;
		}

		if (fv->sig_type == SIG_TYPE_STD) {
			size_t sig_len = FNDSA_SIGNATURE_SIZE(logn);
			if (!comp_encode(logn,
				(int16_t *)tmp6, sig + 41, sig_len - 41))
			{
				continue;
			}
			ret = sig_len;
		} else {
			size_t sx_len = FNDSA_SIGNATURE_SIZE(logn) - 41;
			size_t j = 1;
			if (fv->sig_type == SIG_TYPE_KRM) {
				/* For KRM, nonce is included in the output. */
				j += 40;
			}
			if (!comp_encode(logn,
				(int16_t *)tmp2, sig + j, sx_len))
			{
				continue;
			}
			j += sx_len;
			if (!comp_encode(logn,
				(int16_t *)tmp6, sig + j, sx_len))
			{
				continue;
			}
			ret = j + sx_len;
		}

		/* Success! */
		sig[0] = (fv->sig_type << 4) + logn;
		break;
	}
	return ret;
}
