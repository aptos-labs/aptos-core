/* Test mpz_powm, mpz_lucas_mod.

Copyright 1991, 1993, 1994, 1996, 1999-2001, 2009, 2012, 2018 Free Software
Foundation, Inc.

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
#include <string.h>

#include "gmp-impl.h"
#include "tests.h"

void debug_mp (mpz_t, int);

#define SIZEM 8

/* FIXME: Should we implement another sequence to test lucas mod?	*/
/* Eg: a generalisation of what we use for Fibonacci:	*/
/* U_{2n-1} = U_n^2 - Q*U_{n-1}^2	*/
/* U_{2n+1} = D*U_n^2  + Q*U_{2n-1} + 2*Q^n ; whith D = (P^2-4*Q)	*/
/* P*U_{2n} = U_{2n+1} + Q*U_{2n-1}	*/

int
main (int argc, char **argv)
{
  mpz_t base, exp, mod;
  mpz_t r1, r2, t1, t2;
  mp_size_t base_size, exp_size, mod_size;
  int i, res;
  int reps = 1000;
  long Q;
  gmp_randstate_ptr rands;
  mpz_t bs;
  unsigned long bsi, size_range;

  tests_start ();
  TESTS_REPS (reps, argv, argc);

  rands = RANDS;

  mpz_init (bs);

  mpz_init (base);
  mpz_init (exp);
  mpz_init (mod);
  mpz_init (r1);
  mpz_init (r2);
  mpz_init (t1);
  mpz_init (t2);

  for (i = 0; i < reps; i++)
    {
      mpz_urandomb (bs, rands, 32);
      size_range = mpz_get_ui (bs) % SIZEM + 1;

      do  /* Loop until base >= 2 and fits in a long.  */
	{
	  mpz_urandomb (base, rands, BITS_PER_ULONG - 2);
	}
      while (mpz_cmp_ui (base, 2) < 0 || mpz_fits_slong_p (base) == 0);

      Q = mpz_get_ui (base);

      do
        {
	  ++size_range;
	  size_range = MIN (size_range, SIZEM);
	  mpz_urandomb (bs, rands, size_range);
	  mod_size = mpz_get_ui (bs);
	  mpz_rrandomb (mod, rands, mod_size);
	  mpz_add_ui (mod, mod, 16);
	}
      while (mpz_gcd_ui (NULL, mod, Q) != 1);

      mod_size = mpz_sizeinbase (mod, 2) - 3;
      mpz_urandomb (bs, rands, 32);
      exp_size = mpz_get_ui (bs) % mod_size + 2;

      mpz_tdiv_q_2exp (exp, mod, exp_size);
      mpz_add_ui (exp, exp, 1);

      mpz_urandomb (bs, rands, 2);
      bsi = mpz_get_ui (bs);
      if ((bsi & 1) != 0)
	{
	  mpz_neg (base, base);
	  Q = -Q;
	}

      res = mpz_lucas_mod (t1, r2, Q, exp_size, mod, t2, r1);
      if (res && ++reps)
	continue;
      MPZ_CHECK_FORMAT (r2);
      if (mpz_cmp_ui (r2, 0) < 0)
	mpz_add (r2, r2, mod);
      mpz_powm (r1, base, exp, mod);

      if (mpz_cmp (r1, r2) != 0)
	{
	  fprintf (stderr, "\nIncorrect results in test %d for operands:\n", i);
	  debug_mp (base, -16);
	  debug_mp (exp, -16);
	  debug_mp (mod, -16);
	  fprintf (stderr, "mpz_powm result:\n");
	  debug_mp (r1, -16);
	  fprintf (stderr, "mpz_lucas_mod result (%d) Q=%ld:\n", res, Q);
	  debug_mp (r2, -16);
	  abort ();
	}
    }

  mpz_clear (bs);
  mpz_clear (base);
  mpz_clear (exp);
  mpz_clear (mod);
  mpz_clear (r1);
  mpz_clear (r2);
  mpz_clear (t1);
  mpz_clear (t2);

  tests_end ();
  exit (0);
}

void
debug_mp (mpz_t x, int base)
{
  mpz_out_str (stderr, base, x); fputc ('\n', stderr);
}
