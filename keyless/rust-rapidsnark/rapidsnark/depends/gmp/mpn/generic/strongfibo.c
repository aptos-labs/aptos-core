/* mpn_fib2m -- calculate Fibonacci numbers, modulo m.

Contributed to the GNU project by Marco Bodrato.

   THE FUNCTIONS IN THIS FILE ARE FOR INTERNAL USE ONLY.  THEY'RE ALMOST
   CERTAIN TO BE SUBJECT TO INCOMPATIBLE CHANGES OR DISAPPEAR COMPLETELY IN
   FUTURE GNU MP RELEASES.

Copyright 2001, 2002, 2005, 2009, 2018 Free Software Foundation, Inc.

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

#include <stdio.h>
#include "gmp-impl.h"


#if HAVE_NATIVE_mpn_rsblsh1_n || HAVE_NATIVE_mpn_sublsh1_n
#else
/* Stores |{ap,n}-{bp,n}| in {rp,n},
   returns the sign of {ap,n}-{bp,n}. */
static int
abs_sub_n (mp_ptr rp, mp_srcptr ap, mp_srcptr bp, mp_size_t n)
{
  mp_limb_t  x, y;
  while (--n >= 0)
    {
      x = ap[n];
      y = bp[n];
      if (x != y)
        {
          ++n;
          if (x > y)
            {
              ASSERT_NOCARRY (mpn_sub_n (rp, ap, bp, n));
              return 1;
            }
          else
            {
              ASSERT_NOCARRY (mpn_sub_n (rp, bp, ap, n));
              return -1;
            }
        }
      rp[n] = 0;
    }
  return 0;
}
#endif

/* Computes at most count terms of the sequence needed by the
   Lucas-Lehmer-Riesel test, indexing backward:
   L_i = L_{i+1}^2 - 2

   The sequence is computed modulo M = {mp, mn}.
   The starting point is given in L_{count+1} = {lp, mn}.
   The scratch pointed by sp, needs a space of at least 3 * mn + 1 limbs.

   Returns the index i>0 if L_i = 0 (mod M) is found within the
   computed count terms of the sequence.  Otherwise it returns zero.

   Note: (+/-2)^2-2=2, (+/-1)^2-2=-1, 0^2-2=-2
 */

static mp_bitcnt_t
mpn_llriter (mp_ptr lp, mp_srcptr mp, mp_size_t mn, mp_bitcnt_t count, mp_ptr sp)
{
  do
    {
      mpn_sqr (sp, lp, mn);
      mpn_tdiv_qr (sp + 2 * mn, lp, 0, sp, 2 * mn, mp, mn);
      if (lp[0] < 5)
	{
	  /* If L^2 % M < 5, |L^2 % M - 2| <= 2 */
	  if (mn == 1 || mpn_zero_p (lp + 1, mn - 1))
	    return (lp[0] == 2) ? count : 0;
	  else
	    MPN_DECR_U (lp, mn, 2);
	}
      else
	lp[0] -= 2;
    } while (--count != 0);
  return 0;
}

/* Store the Lucas' number L[n] at lp (maybe), computed modulo m.  lp
   and scratch should have room for mn*2+1 limbs.

   Returns the size of L[n] normally.

   If F[n] is zero modulo m, or L[n] is, returns 0 and lp is
   undefined.
*/

static mp_size_t
mpn_lucm (mp_ptr lp, mp_srcptr np, mp_size_t nn, mp_srcptr mp, mp_size_t mn, mp_ptr scratch)
{
  int		neg;
  mp_limb_t	cy;

  ASSERT (! MPN_OVERLAP_P (lp, MAX(2*mn+1,5), scratch, MAX(2*mn+1,5)));
  ASSERT (nn > 0);

  neg = mpn_fib2m (lp, scratch, np, nn, mp, mn);

  /* F[n] = +/-{lp, mn}, F[n-1] = +/-{scratch, mn} */
  if (mpn_zero_p (lp, mn))
    return 0;

  if (neg) /* One sign is opposite, use sub instead of add. */
    {
#if HAVE_NATIVE_mpn_rsblsh1_n || HAVE_NATIVE_mpn_sublsh1_n
#if HAVE_NATIVE_mpn_rsblsh1_n
      cy = mpn_rsblsh1_n (lp, lp, scratch, mn); /* L[n] = +/-(2F[n-1]-(-F[n])) */
#else
      cy = mpn_sublsh1_n (lp, lp, scratch, mn); /* L[n] = -/+(F[n]-(-2F[n-1])) */
      if (cy != 0)
	cy = mpn_add_n (lp, lp, mp, mn) - cy;
#endif
      if (cy > 1)
	cy += mpn_add_n (lp, lp, mp, mn);
#else
      cy = mpn_lshift (scratch, scratch, mn, 1); /* 2F[n-1] */
      if (UNLIKELY (cy))
	cy -= mpn_sub_n (lp, scratch, lp, mn); /* L[n] = +/-(2F[n-1]-(-F[n])) */
      else
	abs_sub_n (lp, lp, scratch, mn);
#endif
      ASSERT (cy <= 1);
    }
  else
    {
#if HAVE_NATIVE_mpn_addlsh1_n
      cy = mpn_addlsh1_n (lp, lp, scratch, mn); /* L[n] = +/-(2F[n-1]+F[n])) */
#else
      cy = mpn_lshift (scratch, scratch, mn, 1);
      cy+= mpn_add_n (lp, lp, scratch, mn);
#endif
      ASSERT (cy <= 2);
    }
  while (cy || mpn_cmp (lp, mp, mn) >= 0)
    cy -= mpn_sub_n (lp, lp, mp, mn);
  MPN_NORMALIZE (lp, mn);
  return mn;
}

int
mpn_strongfibo (mp_srcptr mp, mp_size_t mn, mp_ptr scratch)
{
  mp_ptr	lp, sp;
  mp_size_t	en;
  mp_bitcnt_t	b0;
  TMP_DECL;

#if GMP_NUMB_BITS % 4 == 0
  b0 = mpn_scan0 (mp, 0);
#else
  {
    mpz_t m = MPZ_ROINIT_N(mp, mn);
    b0 = mpz_scan0 (m, 0);
  }
  if (UNLIKELY (b0 == mn * GMP_NUMB_BITS))
    {
      en = 1;
      scratch [0] = 1;
    }
  else
#endif
    {
      int cnt = b0 % GMP_NUMB_BITS;
      en = b0 / GMP_NUMB_BITS;
      if (LIKELY (cnt != 0))
	mpn_rshift (scratch, mp + en, mn - en, cnt);
      else
	MPN_COPY (scratch, mp + en, mn - en);
      en = mn - en;
      scratch [0] |= 1;
      en -= scratch [en - 1] == 0;
    }
  TMP_MARK;

  lp = TMP_ALLOC_LIMBS (4 * mn + 6);
  sp = lp + 2 * mn + 3;
  en = mpn_lucm (sp, scratch, en, mp, mn, lp);
  if (en != 0 && LIKELY (--b0 != 0))
    {
      mpn_sqr (lp, sp, en);
      lp [0] |= 2; /* V^2 + 2 */
      if (LIKELY (2 * en >= mn))
	mpn_tdiv_qr (sp, lp, 0, lp, 2 * en, mp, mn);
      else
	MPN_ZERO (lp + 2 * en, mn - 2 * en);
      if (! mpn_zero_p (lp, mn) && LIKELY (--b0 != 0))
	b0 = mpn_llriter (lp, mp, mn, b0, lp + mn + 1);
    }
  TMP_FREE;
  return (b0 != 0);
}
