/* primesieve (BIT_ARRAY, N) -- Fills the BIT_ARRAY with a mask for primes up to N.

Contributed to the GNU project by Marco Bodrato.

THE FUNCTION IN THIS FILE IS INTERNAL WITH A MUTABLE INTERFACE.
IT IS ONLY SAFE TO REACH IT THROUGH DOCUMENTED INTERFACES.
IN FACT, IT IS ALMOST GUARANTEED THAT IT WILL CHANGE OR
DISAPPEAR IN A FUTURE GNU MP RELEASE.

Copyright 2010-2012, 2015, 2016 Free Software Foundation, Inc.

This file is part of the GNU MP Library.

The GNU MP Library is free software; you can redistribute it and/or modify
it under the terms of either:

  * the GNU Lesser General Public License as published by the Free
    Software Foundation; either version 3 of the License, or (at your
    option) any later version.

or

  * the GNU General Public License as published by the Free Software
    Foundation; either version 2 of the License, or (at your option) any
    later version.

or both in parallel, as here.

The GNU MP Library is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

You should have received copies of the GNU General Public License and the
GNU Lesser General Public License along with the GNU MP Library.  If not,
see https://www.gnu.org/licenses/.  */

#include "gmp-impl.h"

#if 0
static mp_limb_t
bit_to_n (mp_limb_t bit) { return (bit*3+4)|1; }
#endif

/* id_to_n (x) = bit_to_n (x-1) = (id*3+1)|1*/
static mp_limb_t
id_to_n  (mp_limb_t id)  { return id*3+1+(id&1); }

/* n_to_bit (n) = ((n-1)&(-CNST_LIMB(2)))/3U-1 */
static mp_limb_t
n_to_bit (mp_limb_t n) { return ((n-5)|1)/3U; }

#if 0
static mp_size_t
primesieve_size (mp_limb_t n) { return n_to_bit(n) / GMP_LIMB_BITS + 1; }
#endif

#if GMP_LIMB_BITS > 61
#define SIEVE_SEED CNST_LIMB(0x3294C9E069128480)
#if GMP_LIMB_BITS == 64
/* 110bits pre-sieved mask for primes 5, 11*/
#define SIEVE_MASK1 CNST_LIMB(0x81214a1204892058)
#define SIEVE_MASKT CNST_LIMB(0xc8130681244)
/* 182bits pre-sieved mask for primes 7, 13*/
#define SIEVE_2MSK1 CNST_LIMB(0x9402180c40230184)
#define SIEVE_2MSK2 CNST_LIMB(0x0285021088402120)
#define SIEVE_2MSKT CNST_LIMB(0xa41210084421)
#define SEED_LIMIT 210
#else
#define SEED_LIMIT 202
#endif
#else
#if GMP_LIMB_BITS > 30
#define SIEVE_SEED CNST_LIMB(0x69128480)
#if GMP_LIMB_BITS == 32
/* 70bits pre-sieved mask for primes 5, 7*/
#define SIEVE_MASK1 CNST_LIMB(0x12148960)
#define SIEVE_MASK2 CNST_LIMB(0x44a120cc)
#define SIEVE_MASKT CNST_LIMB(0x1a)
#define SEED_LIMIT 120
#else
#define SEED_LIMIT 114
#endif
#else
#if GMP_LIMB_BITS > 15
#define SIEVE_SEED CNST_LIMB(0x8480)
#define SEED_LIMIT 54
#else
#if GMP_LIMB_BITS > 7
#define SIEVE_SEED CNST_LIMB(0x80)
#define SEED_LIMIT 34
#else
#define SIEVE_SEED CNST_LIMB(0x0)
#define SEED_LIMIT 24
#endif /* 7 */
#endif /* 15 */
#endif /* 30 */
#endif /* 61 */

#define SET_OFF1(m1, m2, M1, M2, off, BITS)		\
  if (off) {						\
    if (off < GMP_LIMB_BITS) {				\
      m1 = (M1 >> off) | (M2 << (GMP_LIMB_BITS - off));	\
      if (off <= BITS - GMP_LIMB_BITS) {		\
	m2 = M1 << (BITS - GMP_LIMB_BITS - off)		\
	  | M2 >> off;					\
      } else {						\
	m1 |= M1 << (BITS - off);			\
	m2 = M1 >> (off + GMP_LIMB_BITS - BITS);	\
      }							\
    } else {						\
      m1 = M1 << (BITS - off)				\
	| M2 >> (off - GMP_LIMB_BITS);			\
      m2 = M2 << (BITS - off)				\
	| M1 >> (off + GMP_LIMB_BITS - BITS);		\
    }							\
  } else {						\
    m1 = M1; m2 = M2;					\
  }

#define SET_OFF2(m1, m2, m3, M1, M2, M3, off, BITS)	\
  if (off) {						\
    if (off <= GMP_LIMB_BITS) {				\
      m1 = M2 << (GMP_LIMB_BITS - off);			\
      m2 = M3 << (GMP_LIMB_BITS - off);			\
      if (off != GMP_LIMB_BITS) {			\
	m1 |= (M1 >> off);				\
	m2 |= (M2 >> off);				\
      }							\
      if (off <= BITS - 2 * GMP_LIMB_BITS) {		\
	m3 = M1 << (BITS - 2 * GMP_LIMB_BITS - off)	\
	  | M3 >> off;					\
      } else {						\
	m2 |= M1 << (BITS - GMP_LIMB_BITS - off);	\
	m3 = M1 >> (off + 2 * GMP_LIMB_BITS - BITS);	\
      }							\
    } else if (off < 2 *GMP_LIMB_BITS) {		\
      m1 = M2 >> (off - GMP_LIMB_BITS)			\
	| M3 << (2 * GMP_LIMB_BITS - off);		\
      if (off <= BITS - GMP_LIMB_BITS) {		\
	m2 = M3 >> (off - GMP_LIMB_BITS)		\
	  | M1 << (BITS - GMP_LIMB_BITS - off);		\
	m3 = M2 << (BITS - GMP_LIMB_BITS - off);	\
	if (off != BITS - GMP_LIMB_BITS) {		\
	  m3 |= M1 >> (off + 2 * GMP_LIMB_BITS - BITS);	\
	}						\
      } else {						\
	m1 |= M1 << (BITS - off);			\
	m2 = M2 << (BITS - off)				\
	  | M1 >> (GMP_LIMB_BITS - BITS + off);		\
	m3 = M2 >> (GMP_LIMB_BITS - BITS + off);	\
      }							\
    } else {						\
      m1 = M1 << (BITS - off)				\
	| M3 >> (off - 2 * GMP_LIMB_BITS);		\
      m2 = M2 << (BITS - off)				\
	| M1 >> (off + GMP_LIMB_BITS - BITS);		\
      m3 = M3 << (BITS - off)				\
	| M2 >> (off + GMP_LIMB_BITS - BITS);		\
    }							\
  } else {						\
    m1 = M1; m2 = M2; m3 = M3;				\
  }

#define ROTATE1(m1, m2, BITS)			\
  do {						\
    mp_limb_t __tmp;				\
    __tmp = m1 >> (2 * GMP_LIMB_BITS - BITS);	\
    m1 = (m1 << (BITS - GMP_LIMB_BITS)) | m2;	\
    m2 = __tmp;					\
  } while (0)

#define ROTATE2(m1, m2, m3, BITS)		\
  do {						\
    mp_limb_t __tmp;				\
    __tmp = m2 >> (3 * GMP_LIMB_BITS - BITS);	\
    m2 = m2 << (BITS - GMP_LIMB_BITS * 2)	\
      | m1 >> (3 * GMP_LIMB_BITS - BITS);	\
    m1 = m1 << (BITS - GMP_LIMB_BITS * 2) | m3;	\
    m3 = __tmp;					\
  } while (0)

static mp_limb_t
fill_bitpattern (mp_ptr bit_array, mp_size_t limbs, mp_limb_t offset)
{
#ifdef SIEVE_2MSK2
  mp_limb_t m11, m12, m21, m22, m23;

  if (offset == 0) { /* This branch is not needed. */
    m11 = SIEVE_MASK1;
    m12 = SIEVE_MASKT;
    m21 = SIEVE_2MSK1;
    m22 = SIEVE_2MSK2;
    m23 = SIEVE_2MSKT;
  } else { /* correctly handle offset == 0... */
    m21 = offset % 110;
    SET_OFF1 (m11, m12, SIEVE_MASK1, SIEVE_MASKT, m21, 110);
    offset %= 182;
    SET_OFF2 (m21, m22, m23, SIEVE_2MSK1, SIEVE_2MSK2, SIEVE_2MSKT, offset, 182);
  }
  /* THINK: Consider handling odd values of 'limbs' outside the loop,
     to have a single exit condition. */
  do {
    bit_array[0] = m11 | m21;
    if (--limbs == 0)
      break;
    ROTATE1 (m11, m12, 110);
    bit_array[1] = m11 | m22;
    bit_array += 2;
    ROTATE1 (m11, m12, 110);
    ROTATE2 (m21, m22, m23, 182);
  } while (--limbs != 0);
  return 4;
#else
#ifdef SIEVE_MASK2
  mp_limb_t mask, mask2, tail;

  if (offset == 0) { /* This branch is not needed. */
    mask = SIEVE_MASK1;
    mask2 = SIEVE_MASK2;
    tail = SIEVE_MASKT;
  } else { /* correctly handle offset == 0... */
    offset %= 70;
    SET_OFF2 (mask, mask2, tail, SIEVE_MASK1, SIEVE_MASK2, SIEVE_MASKT, offset, 70);
  }
  /* THINK: Consider handling odd values of 'limbs' outside the loop,
     to have a single exit condition. */
  do {
    bit_array[0] = mask;
    if (--limbs == 0)
      break;
    bit_array[1] = mask2;
    bit_array += 2;
    ROTATE2 (mask, mask2, tail, 70);
  } while (--limbs != 0);
  return 2;
#else
  MPN_FILL (bit_array, limbs, CNST_LIMB(0));
  return 0;
#endif
#endif
}

static void
first_block_primesieve (mp_ptr bit_array, mp_limb_t n)
{
  mp_size_t bits, limbs;
  mp_limb_t i;

  ASSERT (n > 4);

  bits  = n_to_bit(n);
  limbs = bits / GMP_LIMB_BITS;

  if (limbs != 0)
    i = fill_bitpattern (bit_array + 1, limbs, 0);
  bit_array[0] = SIEVE_SEED;

  if ((bits + 1) % GMP_LIMB_BITS != 0)
    bit_array[limbs] |= MP_LIMB_T_MAX << ((bits + 1) % GMP_LIMB_BITS);

  if (n > SEED_LIMIT) {
    mp_limb_t mask, index;

    ASSERT (i < GMP_LIMB_BITS);

    if (n_to_bit (SEED_LIMIT + 1) < GMP_LIMB_BITS)
      i = 0;
    mask = CNST_LIMB(1) << i;
    index = 0;
    do {
      ++i;
      if ((bit_array[index] & mask) == 0)
	{
	  mp_size_t step, lindex;
	  mp_limb_t lmask;
	  unsigned  maskrot;

	  step = id_to_n(i);
/*	  lindex = n_to_bit(id_to_n(i)*id_to_n(i)); */
	  lindex = i*(step+1)-1+(-(i&1)&(i+1));
/*	  lindex = i*(step+1+(i&1))-1+(i&1); */
	  if (lindex > bits)
	    break;

	  step <<= 1;
	  maskrot = step % GMP_LIMB_BITS;

	  lmask = CNST_LIMB(1) << (lindex % GMP_LIMB_BITS);
	  do {
	    bit_array[lindex / GMP_LIMB_BITS] |= lmask;
	    lmask = lmask << maskrot | lmask >> (GMP_LIMB_BITS - maskrot);
	    lindex += step;
	  } while (lindex <= bits);

/*	  lindex = n_to_bit(id_to_n(i)*bit_to_n(i)); */
	  lindex = i*(i*3+6)+(i&1);

	  lmask = CNST_LIMB(1) << (lindex % GMP_LIMB_BITS);
	  for ( ; lindex <= bits; lindex += step) {
	    bit_array[lindex / GMP_LIMB_BITS] |= lmask;
	    lmask = lmask << maskrot | lmask >> (GMP_LIMB_BITS - maskrot);
	  };
	}
      mask = mask << 1 | mask >> (GMP_LIMB_BITS-1);
      index += mask & 1;
    } while (1);
  }
}

static void
block_resieve (mp_ptr bit_array, mp_size_t limbs, mp_limb_t offset,
	       mp_srcptr sieve)
{
  mp_size_t bits, off = offset;
  mp_limb_t mask, index, i;

  ASSERT (limbs > 0);
  ASSERT (offset >= GMP_LIMB_BITS);

  bits = limbs * GMP_LIMB_BITS - 1;

  i = fill_bitpattern (bit_array, limbs, offset - GMP_LIMB_BITS);

  ASSERT (i < GMP_LIMB_BITS);

  mask = CNST_LIMB(1) << i;
  index = 0;
  do {
    ++i;
    if ((sieve[index] & mask) == 0)
      {
	mp_size_t step, lindex;
	mp_limb_t lmask;
	unsigned  maskrot;

	step = id_to_n(i);

/*	lindex = n_to_bit(id_to_n(i)*id_to_n(i)); */
	lindex = i*(step+1)-1+(-(i&1)&(i+1));
/*	lindex = i*(step+1+(i&1))-1+(i&1); */
	if (lindex > bits + off)
	  break;

	step <<= 1;
	maskrot = step % GMP_LIMB_BITS;

	if (lindex < off)
	  lindex += step * ((off - lindex - 1) / step + 1);

	lindex -= off;

	lmask = CNST_LIMB(1) << (lindex % GMP_LIMB_BITS);
	for ( ; lindex <= bits; lindex += step) {
	  bit_array[lindex / GMP_LIMB_BITS] |= lmask;
	  lmask = lmask << maskrot | lmask >> (GMP_LIMB_BITS - maskrot);
	};

/*	lindex = n_to_bit(id_to_n(i)*bit_to_n(i)); */
	lindex = i*(i*3+6)+(i&1);

	if (lindex < off)
	  lindex += step * ((off - lindex - 1) / step + 1);

	lindex -= off;

	lmask = CNST_LIMB(1) << (lindex % GMP_LIMB_BITS);
	for ( ; lindex <= bits; lindex += step) {
	  bit_array[lindex / GMP_LIMB_BITS] |= lmask;
	  lmask = lmask << maskrot | lmask >> (GMP_LIMB_BITS - maskrot);
	};
      }
      mask = mask << 1 | mask >> (GMP_LIMB_BITS-1);
      index += mask & 1;
  } while (1);
}

#define BLOCK_SIZE 2048

/* Fills bit_array with the characteristic function of composite
   numbers up to the parameter n. I.e. a bit set to "1" represent a
   composite, a "0" represent a prime.

   The primesieve_size(n) limbs pointed to by bit_array are
   overwritten. The returned value counts prime integers in the
   interval [4, n]. Note that n > 4.

   Even numbers and multiples of 3 are excluded "a priori", only
   numbers equivalent to +/- 1 mod 6 have their bit in the array.

   Once sieved, if the bit b is ZERO it represent a prime, the
   represented prime is bit_to_n(b), if the LSbit is bit 0, or
   id_to_n(b), if you call "1" the first bit.
 */

mp_limb_t
gmp_primesieve (mp_ptr bit_array, mp_limb_t n)
{
  mp_size_t size;
  mp_limb_t bits;

  ASSERT (n > 4);

  bits = n_to_bit(n);
  size = bits / GMP_LIMB_BITS + 1;

  if (size > BLOCK_SIZE * 2) {
    mp_size_t off;
    off = BLOCK_SIZE + (size % BLOCK_SIZE);
    first_block_primesieve (bit_array, id_to_n (off * GMP_LIMB_BITS));
    do {
      block_resieve (bit_array + off, BLOCK_SIZE, off * GMP_LIMB_BITS, bit_array);
    } while ((off += BLOCK_SIZE) < size);
  } else {
    first_block_primesieve (bit_array, n);
  }

  if ((bits + 1) % GMP_LIMB_BITS != 0)
    bit_array[size-1] |= MP_LIMB_T_MAX << ((bits + 1) % GMP_LIMB_BITS);

  return size * GMP_LIMB_BITS - mpn_popcount (bit_array, size);
}

#undef BLOCK_SIZE
#undef SEED_LIMIT
#undef SIEVE_SEED
#undef SIEVE_MASK1
#undef SIEVE_MASK2
#undef SIEVE_MASKT
#undef SIEVE_2MSK1
#undef SIEVE_2MSK2
#undef SIEVE_2MSKT
#undef SET_OFF1
#undef SET_OFF2
#undef ROTATE1
#undef ROTATE2
