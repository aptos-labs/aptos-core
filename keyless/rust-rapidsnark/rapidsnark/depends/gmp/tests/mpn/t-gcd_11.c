/* Test mpn_gcd_11.

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
#define COUNT 500000
#endif

static void
one_test (mp_limb_t a, mp_limb_t b, mp_limb_t ref)
{
  mp_limb_t r = mpn_gcd_11 (a, b);
  if (r != ref)
    {
      gmp_fprintf (stderr,
		   "gcd_11 (0x%Mx, 0x%Mx) failed, got: 0x%Mx, ref: 0x%Mx\n",
		   a, b, r, ref);
      abort();
    }
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

      mpz_setbit (a, 0);
      mpz_setbit (b, 0);
      al = mpz_getlimbn (a, 0);
      bl = mpz_getlimbn (b, 0);
      one_test (al, bl, refmpn_gcd_11 (al, bl));
    }

  mpz_clear (a);
  mpz_clear (b);

  tests_end ();
  return 0;
}
