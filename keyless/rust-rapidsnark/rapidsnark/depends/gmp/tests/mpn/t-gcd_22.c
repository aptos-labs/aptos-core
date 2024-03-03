/* Test mpn_gcd_22.

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
#define COUNT 150000
#endif

static void
one_test (mpz_srcptr a, mpz_srcptr b, mpz_srcptr ref)
{
  mp_double_limb_t r = mpn_gcd_22 (mpz_getlimbn (a, 1), mpz_getlimbn (a, 0),
				   mpz_getlimbn (b, 1), mpz_getlimbn (b, 0));
  if (r.d0 != mpz_getlimbn (ref, 0) || r.d1 != mpz_getlimbn (ref, 1))
    {
      gmp_fprintf (stderr,
		   "gcd_22 (0x%Zx, 0x%Zx) failed, got: g1 = 0x%Mx g0 = %Mx, ref: 0x%Zx\n",
                   a, b, r.d1, r.d0, ref);
      abort();
    }
}

int
main (int argc, char **argv)
{
  mpz_t a, b, ref;
  int count = COUNT;
  int test;
  gmp_randstate_ptr rands;

  TESTS_REPS (count, argv, argc);

  tests_start ();
  rands = RANDS;

  mpz_init (a);
  mpz_init (b);
  mpz_init (ref);
  for (test = 0; test < count; test++)
    {
      mp_bitcnt_t asize = 1 + gmp_urandomm_ui(rands, 2*GMP_NUMB_BITS);
      mp_bitcnt_t bsize = 1 + gmp_urandomm_ui(rands, 2*GMP_NUMB_BITS);
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
      refmpz_gcd (ref, a, b);
      one_test (a, b, ref);
    }

  mpz_clear (a);
  mpz_clear (b);
  mpz_clear (ref);

  tests_end ();
  return 0;
}
