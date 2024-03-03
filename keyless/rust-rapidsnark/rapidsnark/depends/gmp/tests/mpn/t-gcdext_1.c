/* Test mpn_gcdext_1.

Copyright 2019 Free Software Foundation, Inc.

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

#ifndef COUNT
#define COUNT 250000
#endif

static void
set_signed_limb (mpz_t r, mp_limb_signed_t x)
{
  mpz_t t;
  mp_limb_t abs_x = ABS_CAST(mp_limb_t, x);
  mpz_set (r, mpz_roinit_n (t, &abs_x, 1));
  if (x < 0)
    mpz_neg (r, r);
}

static void
one_test (mp_limb_t a, mp_limb_t b)
{
  mp_limb_signed_t s, t;
  mp_limb_t g;

  g = mpn_gcdext_1 (&s, &t, a, b);

  if (g > 0)
    {
      mpz_t d, sz, tz, tmp;

      mpz_init (d);
      mpz_init (sz);
      mpz_init (tz);

      set_signed_limb (sz, s);
      set_signed_limb (tz, t);

      mpz_mul (d, mpz_roinit_n (tmp, &a, 1), sz);
      mpz_addmul (d, mpz_roinit_n (tmp, &b, 1), tz);

      if (mpz_cmp (d, mpz_roinit_n (tmp, &g, 1)) == 0
	  && a % g == 0 && b % g == 0)
	{
	  mp_limb_t a_div_g = a / g;
	  mp_limb_t b_div_g = b / g;
	  mp_limb_t abs_s = ABS_CAST(mp_limb_t, s);
	  mp_limb_t abs_t = ABS_CAST(mp_limb_t, t);
	  mpz_mul_ui (sz, sz, 2);
	  mpz_mul_ui (tz, tz, 2);
	  if ((abs_s == 1 || mpz_cmpabs (sz, mpz_roinit_n (tmp, &b_div_g, 1)) < 0)
	       && (abs_t == 1 || mpz_cmpabs (tz, mpz_roinit_n (tmp, &a_div_g, 1)) < 0))
	    {
	      mpz_clear (d);
	      mpz_clear (sz);
	      mpz_clear (tz);

	      return;
	    }
	}
    }
  gmp_fprintf (stderr,
	       "gcdext_1 (0x%Mx, 0x%Mx) failed, got: g = 0x%Mx, s = %s0x%Mx, t = %s0x%Mx\n",
	       a, b, g,
	       s < 0 ? "-" : "", ABS_CAST(mp_limb_t, s),
	       t < 0 ? "-" : "", ABS_CAST(mp_limb_t, t));
  abort();
}

int
main (int argc, char **argv)
{
  mpz_t a, b;
  int count = COUNT;
  int test;
  gmp_randstate_ptr rands;

  TESTS_REPS (count, argv, argc);

  tests_start ();
  rands = RANDS;

  mpz_init (a);
  mpz_init (b);
  for (test = 0; test < count; test++)
    {
      mp_limb_t al, bl;
      mp_bitcnt_t asize = 1 + gmp_urandomm_ui(rands, GMP_NUMB_BITS);
      mp_bitcnt_t bsize = 1 + gmp_urandomm_ui(rands, GMP_NUMB_BITS);
      if (test & 1)
	{
	  mpz_urandomb (a, rands, asize);
	  mpz_urandomb (b, rands, bsize);
	}
      else
	{
	  mpz_rrandomb (a, rands, asize);
	  mpz_rrandomb (b, rands, bsize);
	}

      al = mpz_getlimbn (a, 0);
      bl = mpz_getlimbn (b, 0);
      al += (al == 0);
      bl += (bl == 0);

      one_test (al, bl);
    }

  mpz_clear (a);
  mpz_clear (b);

  tests_end ();
  return 0;
}
