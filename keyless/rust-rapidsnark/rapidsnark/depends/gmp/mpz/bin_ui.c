/* mpz_bin_ui(RESULT, N, K) -- Set RESULT to N over K.

Copyright 1998-2002, 2012, 2013, 2015, 2017-2018 Free Software Foundation, Inc.

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

/* How many special cases? Minimum is 2: 0 and 1;
 * also 3 {0,1,2} and 5 {0,1,2,3,4} are implemented.
 */
#define APARTAJ_KALKULOJ 2

/* Whether to use (1) or not (0) the function mpz_bin_uiui whenever
 * the operands fit.
 */
#define UZU_BIN_UIUI 0

/* Whether to use a shortcut to precompute the product of four
 * elements (1), or precompute only the product of a couple (0).
 *
 * In both cases the precomputed product is then updated with some
 * linear operations to obtain the product of the next four (1)
 * [or two (0)] operands.
 */
#define KVAROPE 1

static void
posmpz_init (mpz_ptr r)
{
  mp_ptr rp;
  ASSERT (SIZ (r) > 0);
  rp = SIZ (r) + MPZ_REALLOC (r, SIZ (r) + 2);
  *rp = 0;
  *++rp = 0;
}

/* Equivalent to mpz_add_ui (r, r, in), but faster when
   0 < SIZ (r) < ALLOC (r) and limbs above SIZ (r) contain 0. */
static void
posmpz_inc_ui (mpz_ptr r, unsigned long in)
{
#if BITS_PER_ULONG > GMP_NUMB_BITS
  mpz_add_ui (r, r, in);
#else
  ASSERT (SIZ (r) > 0);
  MPN_INCR_U (PTR (r), SIZ (r) + 1, in);
  SIZ (r) += (PTR (r)[SIZ (r)] != 0);
#endif
}

/* Equivalent to mpz_sub_ui (r, r, in), but faster when
   0 < SIZ (r) and we know in advance that the result is positive. */
static void
posmpz_dec_ui (mpz_ptr r, unsigned long in)
{
#if BITS_PER_ULONG > GMP_NUMB_BITS
  mpz_sub_ui (r, r, in);
#else
  ASSERT (mpz_cmp_ui (r, in) >= 0);
  MPN_DECR_U (PTR (r), SIZ (r), in);
  SIZ (r) -= (PTR (r)[SIZ (r)-1] == 0);
#endif
}

/* Equivalent to mpz_tdiv_q_2exp (r, r, 1), but faster when
   0 < SIZ (r) and we know in advance that the result is positive. */
static void
posmpz_rsh1 (mpz_ptr r)
{
  mp_ptr rp;
  mp_size_t rn;

  rn = SIZ (r);
  rp = PTR (r);
  ASSERT (rn > 0);
  mpn_rshift (rp, rp, rn, 1);
  SIZ (r) -= rp[rn - 1] == 0;
}

/* Computes r = n(n+(2*k-1))/2
   It uses a sqare instead of a product, computing
   r = ((n+k-1)^2 + n - (k-1)^2)/2
   As a side effect, sets t = n+k-1
 */
static void
mpz_hmul_nbnpk (mpz_ptr r, mpz_srcptr n, unsigned long int k, mpz_ptr t)
{
  ASSERT (k > 0 && SIZ(n) > 0);
  --k;
  mpz_add_ui (t, n, k);
  mpz_mul (r, t, t);
  mpz_add (r, r, n);
  posmpz_rsh1 (r);
  if (LIKELY (k <= (1UL << (BITS_PER_ULONG / 2))))
    posmpz_dec_ui (r, (k + (k & 1))*(k >> 1));
  else
    {
      mpz_t tmp;
      mpz_init_set_ui (tmp, (k + (k & 1)));
      mpz_mul_ui (tmp, tmp, k >> 1);
      mpz_sub (r, r, tmp);
      mpz_clear (tmp);
    }
}

#if KVAROPE
static void
rek_raising_fac4 (mpz_ptr r, mpz_ptr p, mpz_ptr P, unsigned long int k, unsigned long int lk, mpz_ptr t)
{
  if (k - lk < 5)
    {
      do {
	posmpz_inc_ui (p, 4*k+2);
	mpz_addmul_ui (P, p, 4*k);
	posmpz_dec_ui (P, k);
	mpz_mul (r, r, P);
      } while (--k > lk);
    }
  else
    {
      mpz_t lt;
      unsigned long int m;

      m = ((k + lk) >> 1) + 1;
      rek_raising_fac4 (r, p, P, k, m, t);

      posmpz_inc_ui (p, 4*m+2);
      mpz_addmul_ui (P, p, 4*m);
      posmpz_dec_ui (P, m);
      if (t == NULL)
	{
	  mpz_init_set (lt, P);
	  t = lt;
	}
      else
	{
	  ALLOC (lt) = 0;
	  mpz_set (t, P);
	}
      rek_raising_fac4 (t, p, P, m - 1, lk, NULL);

      mpz_mul (r, r, t);
      mpz_clear (lt);
    }
}

/* Computes (n+1)(n+2)...(n+k)/2^(k/2 +k/4) using the helper function
   rek_raising_fac4, and exploiting an idea inspired by a piece of
   code that Fredrik Johansson wrote and by a comment by Niels Möller.

   Assume k = 4i then compute:
     p  = (n+1)(n+4i)/2 - i
	  (n+1+1)(n+4i)/2 = p + i + (n+4i)/2
	  (n+1+1)(n+4i-1)/2 = p + i + ((n+4i)-(n+1+1))/2 = p + i + (n-n+4i-2)/2 = p + 3i-1
     P  = (p + i)*(p+3i-1)/2 = (n+1)(n+2)(n+4i-1)(n+4i)/8
     n' = n + 2
     i' = i - 1
	  (n'-1)(n')(n'+4i'+1)(n'+4i'+2)/8 = P
	  (n'-1)(n'+4i'+2)/2 - i' - 1 = p
	  (n'-1+2)(n'+4i'+2)/2 - i' - 1 = p + (n'+4i'+2)
	  (n'-1+2)(n'+4i'+2-2)/2 - i' - 1 = p + (n'+4i'+2) - (n'-1+2) =  p + 4i' + 1
	  (n'-1+2)(n'+4i'+2-2)/2 - i' = p + 4i' + 2
     p' = p + 4i' + 2 = (n'+1)(n'+4i')/2 - i'
	  p' - 4i' - 2 = p
	  (p' - 4i' - 2 + i)*(p' - 4i' - 2+3i-1)/2 = P
	  (p' - 4i' - 2 + i' + 1)*(p' - 4i' - 2 + 3i' + 3 - 1)/2 = P
	  (p' - 3i' - 1)*(p' - i')/2 = P
	  (p' - 3i' - 1 + 4i' + 1)*(p' - i' + 4i' - 1)/2 = P + (4i' + 1)*(p' - i')/2 + (p' - 3i' - 1 + 4i' + 1)*(4i' - 1)/2
	  (p' + i')*(p' + 3i' - 1)/2 = P + (4i')*(p' + p')/2 + (p' - i' - (p' + i'))/2
	  (p' + i')*(p' + 3i' - 1)/2 = P + 4i'p' + (p' - i' - p' - i')/2
	  (p' + i')*(p' + 3i' - 1)/2 = P + 4i'p' - i'
     P' = P + 4i'p' - i'

   And compute the product P * P' * P" ...
 */

static void
mpz_raising_fac4 (mpz_ptr r, mpz_ptr n, unsigned long int k, mpz_ptr t, mpz_ptr p)
{
  ASSERT ((k >= APARTAJ_KALKULOJ) && (APARTAJ_KALKULOJ > 0));
  posmpz_init (n);
  posmpz_inc_ui (n, 1);
  SIZ (r) = 0;
  if (k & 1)
    {
      mpz_set (r, n);
      posmpz_inc_ui (n, 1);
    }
  k >>= 1;
  if (APARTAJ_KALKULOJ < 2 && k == 0)
    return;

  mpz_hmul_nbnpk (p, n, k, t);
  posmpz_init (p);

  if (k & 1)
    {
      if (SIZ (r))
	mpz_mul (r, r, p);
      else
	mpz_set (r, p);
      posmpz_inc_ui (p, k - 1);
    }
  k >>= 1;
  if (APARTAJ_KALKULOJ < 4 && k == 0)
    return;

  mpz_hmul_nbnpk (t, p, k, n);
  if (SIZ (r))
    mpz_mul (r, r, t);
  else
    mpz_set (r, t);

  if (APARTAJ_KALKULOJ > 8 || k > 1)
    {
      posmpz_dec_ui (p, k);
      rek_raising_fac4 (r, p, t, k - 1, 0, n);
    }
}

#else /* KVAROPE */

static void
rek_raising_fac (mpz_ptr r, mpz_ptr n, unsigned long int k, unsigned long int lk, mpz_ptr t1, mpz_ptr t2)
{
  /* Should the threshold depend on SIZ (n) ? */
  if (k - lk < 10)
    {
      do {
	posmpz_inc_ui (n, k);
	mpz_mul (r, r, n);
	--k;
      } while (k > lk);
    }
  else
    {
      mpz_t t3;
      unsigned long int m;

      m = ((k + lk) >> 1) + 1;
      rek_raising_fac (r, n, k, m, t1, t2);

      posmpz_inc_ui (n, m);
      if (t1 == NULL)
	{
	  mpz_init_set (t3, n);
	  t1 = t3;
	}
      else
	{
	  ALLOC (t3) = 0;
	  mpz_set (t1, n);
	}
      rek_raising_fac (t1, n, m - 1, lk, t2, NULL);

      mpz_mul (r, r, t1);
      mpz_clear (t3);
    }
}

/* Computes (n+1)(n+2)...(n+k)/2^(k/2) using the helper function
   rek_raising_fac, and exploiting an idea inspired by a piece of
   code that Fredrik Johansson wrote.

   Force an even k = 2i then compute:
     p  = (n+1)(n+2i)/2
     i' = i - 1
     p == (n+1)(n+2i'+2)/2
     p' = p + i' == (n+2)(n+2i'+1)/2
     n' = n + 1
     p'== (n'+1)(n'+2i')/2 == (n+1 +1)(n+2i -1)/2

   And compute the product p * p' * p" ...
*/

static void
mpz_raising_fac (mpz_ptr r, mpz_ptr n, unsigned long int k, mpz_ptr t, mpz_ptr p)
{
  unsigned long int hk;
  ASSERT ((k >= APARTAJ_KALKULOJ) && (APARTAJ_KALKULOJ > 1));
  mpz_add_ui (n, n, 1);
  hk = k >> 1;
  mpz_hmul_nbnpk (p, n, hk, t);

  if ((k & 1) != 0)
    {
      mpz_add_ui (t, t, hk + 1);
      mpz_mul (r, t, p);
    }
  else
    {
      mpz_set (r, p);
    }

  if ((APARTAJ_KALKULOJ > 3) || (hk > 1))
    {
      posmpz_init (p);
      rek_raising_fac (r, p, hk - 1, 0, t, n);
    }
}
#endif /* KVAROPE */

/* This is a poor implementation.  Look at bin_uiui.c for improvement ideas.
   In fact consider calling mpz_bin_uiui() when the arguments fit, leaving
   the code here only for big n.

   The identity bin(n,k) = (-1)^k * bin(-n+k-1,k) can be found in Knuth vol
   1 section 1.2.6 part G. */

void
mpz_bin_ui (mpz_ptr r, mpz_srcptr n, unsigned long int k)
{
  mpz_t      ni;
  mp_size_t  negate;

  if (SIZ (n) < 0)
    {
      /* bin(n,k) = (-1)^k * bin(-n+k-1,k), and set ni = -n+k-1 - k = -n-1 */
      mpz_init (ni);
      mpz_add_ui (ni, n, 1L);
      mpz_neg (ni, ni);
      negate = (k & 1);   /* (-1)^k */
    }
  else
    {
      /* bin(n,k) == 0 if k>n
	 (no test for this under the n<0 case, since -n+k-1 >= k there) */
      if (mpz_cmp_ui (n, k) < 0)
	{
	  SIZ (r) = 0;
	  return;
	}

      /* set ni = n-k */
      mpz_init (ni);
      mpz_sub_ui (ni, n, k);
      negate = 0;
    }

  /* Now wanting bin(ni+k,k), with ni positive, and "negate" is the sign (0
     for positive, 1 for negative). */

  /* Rewrite bin(n,k) as bin(n,n-k) if that is smaller.  In this case it's
     whether ni+k-k < k meaning ni<k, and if so change to denominator ni+k-k
     = ni, and new ni of ni+k-ni = k.  */
  if (mpz_cmp_ui (ni, k) < 0)
    {
      unsigned long  tmp;
      tmp = k;
      k = mpz_get_ui (ni);
      mpz_set_ui (ni, tmp);
    }

  if (k < APARTAJ_KALKULOJ)
    {
      if (k == 0)
	{
	  SIZ (r) = 1;
	  MPZ_NEWALLOC (r, 1)[0] = 1;
	}
#if APARTAJ_KALKULOJ > 2
      else if (k == 2)
	{
	  mpz_add_ui (ni, ni, 1);
	  mpz_mul (r, ni, ni);
	  mpz_add (r, r, ni);
	  posmpz_rsh1 (r);
	}
#endif
#if APARTAJ_KALKULOJ > 3
      else if (k > 2)
	{ /* k = 3, 4 */
	  mpz_add_ui (ni, ni, 2); /* n+1 */
	  mpz_mul (r, ni, ni); /* (n+1)^2 */
	  mpz_sub_ui (r, r, 1); /* (n+1)^2-1 */
	  if (k == 3)
	    {
	      mpz_mul (r, r, ni); /* ((n+1)^2-1)(n+1) = n(n+1)(n+2) */
	      /* mpz_divexact_ui (r, r, 6); /\* 6=3<<1; div_by3 ? *\/ */
	      mpn_pi1_bdiv_q_1 (PTR(r), PTR(r), SIZ(r), 3, GMP_NUMB_MASK/3*2+1, 1);
	      MPN_NORMALIZE_NOT_ZERO (PTR(r), SIZ(r));
	    }
	  else /* k = 4 */
	    {
	      mpz_add (ni, ni, r); /* (n+1)^2+n */
	      mpz_mul (r, ni, ni); /* ((n+1)^2+n)^2 */
	      mpz_sub_ui (r, r, 1); /* ((n+1)^2+n)^2-1 = n(n+1)(n+2)(n+3) */
	      /* mpz_divexact_ui (r, r, 24); /\* 24=3<<3; div_by3 ? *\/ */
	      mpn_pi1_bdiv_q_1 (PTR(r), PTR(r), SIZ(r), 3, GMP_NUMB_MASK/3*2+1, 3);
	      MPN_NORMALIZE_NOT_ZERO (PTR(r), SIZ(r));
	    }
	}
#endif
      else
	{ /* k = 1 */
	  mpz_add_ui (r, ni, 1);
	}
    }
#if UZU_BIN_UIUI
  else if (mpz_cmp_ui (ni, ULONG_MAX - k) <= 0)
    {
      mpz_bin_uiui (r, mpz_get_ui (ni) + k, k);
    }
#endif
  else
    {
      mp_limb_t count;
      mpz_t num, den;

      mpz_init (num);
      mpz_init (den);

#if KVAROPE
      mpz_raising_fac4 (num, ni, k, den, r);
      popc_limb (count, k);
      ASSERT (k - (k >> 1) - (k >> 2) - count >= 0);
      mpz_tdiv_q_2exp (num, num, k - (k >> 1) - (k >> 2) - count);
#else
      mpz_raising_fac (num, ni, k, den, r);
      popc_limb (count, k);
      ASSERT (k - (k >> 1) - count >= 0);
      mpz_tdiv_q_2exp (num, num, k - (k >> 1) - count);
#endif

      mpz_oddfac_1(den, k, 0);

      mpz_divexact(r, num, den);
      mpz_clear (num);
      mpz_clear (den);
    }
  mpz_clear (ni);

  SIZ(r) = (SIZ(r) ^ -negate) + negate;
}
