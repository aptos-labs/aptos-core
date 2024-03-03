/* mpz_millerrabin(n,reps) -- An implementation of the probabilistic primality
   test found in Knuth's Seminumerical Algorithms book.  If the function
   mpz_millerrabin() returns 0 then n is not prime.  If it returns 1, then n is
   'probably' prime.  The probability of a false positive is (1/4)**reps, where
   reps is the number of internal passes of the probabilistic algorithm.  Knuth
   indicates that 25 passes are reasonable.

   With the current implementation, the first 24 MR-tests are substituted by a
   Baillie-PSW probable prime test.

   This implementation the Baillie-PSW test was checked up to 31*2^46,
   for smaller values no MR-test is performed, regardless of reps, and
   2 ("surely prime") is returned if the number was not proved composite.

   If GMP_BPSW_NOFALSEPOSITIVES_UPTO_64BITS is defined as non-zero,
   the code assumes that the Baillie-PSW test was checked up to 2^64.

   THE FUNCTIONS IN THIS FILE ARE FOR INTERNAL USE ONLY.  THEY'RE ALMOST
   CERTAIN TO BE SUBJECT TO INCOMPATIBLE CHANGES OR DISAPPEAR COMPLETELY IN
   FUTURE GNU MP RELEASES.

Copyright 1991, 1993, 1994, 1996-2002, 2005, 2014, 2018, 2019 Free
Software Foundation, Inc.

Contributed by John Amanatides.

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

#ifndef GMP_BPSW_NOFALSEPOSITIVES_UPTO_64BITS
#define GMP_BPSW_NOFALSEPOSITIVES_UPTO_64BITS 0
#endif

static int millerrabin (mpz_srcptr,
			mpz_ptr, mpz_ptr,
			mpz_srcptr, unsigned long int);

int
mpz_millerrabin (mpz_srcptr n, int reps)
{
  mpz_t nm, x, y, q;
  unsigned long int k;
  gmp_randstate_t rstate;
  int is_prime;
  TMP_DECL;
  TMP_MARK;

  ASSERT (SIZ (n) > 0);
  MPZ_TMP_INIT (nm, SIZ (n) + 1);
  mpz_tdiv_q_2exp (nm, n, 1);

  MPZ_TMP_INIT (x, SIZ (n) + 1);
  MPZ_TMP_INIT (y, 2 * SIZ (n)); /* mpz_powm_ui needs excessive memory!!! */
  MPZ_TMP_INIT (q, SIZ (n));

  /* Find q and k, where q is odd and n = 1 + 2**k * q.  */
  k = mpz_scan1 (nm, 0L);
  mpz_tdiv_q_2exp (q, nm, k);
  ++k;

  /* BPSW test */
  mpz_set_ui (x, 2);
  is_prime = millerrabin (n, x, y, q, k) && mpz_stronglucas (n, x, y);

  if (is_prime)
    {
      if (
#if GMP_BPSW_NOFALSEPOSITIVES_UPTO_64BITS
	  /* Consider numbers up to 2^64 that pass the BPSW test as primes. */
#if GMP_NUMB_BITS <= 64
	  SIZ (n) <= 64 / GMP_NUMB_BITS
#else
	  0
#endif
#if 64 % GMP_NUMB_BITS != 0
	  || SIZ (n) - 64 / GMP_NUMB_BITS == (PTR (n) [64 / GMP_NUMB_BITS] < CNST_LIMB(1) << 64 % GMP_NUMB_BITS)
#endif
#else
	  /* Consider numbers up to 31*2^46 that pass the BPSW test as primes.
	     This implementation was tested up to 31*2^46 */
	  /* 2^4 < 31 = 0b11111 < 2^5 */
#define GMP_BPSW_LIMB_CONST CNST_LIMB(31)
#define GMP_BPSW_BITS_CONST (LOG2C(31) - 1)
#define GMP_BPSW_BITS_LIMIT (46 + GMP_BPSW_BITS_CONST)

#define GMP_BPSW_LIMBS_LIMIT (GMP_BPSW_BITS_LIMIT / GMP_NUMB_BITS)
#define GMP_BPSW_BITS_MOD (GMP_BPSW_BITS_LIMIT % GMP_NUMB_BITS)

#if GMP_NUMB_BITS <=  GMP_BPSW_BITS_LIMIT
	  SIZ (n) <= GMP_BPSW_LIMBS_LIMIT
#else
	  0
#endif
#if GMP_BPSW_BITS_MOD >=  GMP_BPSW_BITS_CONST
	  || SIZ (n) - GMP_BPSW_LIMBS_LIMIT == (PTR (n) [GMP_BPSW_LIMBS_LIMIT] < GMP_BPSW_LIMB_CONST << (GMP_BPSW_BITS_MOD - GMP_BPSW_BITS_CONST))
#else
#if GMP_BPSW_BITS_MOD != 0
	  || SIZ (n) - GMP_BPSW_LIMBS_LIMIT == (PTR (n) [GMP_BPSW_LIMBS_LIMIT] < GMP_BPSW_LIMB_CONST >> (GMP_BPSW_BITS_CONST -  GMP_BPSW_BITS_MOD))
#else
#if GMP_NUMB_BITS > GMP_BPSW_BITS_CONST
	  || SIZ (nm) - GMP_BPSW_LIMBS_LIMIT + 1 == (PTR (nm) [GMP_BPSW_LIMBS_LIMIT - 1] < GMP_BPSW_LIMB_CONST << (GMP_NUMB_BITS - 1 - GMP_BPSW_BITS_CONST))
#endif
#endif
#endif

#undef GMP_BPSW_BITS_LIMIT
#undef GMP_BPSW_LIMB_CONST
#undef GMP_BPSW_BITS_CONST
#undef GMP_BPSW_LIMBS_LIMIT
#undef GMP_BPSW_BITS_MOD

#endif
	  )
	is_prime = 2;
      else
	{
	  reps -= 24;
	  if (reps > 0)
	    {
	      /* (n-5)/2 */
	      mpz_sub_ui (nm, nm, 2L);
	      ASSERT (mpz_cmp_ui (nm, 1L) >= 0);

	      gmp_randinit_default (rstate);

	      do
		{
		  /* 3 to (n-1)/2 inclusive, don't want 1, 0 or 2 */
		  mpz_urandomm (x, rstate, nm);
		  mpz_add_ui (x, x, 3L);

		  is_prime = millerrabin (n, x, y, q, k);
		} while (--reps > 0 && is_prime);

	      gmp_randclear (rstate);
	    }
	}
    }
  TMP_FREE;
  return is_prime;
}

static int
mod_eq_m1 (mpz_srcptr x, mpz_srcptr m)
{
  mp_size_t ms;
  mp_srcptr mp, xp;

  ms = SIZ (m);
  if (SIZ (x) != ms)
    return 0;
  ASSERT (ms > 0);

  mp = PTR (m);
  xp = PTR (x);
  ASSERT ((mp[0] - 1) == (mp[0] ^ 1)); /* n is odd */

  if ((*xp ^ CNST_LIMB(1) ^ *mp) != CNST_LIMB(0)) /* xp[0] != mp[0] - 1 */
    return 0;
  else
    {
      int cmp;

      --ms;
      ++xp;
      ++mp;

      MPN_CMP (cmp, xp, mp, ms);

      return cmp == 0;
    }
}

static int
millerrabin (mpz_srcptr n, mpz_ptr x, mpz_ptr y,
	     mpz_srcptr q, unsigned long int k)
{
  unsigned long int i;

  mpz_powm (y, x, q, n);

  if (mpz_cmp_ui (y, 1L) == 0 || mod_eq_m1 (y, n))
    return 1;

  for (i = 1; i < k; i++)
    {
      mpz_powm_ui (y, y, 2L, n);
      if (mod_eq_m1 (y, n))
	return 1;
      /* y == 1 means that the previous y was a non-trivial square root
	 of 1 (mod n). y == 0 means that n is a power of the base.
	 In either case, n is not prime. */
      if (mpz_cmp_ui (y, 1L) <= 0)
	return 0;
    }
  return 0;
}
