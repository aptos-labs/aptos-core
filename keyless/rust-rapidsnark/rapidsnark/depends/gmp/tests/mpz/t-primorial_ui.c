/* Exercise mpz_primorial_ui.

Copyright 2000-2002, 2012, 2015 Free Software Foundation, Inc.

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


/* Usage: t-primorial_ui [x|num]

   With no arguments testing goes up to the initial value of "limit" below.
   With a number argument tests are carried that far, or with a literal "x"
   tests are continued without limit (this being meant only for development
   purposes).  */

static int isprime (unsigned long int t);

int
main (int argc, char *argv[])
{
  unsigned long  n;
  unsigned long  limit = 2222;
  gmp_randstate_ptr rands;
  mpz_t          f, r, bs;

  tests_start ();
  rands = RANDS;

  if (argc > 1 && argv[1][0] == 'x')
    limit = ULONG_MAX;
  else
    TESTS_REPS (limit, argv, argc);

  /* for small limb testing */
  limit = MIN (limit, MP_LIMB_T_MAX);

  mpz_init_set_ui (f, 1);  /* 0# = 1 */
  mpz_init (r);

  n = 0;
  do
    {
      mpz_primorial_ui (r, n);
      MPZ_CHECK_FORMAT (r);

      if (mpz_cmp (f, r) != 0)
	{
	  printf ("mpz_primorial_ui(%lu) wrong\n", n);
	  printf ("  got  "); mpz_out_str (stdout, 10, r); printf("\n");
	  printf ("  want "); mpz_out_str (stdout, 10, f); printf("\n");
	  abort ();
	}

      if (isprime (++n))
	mpz_mul_ui (f, f, n);  /* p# = (p-1)# * (p) */
      if (n%16 == 0) { mpz_clear (r); mpz_init (r); }
    } while (n < limit);

  n = 0; limit =1;
  mpz_init (bs);
  do
    {
      unsigned long i, d;

      mpz_urandomb (bs, rands, 21);
      i = mpz_get_ui (bs);
      mpz_urandomb (bs, rands, 9);
      d = mpz_get_ui (bs) + 3*64;
      mpz_primorial_ui (f, i);
      MPZ_CHECK_FORMAT (f);
      mpz_primorial_ui (r, i+d);
      MPZ_CHECK_FORMAT (r);

      do {
	if (isprime (++i))
	  mpz_mul_ui (f, f, i);
      } while (--d != 0);

      if (mpz_cmp (f, r) != 0)
	{
	  printf ("mpz_primorial_ui(%lu) wrong\n", i);
	  printf ("  got  "); mpz_out_str (stdout, 10, r); printf("\n");
	  printf ("  want "); mpz_out_str (stdout, 10, f); printf("\n");
	  abort ();
	}
    } while (++n < limit);
  /* Chech a single "big" value, modulo a larger prime */
  n = 2095637;
  mpz_primorial_ui (r, n);
  mpz_set_ui (f, 13);
  mpz_setbit (f, 64); /* f = 2^64 + 13 */
  mpz_tdiv_r (r, r, f);
  mpz_set_str (f, "BAFCBF3C95B217D5", 16);

  if (mpz_cmp (f, r) != 0)
    {
      printf ("mpz_primorial_ui(%lu) wrong\n", n);
      printf ("  got  "); mpz_out_str (stdout, 10, r); printf("\n");
      printf ("  want "); mpz_out_str (stdout, 10, f); printf("\n");
      abort ();
    }

  mpz_clear (bs);
  mpz_clear (f);
  mpz_clear (r);

  tests_end ();

  exit (0);
}

static int
isprime (unsigned long int t)
{
  unsigned long int q, r, d;

  if (t < 3 || (t & 1) == 0)
    return t == 2;

  for (d = 3, r = 1; r != 0; d += 2)
    {
      q = t / d;
      r = t - q * d;
      if (q < d)
	return 1;
    }
  return 0;
}
