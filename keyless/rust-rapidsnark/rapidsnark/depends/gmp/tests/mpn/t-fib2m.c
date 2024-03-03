/* Test mpn_fib2m.

Copyright 2018 Free Software Foundation, Inc.

This file is part of the GNU MP Library test suite.

The GNU MP Library test suite is free software; you can redistribute it
and/or modify it under the terms of the GNU General Public License as
published by the Free Software Foundation; either version 3 of the License,
or (at your option) any later version.

The GNU MP Library test suite is distributed in the hope that it will be
useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General
Public License for more details.

You should have received a copy of the GNU General Public License along with
the GNU MP Library test suite.  If not, see https://www.gnu.org/licenses/.  */

#include <stdio.h>
#include <stdlib.h>

#include "gmp-impl.h"
#include "tests.h"

#define MAX_K_BITS 16
#define MAX_K (1L << MAX_K_BITS)
#define MIN_K 1

#define MAX_MN 20
#define MAX_KN 30

#define COUNT 200

static int
test_fib2_fib2m (int count, gmp_randstate_ptr rands)
{
  int test;
  mp_ptr fk, fks1, fkm, fks1m, mp, qp;
  mp_size_t mn, fn, size, max_mn;
  TMP_DECL;

  size = MPN_FIB2_SIZE (MAX_K);
  max_mn = size / 4 + 10;
  ASSERT (max_mn < size);

  TMP_MARK;
  fk	 = TMP_ALLOC_LIMBS (size);
  fks1	 = TMP_ALLOC_LIMBS (size);
  qp	 = TMP_ALLOC_LIMBS (size);
  mp	 = TMP_ALLOC_LIMBS (max_mn);
  fkm	 = 1 + TMP_ALLOC_LIMBS (max_mn * 2 + 1 + 2);
  fks1m	 = 1 + TMP_ALLOC_LIMBS (max_mn * 2 + 1 + 2);

  for (test = 1; test <= count; ++test)
    {
      mp_limb_t fk_before, fk_after, fk1_before, fk1_after;
      int signflip;
      unsigned long k;

      k = MIN_K +
	gmp_urandomm_ui (rands, test < MAX_K_BITS ?
			 MAX_K >> test : (MAX_K - MIN_K));

      fn = mpn_fib2_ui (fk, fks1, k);
      do {
	mn = gmp_urandomm_ui (rands, MAX_K) % (fn / 4 + 10);
      } while (mn == 0);
      ASSERT (mn <= max_mn);
      mpn_random2 (mp, mn);
      ASSERT (mp [mn - 1] != 0);

      if (fn >= mn)
	{
	  mpn_tdiv_qr (qp, fk, 0, fk, fn, mp, mn);
	  mpn_tdiv_qr (qp, fks1, 0, fks1, fn, mp, mn);
	}
      else
	{
	  MPN_ZERO (fk + fn, mn - fn);
	  MPN_ZERO (fks1 + fn, mn - fn);
	}

      mpn_random2 (fkm - 1, 2*mn+1+2);
      fk_before = fkm [-1];
      fk_after = fkm [2 * mn + 1];

      mpn_random2 (fks1m - 1, 2*mn+1+2);
      fk1_before = fks1m [-1];
      fk1_after = fks1m [2 * mn + 1];

      qp [0] = k;
      signflip = mpn_fib2m (fkm, fks1m, qp, 1, mp, mn);
      if (fkm [-1] != fk_before || fkm [2 * mn + 1] != fk_after
	  || fks1m [-1] != fk1_before || fks1m [2 * mn + 1] != fk1_after)
	{
	  printf ("REDZONE violation in test %d, k = %lu, mn = %u\n",
		  test, k, (unsigned) mn);
	  if (fkm[-1] != fk_before)
	    {
	      printf ("before fkm:"); mpn_dump (fkm - 1, 1);
	      printf ("keep:   "); mpn_dump (&fk_before, 1);
	    }
	  if (fkm[2 * mn + 1] != fk_after)
	    {
	      printf ("after fkm:"); mpn_dump (fkm + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fk_after, 1);
	    }
	  if (fks1m[-1] != fk1_before)
	    {
	      printf ("before fks1m:"); mpn_dump (fks1m - 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1_before, 1);
	    }
	  if (fks1m[2 * mn + 1] != fk1_after)
	    {
	      printf ("after fks1m:"); mpn_dump (fks1m + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1_after, 1);
	    }
	  abort();
	}

      if (mpn_cmp (fkm, fk, mn) != 0)
	{
	  if (mpn_sub_n (fk, mp, fk, mn) || mpn_cmp (fkm, fk, mn) != 0)
	    {
	      printf ("ERROR(k) in test %d, k = %lu, mn = %u\n",
		      test, k, (unsigned) mn);
	      mpn_dump (fk, mn);
	      mpn_dump (fkm, mn);
	      mpn_dump (mp, mn);
	      abort();
	    }
	  signflip ^= 1;
	}

      if (mpn_cmp (fks1m, fks1, mn) != 0)
	{
	  if (mpn_sub_n (fks1, mp, fks1, mn) || mpn_cmp (fks1m, fks1, mn) != 0)
	    {
	      printf ("ERROR(k-1) in test %d, k = %lu, mn = %u\n",
		      test, k, (unsigned) mn);
	      mpn_dump (fks1, mn);
	      mpn_dump (fks1m, mn);
	      mpn_dump (mp, mn);
	      abort();
	    }
	  signflip ^= 1;
	}

      if (signflip != 0 && ! mpn_zero_p (fks1m, mn) && ! mpn_zero_p (fkm, mn))
	{
	  if ((mp [0] & 1) == 0) /* Should we test only odd modulus-es? */
	    {
	      if (! mpn_lshift (fks1m, fks1m, mn, 1) &&
		  mpn_cmp (mp, fks1m, mn) == 0)
		continue;
	      if (! mpn_lshift (fkm, fkm, mn, 1) &&
		  mpn_cmp (mp, fkm, mn) == 0)
		continue;
	    }
	  printf ("ERROR(sign) in test %d, k = %lu, mn = %u\n",
		  test, k, (unsigned) mn);
	  abort();
	}
    }
  TMP_FREE;
  return 0;
}

static int
test_fib2m_2exp (int count, gmp_randstate_ptr rands)
{
  int test;
  mp_ptr fka, fks1a, fkb, fks1b, mp, kp;
  TMP_DECL;

  TMP_MARK;
  kp	 = TMP_ALLOC_LIMBS (MAX_KN);
  mp	 = TMP_ALLOC_LIMBS (MAX_MN);
  fka	 = 1 + TMP_ALLOC_LIMBS (MAX_MN * 2 + 1 + 2);
  fks1a	 = 1 + TMP_ALLOC_LIMBS (MAX_MN * 2 + 1 + 2);
  fkb	 = 1 + TMP_ALLOC_LIMBS (MAX_MN * 2 + 1 + 2);
  fks1b	 = 1 + TMP_ALLOC_LIMBS (MAX_MN * 2 + 1 + 2);

  for (test = 1; test <= count; ++test)
    {
      mp_limb_t fka_before, fka_after, fk1a_before, fk1a_after;
      mp_limb_t fkb_before, fkb_after, fk1b_before, fk1b_after;
      mp_size_t mn, kn;
      int signflip;
      mp_bitcnt_t exp2;

      mn = gmp_urandomm_ui (rands, MAX_MN - 1) + 1;
      mpn_random2 (mp, mn);

      exp2 = MIN_K + 1 + gmp_urandomm_ui (rands, MAX_KN * GMP_NUMB_BITS - MIN_K - 1);

      kn = BITS_TO_LIMBS (exp2);
      MPN_ZERO (kp, kn - 1);
      kp [kn - 1] = CNST_LIMB (1) << ((exp2 - 1) % GMP_NUMB_BITS);

      mpn_random2 (fka - 1, 2*mn+1+2);
      fka_before = fka [-1];
      fka_after = fka [2 * mn + 1];

      mpn_random2 (fks1a - 1, 2*mn+1+2);
      fk1a_before = fks1a [-1];
      fk1a_after = fks1a [2 * mn + 1];

      signflip = mpn_fib2m (fka, fks1a, kp, kn, mp, mn);
      if (fka [-1] != fka_before || fka [2 * mn + 1] != fka_after
	  || fks1a [-1] != fk1a_before || fks1a [2 * mn + 1] != fk1a_after)
	{
	  printf ("REDZONE(a) violation in test %d, exp2 = %lu\n", test, exp2);
	  if (fka[-1] != fka_before)
	    {
	      printf ("before fka:"); mpn_dump (fka - 1, 1);
	      printf ("keep:   "); mpn_dump (&fka_before, 1);
	    }
	  if (fka[2 * mn + 1] != fka_after)
	    {
	      printf ("after fka:"); mpn_dump (fka + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fka_after, 1);
	    }
	  if (fks1a[-1] != fk1a_before)
	    {
	      printf ("before fks1a:"); mpn_dump (fks1a - 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1a_before, 1);
	    }
	  if (fks1a[2 * mn + 1] != fk1a_after)
	    {
	      printf ("after fks1a:"); mpn_dump (fks1a + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1a_after, 1);
	    }
	  abort();
	}

      if (signflip && ! mpn_zero_p (fks1a, mn))
	mpn_sub_n (fks1a, mp, fks1a, mn);
      if (mpn_sub_n (fka, fka, fks1a, mn))
	ASSERT_CARRY (mpn_add_n (fka, fka, mp, mn));

      mpn_sub_1 (kp, kp, kn, 1);
      ASSERT (exp2 % GMP_NUMB_BITS == 1 || kp [kn - 1] != 0);
      kn -= kp [kn - 1] == 0;

      mpn_random2 (fkb - 1, 2*mn+1+2);
      fkb_before = fkb [-1];
      fkb_after = fkb [2 * mn + 1];

      mpn_random2 (fks1b - 1, 2*mn+1+2);
      fk1b_before = fks1b [-1];
      fk1b_after = fks1b [2 * mn + 1];

      signflip = mpn_fib2m (fkb, fks1b, kp, kn, mp, mn);
      if (fkb [-1] != fkb_before || fkb [2 * mn + 1] != fkb_after
	  || fks1b [-1] != fk1b_before || fks1b [2 * mn + 1] != fk1b_after)
	{
	  printf ("REDZONE(b) violation in test %d, exp2 = %lu\n", test, exp2);
	  if (fkb[-1] != fkb_before)
	    {
	      printf ("before fkb:"); mpn_dump (fkb - 1, 1);
	      printf ("keep:   "); mpn_dump (&fkb_before, 1);
	    }
	  if (fkb[2 * mn + 1] != fkb_after)
	    {
	      printf ("after fkb:"); mpn_dump (fkb + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fkb_after, 1);
	    }
	  if (fks1b[-1] != fk1b_before)
	    {
	      printf ("before fks1b:"); mpn_dump (fks1b - 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1b_before, 1);
	    }
	  if (fks1b[2 * mn + 1] != fk1b_after)
	    {
	      printf ("after fks1b:"); mpn_dump (fks1b + 2 * mn + 1, 1);
	      printf ("keep:   "); mpn_dump (&fk1b_after, 1);
	    }
	  abort();
	}

      if (mpn_cmp (fks1a, fkb, mn) != 0)
	{
	  if (mpn_sub_n (fkb, mp, fkb, mn) || mpn_cmp (fks1a, fkb, mn) != 0)
	    {
	      printf ("ERROR(k) in test %d, exp2 = %lu\n", test, exp2);
	      mpn_dump (fks1a, mn);
	      mpn_dump (fkb, mn);
	      mpn_dump (mp, mn);
	      abort();
	    }
	  signflip ^= 1;
	}

      if (mpn_cmp (fka, fks1b, mn) != 0)
	{
	  if (mpn_sub_n (fks1b, mp, fks1b, mn) || mpn_cmp (fka, fks1b, mn) != 0)
	    {
	      printf ("ERROR(k-1) in test %d, exp2 = %lu\n", test, exp2);
	      mpn_dump (fka, mn);
	      mpn_dump (fks1b, mn);
	      mpn_dump (mp, mn);
	      abort();
	    }
	  signflip ^= 1;
	}

      if (signflip != 0 && ! mpn_zero_p (fks1b, mn) && ! mpn_zero_p (fkb, mn))
	{
	  if ((mp [0] & 1) == 0) /* Should we test only odd modulus-es? */
	    {
	      if (! mpn_lshift (fks1b, fks1b, mn, 1) &&
		  mpn_cmp (mp, fks1b, mn) == 0)
		continue;
	      if (! mpn_lshift (fkb, fkb, mn, 1) &&
		  mpn_cmp (mp, fkb, mn) == 0)
		continue;
	    }
	  printf ("ERROR(sign) in test %d, exp2 = %lu\n",
		  test, exp2);
	  abort();
	}
    }
  TMP_FREE;
  return 0;
}

int
main (int argc, char **argv)
{
  int count = COUNT;
  gmp_randstate_ptr rands;

  tests_start ();
  TESTS_REPS (count, argv, argc);
  rands = RANDS;

  test_fib2_fib2m (count / 2, rands);
  test_fib2m_2exp (count / 2, rands);

  tests_end ();
  exit (0);
}
