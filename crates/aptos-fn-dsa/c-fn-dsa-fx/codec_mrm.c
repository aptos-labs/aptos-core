/*
 * Encoding/decoding primitives for message recovery mode.
 */

#include "inner.h"

/* Encoding format:
     s[0..11] -> z = \sum_{i=0}^{11} s[i]*q^i
   z = \sum_{i=0}^{162} z[i]*2^i  (bits in low-to-high order)
   Bits z[i] are written in increasing order (i = 0 to 162).
   Bits in an output bytes are filled in increasing order (2^0 to 2^7).

   These encoding functions are for MRM; they operate on non-secret data. */

/* Read the next 32 bits from the reader. */
static inline uint32_t
bits_read32(bits_reader *br)
{
	uint32_t x = 0;
	for (unsigned i = 0; i < 32; i += 8) {
		if (br->src_len != 0) {
			x |= ((uint32_t)*br->src ++) << i;
			br->src_len --;
		} else if (br->pad) {
			x |= (uint32_t)1 << i;
			br->pad = 0;
		}
	}
	unsigned acc_len = br->acc_len;
	uint32_t r = br->acc | (x << acc_len);
	br->acc = (x >> 16) >> (16 - acc_len);
	return r;
}

/* Read the next 27 bits from the reader. */
static inline uint32_t
bits_read27(bits_reader *br)
{
	/* We need 3 or 4 bytes, depending on how many bits are already
	   in acc. */
	unsigned acc_len = br->acc_len;
	unsigned num = (acc_len < 3) ? 32 : 24;
	uint32_t x = 0;
	for (unsigned i = 0; i < num; i += 8) {
		if (br->src_len != 0) {
			x |= ((uint32_t)*br->src ++) << i;
			br->src_len --;
		} else if (br->pad) {
			x |= (uint32_t)1 << i;
			br->pad = 0;
		}
	}
	uint32_t r = br->acc | (x << acc_len);
	br->acc = x >> (27 - acc_len);
	br->acc_len = acc_len + num - 27;
	return r & 0x07FFFFFF;
}

/* Read the next 3 bits from the reader. */
static inline unsigned
bits_read3(bits_reader *br)
{
	uint32_t acc = br->acc;
	unsigned acc_len = br->acc_len;
	if (acc_len < 3) {
		if (br->src_len != 0) {
			acc |= ((uint32_t)*br->src ++) << acc_len;
			br->src_len --;
		} else if (br->pad) {
			acc |= (uint32_t)1 << acc_len;
			br->pad = 0;
		}
		acc_len += 8;
	}
	br->acc = acc >> 3;
	br->acc_len = acc_len - 3;
	return acc & 0x07;
}

/* see inner.h */
void
bits_163_to_q12(bits_reader *br, uint16_t *d)
{
	/* p2[j] contains 2^(32*j) in radix q. */
	static const uint16_t p2[6][12] = {
		/* 2^0 */
		{     1,     0,     0,     0,     0,     0,
		      0,     0,     0,     0,     0,     0 },
		/* 2^32 */
		{ 10952,  5404,    28,     0,     0,     0,
		      0,     0,     0,     0,     0,     0 },
		/* 2^64 */
		{  5664, 11328,   757, 10115,   808,     0,
		      0,     0,     0,     0,     0,     0 },
		/* 2^96 */
		{  9545,  8205, 12013,  8364,  7115,  9580,
		  10713,     1,     0,     0,     0,     0 },
		/* 2^128 */
		{  6606,  4456,  9862,  6372,  4073,  4605,
		   4449,  9895,  2877,    53,     0,     0 },
		/* 2^160 */
		{  3569,  7659,  1896,  6410,  2451,  1333,
		   9111, 10510,  3227,  5935, 11922,  1513 },
	};

	/* We maintain in x[] a big integer in radix q. */
	uint32_t x[12] = { 0 };

	for (size_t i = 0; i < 6; i ++) {
		/* Get next input word m */
		uint32_t m;
		if (i < 5) {
			m = bits_read32(br);
		} else {
			m = bits_read3(br);
		}

		/* Add m*2^(32*i) to the current value.
		   TODO: inner loop could have a smaller range for the
		   first few outer iterations. */
		uint32_t cc = 0;
		for (size_t j = 0; j < 12; j ++) {
			uint64_t z = (uint64_t)p2[i][j] * (uint64_t)m
				+ (uint64_t)x[j] + (uint64_t)cc;
			/* TODO: if the compiler does not optimize this
			   division properly, do it manually. */
			cc = (uint32_t)(z / 12289);
			x[j] = (uint32_t)(z % 12289);
		}
	}

	for (size_t i = 0; i < 12; i ++) {
		d[i] = (uint16_t)x[i];
	}
}

/* see inner.h */
void
bits_27_to_q2(bits_reader *br, uint16_t *d)
{
	uint32_t x = bits_read27(br);
	d[0] = (uint16_t)(x % 12289);
	d[1] = (uint16_t)(x / 12289);
}

/* Write out the low x_len bits of x (little-endian order).
   The value in x MUST be less than 2^{x_len}. */
static inline int
bits_write(uint32_t x, unsigned x_len, bits_writer *bw)
{
	unsigned acc_len = bw->acc_len;

	/* Put the new bits after the current ones.
	   Truncation may occur if the total number of bits exceeds 32. */
	uint32_t z = bw->acc | (x << acc_len);
	unsigned n_len = x_len + acc_len;

	/* Get into num the number of full bytes to emit. */
	unsigned num;
	if (n_len >= 32) {
		/* We have at least 32 bits: emit 4 bytes; moreover,
		   truncation may have occured. */
		num = 4;
		n_len -= 32;
		/* Double shift to avoid UB is acc_len == 0 */
		bw->acc = (x >> 16) >> (16 - acc_len);
	} else {
		/* Less than 32 bits; emit at most 3 bytes. Truncation
		   did not occur, z contains all the bits. */
		num = n_len >> 3;
		bw->acc = z >> (num << 3);
		n_len &= 0x07;
	}
	bw->acc_len = n_len;

	/* Encode num full bytes from z. */
	while (num -- > 0) {
		if (bw->dst_len == 0) {
			/* If we run out of space, then z must contain
			   only zeros (and optionally the padding bit). */
			if (bw->pad) {
				bw->pad = 0;
				return z == 1;
			} else {
				return z == 0;
			}
		} else {
			bw->dst_len --;
			*bw->dst ++ = (uint8_t)z;
			z >>= 8;
		}
	}
	return 1;
}

/* see inner.h */
int
bits_q12_to_163(const uint16_t *s, bits_writer *bw)
{
	uint32_t x[7];

	/* Decode the 12 values into a 192-bit integer x */
	x[0] = (uint32_t)s[11];
	for (size_t i = 1; i < 12; i ++) {
		uint32_t cc = (uint32_t)s[11 - i];
		for (size_t j = 0; j < ((i + 1) >> 1); j ++) {
			uint64_t z = (uint64_t)x[j] * Q_MOD + (uint64_t)cc;
			x[j] = (uint32_t)z;
			cc = (uint32_t)(z >> 32);
		}
		x[(i + 1) >> 1] = cc;
	}

	/* Value should fit in 163 bits, hence top word should contain at
	   most 3 bits. */
	if (x[5] > 0x07) {
		return 0;
	}

	/* Write the 163 bits. */
	for (size_t i = 0; i < 5; i ++) {
		if (!bits_write(x[i], 32, bw)) {
			return 0;
		}
	}
	return bits_write(x[5], 3, bw);
}

/* see inner.h */
int
bits_q2_to_27(const uint16_t *s, bits_writer *bw)
{
	uint32_t x;

	/* Decode the 2 value into an integer less than q^2. */
	x = (uint32_t)s[0] + (uint32_t)s[1] * Q_MOD;

	/* Value is supposed to fit in 27 bits. */
	if (x > 0x07FFFFFF) {
		return 0;
	}
	return bits_write(x, 27, bw);
}

/* see inner.h */
int
bits_q12_msb(const uint16_t *s)
{
	uint32_t x[7];

	/* Decode the 12 values into a 192-bit integer x */
	x[0] = (uint32_t)s[11];
	for (size_t i = 1; i < 12; i ++) {
		uint32_t cc = (uint32_t)s[11 - i];
		for (size_t j = 0; j < ((i + 1) >> 1); j ++) {
			uint64_t z = (uint64_t)x[j] * Q_MOD + (uint64_t)cc;
			x[j] = (uint32_t)z;
			cc = (uint32_t)(z >> 32);
		}
		x[(i + 1) >> 1] = cc;
	}

	/* Value should fit in 163 bits, hence top word should contain at
	   most 3 bits. */
	if (x[5] > 0x07) {
		return -1;
	}

	/* Find highest non-zero word. */
	for (int j = 5; j >= 0; j --) {
		if (x[j] != 0) {
			return (j << 5) + 31 - lzcnt_nonzero(x[j]);
		}
	}
	return -1;
}

/* see inner.h */
int
bits_q2_msb(const uint16_t *s)
{
	uint32_t x = (uint32_t)s[0] + (uint32_t)s[1] * Q_MOD;
	if (x == 0 || x > 0x07FFFFFF) {
		return -1;
	}
	return 31 - lzcnt_nonzero(x);
}
