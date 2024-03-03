/* mpn_compute_powtab.

   Contributed to the GNU project by Torbjorn Granlund.

   THE FUNCTIONS IN THIS FILE ARE INTERNAL WITH MUTABLE INTERFACES.  IT IS ONLY
   SAFE TO REACH THEM THROUGH DOCUMENTED INTERFACES.  IN FACT, IT IS ALMOST
   GUARANTEED THAT THEY WILL CHANGE OR DISAPPEAR IN A FUTURE GNU MP RELEASE.

Copyright 1991-2017 Free Software Foundation, Inc.

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

/*
  CAVEATS:
  * The exptab and powtab vectors are in opposite orders.  Probably OK.
  * Consider getting rid of exptab, doing bit ops on the un argument instead.
  * Consider rounding greatest power slightly upwards to save adjustments.
  * In powtab_decide, consider computing cost from just the 2-3 largest
    operands, since smaller operand contribute little.  This makes most sense
    if exptab is suppressed.
*/

#include "gmp-impl.h"

#ifndef DIV_1_VS_MUL_1_PERCENT
#define DIV_1_VS_MUL_1_PERCENT 150
#endif

#define SET_powers_t(dest, ptr, size, dib, b, sh)	\
  do {							\
    dest.p = ptr;					\
    dest.n = size;					\
    dest.digits_in_base = dib;				\
    dest.base = b;					\
    dest.shift = sh;					\
  } while (0)

#if DIV_1_VS_MUL_1_PERCENT > 120
#define HAVE_mpn_compute_powtab_mul 1
static void
mpn_compute_powtab_mul (powers_t *powtab, mp_ptr powtab_mem, mp_size_t un,
			int base, const size_t *exptab, size_t n_pows)
{
  mp_size_t n;
  mp_ptr p, t;
  mp_limb_t cy;
  long start_idx;
  int c;
  mp_size_t shift;
  long pi;

  mp_limb_t big_base = mp_bases[base].big_base;
  int chars_per_limb = mp_bases[base].chars_per_limb;

  mp_ptr powtab_mem_ptr = powtab_mem;

  size_t digits_in_base = chars_per_limb;

  powers_t *pt = powtab;

  p = powtab_mem_ptr;
  powtab_mem_ptr += 1;
  p[0] = big_base;

  SET_powers_t (pt[0], p, 1, digits_in_base, base, 0);
  pt++;

  t = powtab_mem_ptr;
  powtab_mem_ptr += 2;
  t[1] = mpn_mul_1 (t, p, 1, big_base);
  n = 2;

  digits_in_base *= 2;

  c = t[0] == 0;
  t += c;
  n -= c;
  shift = c;

  SET_powers_t (pt[0], t, n, digits_in_base, base, shift);
  p = t;
  pt++;

  if (exptab[0] == ((size_t) chars_per_limb << n_pows))
    {
      start_idx = n_pows - 2;
    }
  else
    {
      if (((digits_in_base + chars_per_limb) << (n_pows-2)) <= exptab[0])
	{
	  /* 3, sometimes adjusted to 4.  */
	  t = powtab_mem_ptr;
	  powtab_mem_ptr += 4;
	  t[n] = cy = mpn_mul_1 (t, p, n, big_base);
	  n += cy != 0;;

	  digits_in_base += chars_per_limb;

	  c  = t[0] == 0;
	  t += c;
	  n -= c;
	  shift += c;
	}
      else
	{
	  /* 2 copy, will always become 3 with back-multiplication.  */
	  t = powtab_mem_ptr;
	  powtab_mem_ptr += 3;
	  t[0] = p[0];
	  t[1] = p[1];
	}

      SET_powers_t (pt[0], t, n, digits_in_base, base, shift);
      p = t;
      pt++;
      start_idx = n_pows - 3;
    }

  for (pi = start_idx; pi >= 0; pi--)
    {
      t = powtab_mem_ptr;
      powtab_mem_ptr += 2 * n + 2;

      ASSERT (powtab_mem_ptr < powtab_mem + mpn_str_powtab_alloc (un));

      mpn_sqr (t, p, n);

      digits_in_base *= 2;
      n *= 2;
      n -= t[n - 1] == 0;
      shift *= 2;

      c = t[0] == 0;
      t += c;
      n -= c;
      shift += c;

      /* Adjust new value if it is too small as input to the next squaring.  */
      if (((digits_in_base + chars_per_limb) << pi) <= exptab[0])
	{
	  t[n] = cy = mpn_mul_1 (t, t, n, big_base);
	  n += cy != 0;

	  digits_in_base += chars_per_limb;

	  c  = t[0] == 0;
	  t += c;
	  n -= c;
	  shift += c;
	}

      SET_powers_t (pt[0], t, n, digits_in_base, base, shift);

      /* Adjust previous value if it is not at its target power.  */
      if (pt[-1].digits_in_base < exptab[pi + 1])
	{
	  mp_size_t n = pt[-1].n;
	  mp_ptr p = pt[-1].p;
	  p[n] = cy = mpn_mul_1 (p, p, n, big_base);
	  n += cy != 0;

	  ASSERT (pt[-1].digits_in_base + chars_per_limb == exptab[pi + 1]);
	  pt[-1].digits_in_base = exptab[pi + 1];

	  c = p[0] == 0;
	  pt[-1].p = p + c;
	  pt[-1].n = n - c;
	  pt[-1].shift += c;
	}

      p = t;
      pt++;
    }
}
#endif

#if DIV_1_VS_MUL_1_PERCENT < 275
#define HAVE_mpn_compute_powtab_div 1
static void
mpn_compute_powtab_div (powers_t *powtab, mp_ptr powtab_mem, mp_size_t un,
			int base, const size_t *exptab, size_t n_pows)
{
  mp_ptr p, t;

  mp_limb_t big_base = mp_bases[base].big_base;
  int chars_per_limb = mp_bases[base].chars_per_limb;

  mp_ptr powtab_mem_ptr = powtab_mem;

  size_t digits_in_base = chars_per_limb;

  powers_t *pt = powtab;

  mp_size_t n = 1;
  mp_size_t shift = 0;
  long pi;

  p = powtab_mem_ptr;
  powtab_mem_ptr += 1;
  p[0] = big_base;

  SET_powers_t (pt[0], p, 1, digits_in_base, base, 0);
  pt++;

  for (pi = n_pows - 1; pi >= 0; pi--)
    {
      t = powtab_mem_ptr;
      powtab_mem_ptr += 2 * n;

      ASSERT (powtab_mem_ptr < powtab_mem + mpn_str_powtab_alloc (un));

      mpn_sqr (t, p, n);
      n = 2 * n - 1; n += t[n] != 0;
      digits_in_base *= 2;

      if (digits_in_base != exptab[pi])	/* if ((((un - 1) >> pi) & 2) == 0) */
	{
#if HAVE_NATIVE_mpn_pi1_bdiv_q_1 || ! HAVE_NATIVE_mpn_divexact_1
	  if (__GMP_LIKELY (base == 10))
	    mpn_pi1_bdiv_q_1 (t, t, n, big_base >> MP_BASES_BIG_BASE_CTZ_10,
			      MP_BASES_BIG_BASE_BINVERTED_10,
			      MP_BASES_BIG_BASE_CTZ_10);
	  else
#endif
	    /* FIXME: We could use _pi1 here if we add big_base_binverted and
	       big_base_ctz fields to struct bases.  That would add about 2 KiB
	       to mp_bases.c.
	       FIXME: Use mpn_bdiv_q_1 here when mpn_divexact_1 is converted to
	       mpn_bdiv_q_1 for more machines. */
	    mpn_divexact_1 (t, t, n, big_base);

	  n -= t[n - 1] == 0;
	  digits_in_base -= chars_per_limb;
	}

      shift *= 2;
      /* Strip low zero limbs, but be careful to keep the result divisible by
	 big_base.  */
      while (t[0] == 0 && (t[1] & ((big_base & -big_base) - 1)) == 0)
	{
	  t++;
	  n--;
	  shift++;
	}
      p = t;

      SET_powers_t (pt[0], p, n, digits_in_base, base, shift);
      pt++;
    }

  /* Strip any remaining low zero limbs.  */
  pt -= n_pows + 1;
  for (pi = n_pows; pi >= 0; pi--)
    {
      mp_ptr t = pt[pi].p;
      mp_size_t shift = pt[pi].shift;
      mp_size_t n = pt[pi].n;
      int c;
      c = t[0] == 0;
      t += c;
      n -= c;
      shift += c;
      pt[pi].p = t;
      pt[pi].shift = shift;
      pt[pi].n = n;
    }
}
#endif

static long
powtab_decide (size_t *exptab, size_t un, int base)
{
  int chars_per_limb = mp_bases[base].chars_per_limb;
  long n_pows = 0;
  size_t pn;
  for (pn = (un + 1) >> 1; pn != 1; pn = (pn + 1) >> 1)
    {
      exptab[n_pows] = pn * chars_per_limb;
      n_pows++;
    }
  exptab[n_pows] = chars_per_limb;

#if HAVE_mpn_compute_powtab_mul && HAVE_mpn_compute_powtab_div
  {
  size_t pn = un - 1;
  size_t xn = (un + 1) >> 1;
  unsigned mcost = 1;
  unsigned dcost = 1;
  long i;
  for (i = n_pows - 2; i >= 0; i--)
    {
      size_t pow = (pn >> (i + 1)) + 1;

      if (pow & 1)
	dcost += pow;

      if (xn != (pow << i))
	{
	  if (pow > 2 && (pow & 1) == 0)
	    mcost += 2 * pow;
	  else
	    mcost += pow;
	}
      else
	{
	  if (pow & 1)
	    mcost += pow;
	}
    }

  dcost = dcost * DIV_1_VS_MUL_1_PERCENT / 100;

  if (mcost <= dcost)
    return n_pows;
  else
    return -n_pows;
  }
#elif HAVE_mpn_compute_powtab_mul
  return n_pows;
#elif HAVE_mpn_compute_powtab_div
  return -n_pows;
#else
#error "no powtab function available"
#endif
}

size_t
mpn_compute_powtab (powers_t *powtab, mp_ptr powtab_mem, mp_size_t un, int base)
{
  size_t exptab[GMP_LIMB_BITS];

  long n_pows = powtab_decide (exptab, un, base);

#if HAVE_mpn_compute_powtab_mul && HAVE_mpn_compute_powtab_div
  if (n_pows >= 0)
    {
      mpn_compute_powtab_mul (powtab, powtab_mem, un, base, exptab, n_pows);
      return n_pows;
    }
  else
    {
      mpn_compute_powtab_div (powtab, powtab_mem, un, base, exptab, -n_pows);
      return -n_pows;
    }
#elif HAVE_mpn_compute_powtab_mul
  ASSERT (n_pows > 0);
  mpn_compute_powtab_mul (powtab, powtab_mem, un, base, exptab, n_pows);
  return n_pows;
#elif HAVE_mpn_compute_powtab_div
  ASSERT (n_pows < 0);
  mpn_compute_powtab_div (powtab, powtab_mem, un, base, exptab, -n_pows);
  return -n_pows;
#else
#error "no powtab function available"
#endif
}
