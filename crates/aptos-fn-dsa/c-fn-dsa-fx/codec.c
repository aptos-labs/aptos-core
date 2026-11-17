/*
 * Encoding/decoding primitives.
 */

#include "inner.h"

/* see inner.h */
size_t
trim_i8_encode(unsigned logn, const int8_t *f, unsigned nbits, uint8_t *d)
{
	size_t n = (size_t)1 << logn;
	if (nbits == 8) {
		memmove(d, f, n);
		return n;
	}
	size_t j = 0;
	uint32_t acc = 0;
	unsigned acc_len = 0;
	uint32_t mask = ((uint32_t)1 << nbits) - 1;
	for (size_t i = 0; i < n; i ++) {
		acc |= ((uint32_t)f[i] & mask) << acc_len;
		acc_len += nbits;
		if (acc_len >= 8) {
			d[j ++] = (uint8_t)acc;
			acc >>= 8;
			acc_len -= 8;
		}
	}
	return j;
}

/* see inner.h */
size_t
trim_i8_decode(unsigned logn, const uint8_t *d, int8_t *f, unsigned nbits)
{
	size_t needed = ((size_t)nbits << logn) >> 3;
	if (logn >= 3) {
		if (nbits == 6) {
			size_t n = (size_t)1 << logn;
			for (size_t i = 0, j = 0; i < n; i += 8, j += 6) {
				uint32_t x0 = (uint32_t)d[j + 0]
					| ((uint32_t)d[j + 1] << 8)
					| ((uint32_t)d[j + 2] << 16)
					| ((uint32_t)d[j + 3] << 24);
				uint32_t x1 = (uint32_t)d[j + 4]
					| ((uint32_t)d[j + 5] << 8);
				uint32_t y0 = x0 << 26;
				uint32_t y1 = x0 << 20;
				uint32_t y2 = x0 << 14;
				uint32_t y3 = x0 <<  8;
				uint32_t y4 = x0 <<  2;
				uint32_t y5 = (x1 << 28) | (x0 >> 4);
				uint32_t y6 = x1 << 22;
				uint32_t y7 = x1 << 16;
				f[i + 0] = *(int32_t *)&y0 >> 26;
				f[i + 1] = *(int32_t *)&y1 >> 26;
				f[i + 2] = *(int32_t *)&y2 >> 26;
				f[i + 3] = *(int32_t *)&y3 >> 26;
				f[i + 4] = *(int32_t *)&y4 >> 26;
				f[i + 5] = *(int32_t *)&y5 >> 26;
				f[i + 6] = *(int32_t *)&y6 >> 26;
				f[i + 7] = *(int32_t *)&y7 >> 26;
			}
			return needed;
		}
		if (nbits == 5) {
			size_t n = (size_t)1 << logn;
			for (size_t i = 0, j = 0; i < n; i += 8, j += 5) {
				uint32_t x0 = (uint32_t)d[j + 0]
					| ((uint32_t)d[j + 1] << 8)
					| ((uint32_t)d[j + 2] << 16)
					| ((uint32_t)d[j + 3] << 24);
				uint32_t x1 = (uint32_t)d[j + 4];
				uint32_t y0 = x0 << 27;
				uint32_t y1 = x0 << 22;
				uint32_t y2 = x0 << 17;
				uint32_t y3 = x0 << 12;
				uint32_t y4 = x0 <<  7;
				uint32_t y5 = x0 <<  2;
				uint32_t y6 = (x1 << 29) | (x0 >> 3);
				uint32_t y7 = x1 << 24;
				f[i + 0] = *(int32_t *)&y0 >> 27;
				f[i + 1] = *(int32_t *)&y1 >> 27;
				f[i + 2] = *(int32_t *)&y2 >> 27;
				f[i + 3] = *(int32_t *)&y3 >> 27;
				f[i + 4] = *(int32_t *)&y4 >> 27;
				f[i + 5] = *(int32_t *)&y5 >> 27;
				f[i + 6] = *(int32_t *)&y6 >> 27;
				f[i + 7] = *(int32_t *)&y7 >> 27;
			}
			return needed;
		}
	}
	size_t j = 0;
	uint32_t acc = 0;
	unsigned acc_len = 0;
	uint32_t mask1 = (1 << nbits) - 1;
	uint32_t mask2 = 1 << (nbits - 1);
	for (size_t i = 0; i < needed; i ++) {
		acc |= (uint32_t)d[i] << acc_len;
		acc_len += 8;
		while (acc_len >= nbits) {
			uint32_t w = acc & mask1;
			acc >>= nbits;
			acc_len -= nbits;
			w |= -(w & mask2);
			f[j ++] = (int8_t)*(int32_t *)&w;
		}
	}
	return needed;
}

/* see inner.h */
size_t
mqpoly_encode(unsigned logn, const uint16_t *h, uint8_t *d)
{
	size_t n = (size_t)1 << logn;
	size_t j = 0;
	for (size_t i = 0; i < n; i += 4) {
		uint32_t h0 = h[i + 0];
		uint32_t h1 = h[i + 1];
		uint32_t h2 = h[i + 2];
		uint32_t h3 = h[i + 3];
		d[j + 0] = (uint8_t)h0;
		d[j + 1] = (uint8_t)((h0 >> 8) | (h1 << 6));
		d[j + 2] = (uint8_t)(h1 >> 2);
		d[j + 3] = (uint8_t)((h1 >> 10) | (h2 << 4));
		d[j + 4] = (uint8_t)(h2 >> 4);
		d[j + 5] = (uint8_t)((h2 >> 12) | (h3 << 2));
		d[j + 6] = (uint8_t)(h3 >> 6);
		j += 7;
	}
	return j;
}

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
size_t
mqpoly_decode(unsigned logn, const uint8_t *d, uint16_t *h)
{
	size_t n = (size_t)1 << logn;
	size_t j = 0;
	uint32_t ov = 0xFFFFFFFF;
	for (size_t i = 0; i < n; i += 4) {
		uint32_t d0 = d[j + 0];
		uint32_t d1 = d[j + 1];
		uint32_t d2 = d[j + 2];
		uint32_t d3 = d[j + 3];
		uint32_t d4 = d[j + 4];
		uint32_t d5 = d[j + 5];
		uint32_t d6 = d[j + 6];
		j += 7;
		uint32_t h0 = (d0 | (d1 << 8)) & 0x3FFF;
		uint32_t h1 = ((d1 >> 6) | (d2 << 2) | (d3 << 10)) & 0x3FFF;
		uint32_t h2 = ((d3 >> 4) | (d4 << 4) | (d5 << 12)) & 0x3FFF;
		uint32_t h3 = (d5 >> 2) | (d6 << 6);
		h[i + 0] = h0;
		h[i + 1] = h1;
		h[i + 2] = h2;
		h[i + 3] = h3;
		ov &= h0 - 12289;
		ov &= h1 - 12289;
		ov &= h2 - 12289;
		ov &= h3 - 12289;
	}
	if ((ov >> 16) == 0) {
		return 0;
	} else {
		return j;
	}
}
#endif

/* see inner.h */
int
comp_encode(unsigned logn, const int16_t *s, uint8_t *d, size_t dlen)
{
	size_t n = (size_t)1 << logn;
	uint32_t acc = 0;
	unsigned acc_len = 0;
	size_t j = 0;
	for (size_t i = 0; i < n; i ++) {
		/* Invariant: acc_len <= 7 */
		int32_t x = s[i];

		/* Get sign mask and absolute value. */
		uint32_t sw = (uint32_t)(x >> 16);
		uint32_t w = ((uint32_t)x ^ sw) - sw;
		if (w > B_INF) {
			return 0;
		}

		/* Encode sign bit and low 7 bits of the absolute value. */
		acc |= ((sw & 1) | ((w & 0x7F) << 1)) << acc_len;
		acc_len += 8;

		/* Encode the high bits. Since |x| <= 2047, the high bits
		   have a value in [0,15], hence we add at most 16 bits
		   (actual range is lower because B_INF < 2047, but
		   the code would be good up to 2047 anyway). */
		unsigned wh = w >> 7;
		acc |= (uint32_t)1 << (acc_len + wh);
		acc_len += wh + 1;

		/* We appended at most 8 + 15 + 1 = 24 bits, so the total
		   accumulated length in acc is at most 31 bits. */
		while (acc_len >= 8) {
			if (j >= dlen) {
				return 0;
			}
			d[j ++] = (uint8_t)acc;
			acc >>= 8;
			acc_len -= 8;
		}
	}

	/* Flush remaining bits (if any). */
	if (acc_len > 0) {
		if (j >= dlen) {
			return 0;
		}
		d[j ++] = (uint8_t)acc;
	}

	/* Pad with zeros. */
	while (j < dlen) {
		d[j ++] = 0;
	}
	return 1;
}

#if !FNDSA_ASM_CORTEXM4
/* see inner.h */
int
comp_decode(unsigned logn, const uint8_t *d, size_t dlen, int16_t *s)
{
	size_t n = (size_t)1 << logn;
	uint32_t acc = 0;
	unsigned acc_len = 0;
	size_t j = 0;
	for (size_t i = 0; i < n; i ++) {
		/* Invariant: acc_len <= 7 */

		/* Get next 8 bits and split into sign bit (t) and low
		   bits of the absolute value (m). */
		if (j >= dlen) {
			return 0;
		}
		acc |= (uint32_t)d[j ++] << acc_len;
		uint32_t t = acc & 1;
		uint32_t m = (acc >> 1) & 0x7F;
		acc >>= 8;

		/* Get next bits until a 1 is reached. Since there are at
		   most 6 bits of value zero before the next 1 (in a valid
		   encoding), we will need at most one extra bytes. */
		if (acc == 0) {
			if (j >= dlen) {
				return 0;
			}
			acc |= (uint32_t)d[j ++] << acc_len;
			acc_len += 8;
			if (acc == 0) {
				return 0;
			}
		}
		uint32_t tz = ctz32_nonzero(acc);
		/* Since acc is non-zero but fits on 15 bits, maximum value
		   of tz is 14. */
		m += tz << 7;
		if (m > B_INF) {
			return 0;
		}
		acc >>= tz + 1;
		acc_len -= tz + 1;

		/* Reject "-0" (which is an invalid encoding). */
		if (m == 0 && t != 0) {
			return 0;
		}

		m = (m ^ -t) + t;
		s[i] = (int16_t)*(int32_t *)&m;
	}

	/* Check that the unused bits are all zero. */
	if (acc != 0) {
		return 0;
	}
	while (j < dlen) {
		if (d[j ++] != 0) {
			return 0;
		}
	}
	return 1;
}
#endif
