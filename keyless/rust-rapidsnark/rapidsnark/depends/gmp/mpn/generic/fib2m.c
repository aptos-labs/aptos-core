/* mpn_fib2m -- calculate Fibonacci numbers, modulo m.

Contributed to the GNU project by Marco Bodrato, based on the previous
fib2_ui.c file.

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
#include "longlong.h"


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

/* Store F[n] at fp and F[n-1] at f1p.  Both are computed modulo m.
   fp and f1p should have room for mn*2+1 limbs.

   The sign of one or both the values may be flipped (n-F, instead of F),
   the return value is 0 (zero) if the signs are coherent (both positive
   or both negative) and 1 (one) otherwise.

   Notes:

   In F[2k+1] with k even, +2 is applied to 4*F[k]^2 just by ORing into the
   low limb.

   In F[2k+1] with k odd, -2 is applied to F[k-1]^2 just by ORing into the
   low limb.

   TODO: Should {tp, 2 * mn} be passed as a scratch pointer?
   Should the call to mpn_fib2_ui() obtain (up to) 2*mn limbs?
*/

int
mpn_fib2m (mp_ptr fp, mp_ptr f1p, mp_srcptr np, mp_size_t nn, mp_srcptr mp, mp_size_t mn)
{
  unsigned long	nfirst;
  mp_limb_t	nh;
  mp_bitcnt_t	nbi;
  mp_size_t	sn, fn;
  int		fcnt, ncnt;

  ASSERT (! MPN_OVERLAP_P (fp, MAX(2*mn+1,5), f1p, MAX(2*mn+1,5)));
  ASSERT (nn > 0 && np[nn - 1] != 0);

  /* Estimate the maximal n such that fibonacci(n) fits in mn limbs. */
#if GMP_NUMB_BITS % 16 == 0
  if (UNLIKELY (ULONG_MAX / (23 * (GMP_NUMB_BITS / 16)) <= mn))
    nfirst = ULONG_MAX;
  else
    nfirst = mn * (23 * (GMP_NUMB_BITS / 16));
#else
  {
    mp_bitcnt_t	mbi;
    mbi = (mp_bitcnt_t) mn * GMP_NUMB_BITS;

    if (UNLIKELY (ULONG_MAX / 23 < mbi))
      {
	if (UNLIKELY (ULONG_MAX / 23 * 16 <= mbi))
	  nfirst = ULONG_MAX;
	else
	  nfirst = mbi / 16 * 23;
      }
    else
      nfirst = mbi * 23 / 16;
  }
#endif

  sn = nn - 1;
  nh = np[sn];
  count_leading_zeros (ncnt, nh);
  count_leading_zeros (fcnt, nfirst);

  if (fcnt >= ncnt)
    {
      ncnt = fcnt - ncnt;
      nh >>= ncnt;
    }
  else if (sn > 0)
    {
      ncnt -= fcnt;
      nh <<= ncnt;
      ncnt = GMP_NUMB_BITS - ncnt;
      --sn;
      nh |= np[sn] >> ncnt;
    }
  else
    ncnt = 0;

  nbi = sn * GMP_NUMB_BITS + ncnt;
  if (nh > nfirst)
    {
      nh >>= 1;
      ++nbi;
    }

  ASSERT (nh <= nfirst);
  /* Take a starting pair from mpn_fib2_ui. */
  fn = mpn_fib2_ui (fp, f1p, nh);
  MPN_ZERO (fp + fn, mn - fn);
  MPN_ZERO (f1p + fn, mn - fn);

  if (nbi == 0)
    {
      if (fn == mn)
	{
	  mp_limb_t qp[2];
	  mpn_tdiv_qr (qp, fp, 0, fp, fn, mp, mn);
	  mpn_tdiv_qr (qp, f1p, 0, f1p, fn, mp, mn);
	}

      return 0;
    }
  else
    {
      mp_ptr	tp;
      unsigned	pb = nh & 1;
      int	neg;
      TMP_DECL;

      TMP_MARK;

      tp = TMP_ALLOC_LIMBS (2 * mn + (mn < 2));

      do
	{
	  mp_ptr	rp;
	  /* Here fp==F[k] and f1p==F[k-1], with k being the bits of n from
	     nbi upwards.

	     Based on the next bit of n, we'll double to the pair
	     fp==F[2k],f1p==F[2k-1] or fp==F[2k+1],f1p==F[2k], according as
	     that bit is 0 or 1 respectively.  */

	  mpn_sqr (tp, fp,  mn);
	  mpn_sqr (fp, f1p, mn);

	  /* Calculate F[2k-1] = F[k]^2 + F[k-1]^2. */
	  f1p[2 * mn] = mpn_add_n (f1p, tp, fp, 2 * mn);

	  /* Calculate F[2k+1] = 4*F[k]^2 - F[k-1]^2 + 2*(-1)^k.
	     pb is the low bit of our implied k.  */

	  /* fp is F[k-1]^2 == 0 or 1 mod 4, like all squares. */
	  ASSERT ((fp[0] & 2) == 0);
	  ASSERT (pb == (pb & 1));
	  ASSERT ((fp[0] + (pb ? 2 : 0)) == (fp[0] | (pb << 1)));
	  fp[0] |= pb << 1;		/* possible -2 */
#if HAVE_NATIVE_mpn_rsblsh2_n
	  fp[2 * mn] = 1 + mpn_rsblsh2_n (fp, fp, tp, 2 * mn);
	  MPN_INCR_U(fp, 2 * mn + 1, (1 ^ pb) << 1);	/* possible +2 */
	  fp[2 * mn] = (fp[2 * mn] - 1) & GMP_NUMB_MAX;
#else
	  {
	    mp_limb_t  c;

	    c = mpn_lshift (tp, tp, 2 * mn, 2);
	    tp[0] |= (1 ^ pb) << 1;	/* possible +2 */
	    c -= mpn_sub_n (fp, tp, fp, 2 * mn);
	    fp[2 * mn] = c & GMP_NUMB_MAX;
	  }
#endif
	  neg = fp[2 * mn] == GMP_NUMB_MAX;

	  /* Calculate F[2k-1] = F[k]^2 + F[k-1]^2 */
	  /* Calculate F[2k+1] = 4*F[k]^2 - F[k-1]^2 + 2*(-1)^k */

	  /* Calculate F[2k] = F[2k+1] - F[2k-1], replacing the unwanted one of
	     F[2k+1] and F[2k-1].  */
	  --nbi;
	  pb = (np [nbi / GMP_NUMB_BITS] >> (nbi % GMP_NUMB_BITS)) & 1;
	  rp = pb ? f1p : fp;
	  if (neg)
	    {
	      /* Calculate -(F[2k+1] - F[2k-1]) */
	      rp[2 * mn] = f1p[2 * mn] + 1 - mpn_sub_n (rp, f1p, fp, 2 * mn);
	      neg = ! pb;
	      if (pb) /* fp not overwritten, negate it. */
		fp [2 * mn] = 1 ^ mpn_neg (fp, fp, 2 * mn);
	    }
	  else
	    {
	      neg = abs_sub_n (rp, fp, f1p, 2 * mn + 1) < 0;
	    }

	  mpn_tdiv_qr (tp, fp, 0, fp, 2 * mn + 1, mp, mn);
	  mpn_tdiv_qr (tp, f1p, 0, f1p, 2 * mn + 1, mp, mn);
	}
      while (nbi != 0);

      TMP_FREE;

      return neg;
    }
}
