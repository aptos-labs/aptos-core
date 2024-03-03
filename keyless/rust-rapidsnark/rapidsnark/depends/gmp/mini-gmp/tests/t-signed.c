/* Exercise some mpz_..._si functions.

Copyright 2013, 2016 Free Software Foundation, Inc.

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

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

#include "testutils.h"

/* Always called with sz fitting in a signed long, and si is the
   corresponding value. */
int
check_si (const mpz_t sz, long si)
{
  mpz_t t;

  /* Checks on sz/si */
  if ((mpz_cmp_si (sz, si)) != 0)
    {
      printf ("mpz_cmp_si (sz, %ld) != 0.\n", si);
      return 0;
    }
  if (mpz_get_si (sz) != si)
    {
      printf ("mpz_get_si (sz) != %ld.\n", si);
      return 0;
    }

  mpz_init_set_si (t, si);

  if (mpz_cmp (t, sz) != 0)
    {
      printf ("mpz_init_set_si (%ld) failed.\n", si);
      printf (" got="); mpz_out_str (stdout, 10, t); printf ("\n");
      return 0;
    }

  mpz_clear (t);
  return 1;
}

/* Called with mpz_cmp (sz, oz) == c. If sz fits in a signed long,
   si is the coresponding value, and similarly for oz and oi. */
void
check_si_cmp (const mpz_t sz, const mpz_t oz, long si, long oi, int c)
{
  if (mpz_cmp (sz, oz) != c)
    {
      printf ("mpz_cmp (sz, oz) != %i.\n", c);
      goto fail;
    }

  if (mpz_fits_slong_p (sz))
    {
      if (!check_si (sz, si))
	goto fail;
      if (mpz_cmp_si (oz, si) != -c)
	{
	  printf ("mpz_cmp_si (oz, %ld) != %i.\n", si, -c);
	  goto fail;
	}
    }
  else
    {
      if (mpz_cmp_si (sz, si) != c)
	{
	  printf ("mpz_cmp_si (sz, %ld) != %i.\n", si, c);
	  goto fail;
	}
      if (mpz_cmp_si (sz, -c) != c)
	{
	  printf ("mpz_cmp_si (sz, %i) != %i.\n", -c, c);
	  goto fail;
	}
    }
  if (mpz_fits_slong_p (oz))
    {
      if (!check_si (oz, oi))
	goto fail;
      if (mpz_cmp_si (sz, oi) != c)
	{
	  printf ("mpz_cmp_si (sz, %ld) != %i.\n", oi, c);
	  goto fail;
	}
    }
  return;

 fail:
  printf (" sz="); mpz_out_str (stdout, 10, sz); printf ("\n");
  printf (" si=%ld\n", si);
  printf (" oz="); mpz_out_str (stdout, 10, oz); printf ("\n");
  printf (" oi=%ld\n", si);
  abort ();
}

void
try_op_si (int c)
{
  long  si, oi;
  mpz_t sz, oz;
  unsigned overflow_count;

  si = c;
  mpz_init_set_si (sz, si);

  oi = si;
  mpz_init_set (oz, sz);

  /* To get a few tests with operands straddling the border, don't
     stop at the very first operand exceeding a signed long. */
  for (overflow_count = 0; overflow_count < 10; )
    {
      /* c * 2^k */
      mpz_mul_2exp (sz, sz, 1);
      if (mpz_fits_slong_p (sz))
	si *= 2;
      else
	overflow_count++;

      check_si_cmp (sz, oz, si, oi, c);

      /* c * (2^k + 1) */
      if (c == -1)
	mpz_sub_ui (oz, sz, 1);
      else
	mpz_add_ui (oz, sz, 1);
      if (mpz_fits_slong_p (oz))
	oi = si + c;
      else
	overflow_count++;
      check_si_cmp (oz, sz, oi, si, c);

      /* c * (2^K - 1) */
      mpz_mul_si (oz, sz, 2*c);
      if (c == -1)
	mpz_ui_sub (oz, 1, oz); /* oz = sz * 2 + 1 */
      else
	mpz_sub_ui (oz, oz, 1); /* oz = sz * 2 - 1 */
      if (mpz_fits_slong_p (oz))
	oi = (si - c) * 2 + c;
      else
	overflow_count++;

      check_si_cmp (oz, sz, oi, si, c);
    };

  mpz_clear (sz);
  mpz_clear (oz);
}

void
try_fits_slong_p (void)
{
  mpz_t x;
  mpz_init_set_si (x, LONG_MAX);
  if (!mpz_fits_slong_p (x))
    {
      printf ("mpz_fits_slong_p (LONG_MAX) false!\n");
      abort ();
    }
  mpz_add_ui (x, x, 1);
  if (mpz_fits_slong_p (x))
    {
      printf ("mpz_fits_slong_p (LONG_MAX + 1) true!\n");
      abort ();
    }
  mpz_set_si (x, LONG_MIN);
  if (!mpz_fits_slong_p (x))
    {
      printf ("mpz_fits_slong_p (LONG_MIN) false!\n");
      abort ();
    }
  mpz_sub_ui (x, x, 1);
  if (mpz_fits_slong_p (x))
    {
      printf ("mpz_fits_slong_p (LONG_MIN - 1) true!\n");
      abort ();
    }

  mpz_clear (x);
}

void
testmain (int argc, char *argv[])
{
  try_fits_slong_p ();
  try_op_si (-1);
  try_op_si (1);
}
