/* Test mpz_gcd_ui.

Copyright 2003 Free Software Foundation, Inc.

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

/* Check mpz_gcd_ui doesn't try to return a value out of range.
   This was wrong in gmp 4.1.2 with a long long limb.  */
static void
check_ui_range (void)
{
  unsigned long  got;
  mpz_t  x;
  int  i;

  mpz_init_set_ui (x, ULONG_MAX);

  for (i = 0; i < 20; i++)
    {
      mpz_mul_2exp (x, x, 1L);
      got = mpz_gcd_ui (NULL, x, 0L);
      if (got != 0)
        {
          printf ("mpz_gcd_ui (ULONG_MAX*2^%d, 0)\n", i);
          printf ("   return %#lx\n", got);
          printf ("   should be 0\n");
          abort ();
        }
    }

  mpz_clear (x);
}

static void
check_ui_factors (void)
{
#define NUM_FACTORS 9
  static const char* factors[NUM_FACTORS] = {
    "641", "274177", "3", "5", "17", "257", "65537",
    "59649589127497217", "1238926361552897" };
  unsigned long  got;
  mpz_t  x, b, d, f, g;
  int  i, j;
  gmp_randstate_ptr rands;

  if (GMP_NUMB_BITS < 5 || GMP_NUMB_BITS == 8
      || GMP_NUMB_BITS == 16 || GMP_NUMB_BITS > 511)
    {
      printf ("No usable factors for 2^%i+1.\n", GMP_NUMB_BITS);
      return;
    }

  mpz_init (x);
  mpz_init (d);
  mpz_init (f);
  mpz_init (g);

  mpz_setbit (x, GMP_NUMB_BITS);
  mpz_add_ui (x, x, 1);

  for (i = 0; i < NUM_FACTORS; ++i)
    {
      mpz_set_str (f, factors[i], 10);
      if (mpz_divisible_p (x, f))
	{
	  mpz_mul_2exp (f, f, 1);
	  /* d is an odd multiple of the factor f, exactly filling a limb. */
	  mpz_sub (d, x, f);
	  /* f = 2^GMP_NUMB_BITS mod d. */
	  mpz_sub_ui (f, f, 1);
	  break;
	}
    }

  mpz_gcd (g, f, d);
  if (mpz_even_p (d) || mpz_cmp (d, f) <= 0 || mpz_cmp_ui (g, 1) != 0)
    {
      printf ("No usable factor found.\n");
      abort ();
    }

  rands = RANDS;
  mpz_mul_ui (x, d, gmp_urandomm_ui (rands, 30000) + 1);

  mpz_init (b);
  mpz_setbit (b, GMP_NUMB_BITS - 1);
  for (j = 0; j < 4; ++j)
    {
      mpz_add (x, x, b);

      for (i = 1; i >= -1; --i)
	{
	  if (mpz_fits_ulong_p (d)
	      && ((got = mpz_gcd_ui (NULL, x, mpz_get_ui (d)))
		  != (i != 0 ? 1 : mpz_get_ui (d))))
	    {
	      printf ("mpz_gcd_ui (f, kV+%i*2^%i, V): error (j = %i)\n", i, GMP_NUMB_BITS - 1, j);
	      printf ("   return %#lx\n", got);
	      printf ("   should be %#lx\n", (i != 0 ? 1 : mpz_get_ui (d)));
	      abort ();
	    }

	  mpz_gcd (g, x, d);
	  if ((mpz_cmp_ui (g, 1) == 0) != (i != 0))
	    {
	      printf ("mpz_gcd (f, kV+%i*2^%i, V): error (j = %i)\n", i, GMP_NUMB_BITS - 1, j);
	      printf ("   should%s be one.\n",(i != 0 ? "" : " not"));
	      abort ();
	    }

	  mpz_sub (x, x, b);
	}
      /* Back to the original x. */
      mpz_addmul_ui (x, b, 2);
      mpz_mul (b, b, f);
      mpz_mod (b, b, d);
    }

  mpz_clear (g);
  mpz_clear (x);
  mpz_clear (f);
  mpz_clear (d);
  mpz_clear (b);
}


int
main (void)
{
  tests_start ();

  check_ui_range ();
  check_ui_factors ();

  tests_end ();
  exit (0);
}
