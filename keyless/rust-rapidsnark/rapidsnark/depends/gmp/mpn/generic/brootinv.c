/* mpn_brootinv, compute r such that r^k * y = 1 (mod 2^b).

   Contributed to the GNU project by Martin Boij (as part of perfpow.c).

Copyright 2009, 2010, 2012, 2013, 2018 Free Software Foundation, Inc.

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

/* Computes a^2e (mod B). Uses right-to-left binary algorithm, since
   typical use will have e small. */
static mp_limb_t
powsquaredlimb (mp_limb_t a, mp_limb_t e)
{
  mp_limb_t r;

  r = 1;
  /* if (LIKELY (e != 0)) */
  do {
    a *= a;
    if (e & 1)
      r *= a;
    e >>= 1;
  } while (e != 0);

  return r;
}

/* Compute r such that r^k * y = 1 (mod B^n).

   Iterates
     r' <-- k^{-1} ((k+1) r - r^{k+1} y) (mod 2^b)
   using Hensel lifting, each time doubling the number of known bits in r.

   Works just for odd k.  Else the Hensel lifting degenerates.

   FIXME:

     (1) Make it work for k == GMP_LIMB_MAX (k+1 below overflows).

     (2) Rewrite iteration as
	   r' <-- r - k^{-1} r (r^k y - 1)
	 and take advantage of the zero low part of r^k y - 1.

     (3) Use wrap-around trick.

     (4) Use a small table to get starting value.

   Scratch need: bn + (((bn + 1) >> 1) + 1) + scratch for mpn_powlo
   Currently mpn_powlo requires 3*bn
   so that 5*bn is surely enough, where bn = ceil (bnb / GMP_NUMB_BITS).
*/

void
mpn_brootinv (mp_ptr rp, mp_srcptr yp, mp_size_t bn, mp_limb_t k, mp_ptr tp)
{
  mp_ptr tp2, tp3;
  mp_limb_t kinv, k2, r0, y0;
  mp_size_t order[GMP_LIMB_BITS + 1];
  int d;

  ASSERT (bn > 0);
  ASSERT ((k & 1) != 0);

  tp2 = tp + bn;
  tp3 = tp + bn + ((bn + 3) >> 1);
  k2 = (k >> 1) + 1; /* (k + 1) / 2 , but avoid k+1 overflow */

  binvert_limb (kinv, k);

  /* 4-bit initial approximation:

   y%16 | 1  3  5  7  9 11 13 15,
    k%4 +-------------------------+k2%2
     1  | 1 11 13  7  9  3  5 15  |  1
     3  | 1  3  5  7  9 11 13 15  |  0

  */
  y0 = yp[0];

  r0 = y0 ^ (((y0 << 1) ^ (y0 << 2)) & (k2 << 3) & 8);			/* 4 bits */
  r0 = kinv * (k2 * r0 * 2 - y0 * powsquaredlimb(r0, k2 & 0x3f));	/* 8 bits */
  r0 = kinv * (k2 * r0 * 2 - y0 * powsquaredlimb(r0, k2 & 0x3fff));	/* 16 bits */
#if GMP_NUMB_BITS > 16
  {
    unsigned prec = 16;
    do
      {
	r0 = kinv * (k2 * r0 * 2 - y0 * powsquaredlimb(r0, k2));
	prec *= 2;
      }
    while (prec < GMP_NUMB_BITS);
  }
#endif

  rp[0] = r0;
  if (bn == 1)
    return;

  d = 0;
  for (; bn != 2; bn = (bn + 1) >> 1)
    order[d++] = bn;

  order[d] = 2;
  bn = 1;

  do
    {
      mpn_sqr (tp, rp, bn); /* Result may overlap tp2 */
      tp2[bn] = mpn_mul_1 (tp2, rp, bn, k2 << 1);

      bn = order[d];

      mpn_powlo (rp, tp, &k2, 1, bn, tp3);
      mpn_mullo_n (tp, yp, rp, bn);

      /* mpn_sub (tp, tp2, ((bn + 1) >> 1) + 1, tp, bn); */
      /* The function above is not handled, ((bn + 1) >> 1) + 1 <= bn*/
      {
	mp_size_t pbn = (bn + 3) >> 1; /* Size of tp2 */
	int borrow;
	borrow = mpn_sub_n (tp, tp2, tp, pbn) != 0;
	if (bn > pbn) /* 3 < bn */
	  {
	    if (borrow)
	      mpn_com (tp + pbn, tp + pbn, bn - pbn);
	    else
	      mpn_neg (tp + pbn, tp + pbn, bn - pbn);
	  }
      }
      mpn_pi1_bdiv_q_1 (rp, tp, bn, k, kinv, 0);
    }
  while (--d >= 0);
}
