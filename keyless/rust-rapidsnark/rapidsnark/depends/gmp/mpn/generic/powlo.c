/* mpn_powlo -- Compute R = U^E mod B^n, where B is the limb base.

Copyright 2007-2009, 2012, 2015, 2016, 2018 Free Software Foundation, Inc.

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
#include "longlong.h"


#define getbit(p,bi) \
  ((p[(bi - 1) / GMP_LIMB_BITS] >> (bi - 1) % GMP_LIMB_BITS) & 1)

static inline mp_limb_t
getbits (const mp_limb_t *p, mp_bitcnt_t bi, unsigned nbits)
{
  unsigned nbits_in_r;
  mp_limb_t r;
  mp_size_t i;

  if (bi < nbits)
    {
      return p[0] & (((mp_limb_t) 1 << bi) - 1);
    }
  else
    {
      bi -= nbits;			/* bit index of low bit to extract */
      i = bi / GMP_NUMB_BITS;		/* word index of low bit to extract */
      bi %= GMP_NUMB_BITS;		/* bit index in low word */
      r = p[i] >> bi;			/* extract (low) bits */
      nbits_in_r = GMP_NUMB_BITS - bi;	/* number of bits now in r */
      if (nbits_in_r < nbits)		/* did we get enough bits? */
	r += p[i + 1] << nbits_in_r;	/* prepend bits from higher word */
      return r & (((mp_limb_t ) 1 << nbits) - 1);
    }
}

static inline unsigned
win_size (mp_bitcnt_t eb)
{
  unsigned k;
  static mp_bitcnt_t x[] = {7,25,81,241,673,1793,4609,11521,28161,~(mp_bitcnt_t)0};
  ASSERT (eb > 1);
  for (k = 0; eb > x[k++];)
    ;
  return k;
}

/* rp[n-1..0] = bp[n-1..0] ^ ep[en-1..0] mod B^n, B is the limb base.
   Requires that ep[en-1] is non-zero.
   Uses scratch space tp[3n-1..0], i.e., 3n words.  */
/* We only use n words in the scratch space, we should pass tp + n to
   mullo/sqrlo as a temporary area, it is needed. */
void
mpn_powlo (mp_ptr rp, mp_srcptr bp,
	   mp_srcptr ep, mp_size_t en,
	   mp_size_t n, mp_ptr tp)
{
  unsigned cnt;
  mp_bitcnt_t ebi;
  unsigned windowsize, this_windowsize;
  mp_limb_t expbits;
  mp_limb_t *pp;
  long i;
  int flipflop;
  TMP_DECL;

  ASSERT (en > 1 || (en == 1 && ep[0] > 1));

  TMP_MARK;

  MPN_SIZEINBASE_2EXP(ebi, ep, en, 1);

  windowsize = win_size (ebi);
  if (windowsize > 1)
    {
      mp_limb_t *this_pp, *last_pp;
      ASSERT (windowsize < ebi);

      pp = TMP_ALLOC_LIMBS ((n << (windowsize - 1)));

      this_pp = pp;

      MPN_COPY (this_pp, bp, n);

      /* Store b^2 in tp.  */
      mpn_sqrlo (tp, bp, n);

      /* Precompute odd powers of b and put them in the temporary area at pp.  */
      i = (1 << (windowsize - 1)) - 1;
      do
	{
	  last_pp = this_pp;
	  this_pp += n;
	  mpn_mullo_n (this_pp, last_pp, tp, n);
	} while (--i != 0);

      expbits = getbits (ep, ebi, windowsize);

      /* THINK: Should we initialise the case expbits % 4 == 0 with a mullo? */
      count_trailing_zeros (cnt, expbits);
      ebi -= windowsize;
      ebi += cnt;
      expbits >>= cnt;

      MPN_COPY (rp, pp + n * (expbits >> 1), n);
    }
  else
    {
      pp = tp + n;
      MPN_COPY (pp, bp, n);
      MPN_COPY (rp, bp, n);
      --ebi;
    }

  flipflop = 0;

  do
    {
      while (getbit (ep, ebi) == 0)
	{
	  mpn_sqrlo (tp, rp, n);
	  MP_PTR_SWAP (rp, tp);
	  flipflop = ! flipflop;
	  if (--ebi == 0)
	    goto done;
	}

      /* The next bit of the exponent is 1.  Now extract the largest block of
	 bits <= windowsize, and such that the least significant bit is 1.  */

      expbits = getbits (ep, ebi, windowsize);
      this_windowsize = MIN (windowsize, ebi);
      ebi -= this_windowsize;

      count_trailing_zeros (cnt, expbits);
      this_windowsize -= cnt;
      ebi += cnt;
      expbits >>= cnt;

      while (this_windowsize > 1)
	{
	  mpn_sqrlo (tp, rp, n);
	  mpn_sqrlo (rp, tp, n);
	  this_windowsize -= 2;
	}

      if (this_windowsize != 0)
	mpn_sqrlo (tp, rp, n);
      else
	{
	  MP_PTR_SWAP (rp, tp);
	  flipflop = ! flipflop;
	}

      mpn_mullo_n (rp, tp, pp + n * (expbits >> 1), n);
    } while (ebi != 0);

 done:
  if (flipflop)
    MPN_COPY (tp, rp, n);
  TMP_FREE;
}
