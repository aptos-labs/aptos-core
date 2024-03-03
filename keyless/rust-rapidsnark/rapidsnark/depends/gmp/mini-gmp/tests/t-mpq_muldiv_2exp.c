/*

Copyright 2012, 2013, 2018 Free Software Foundation, Inc.

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

#include <assert.h>
#include <stdlib.h>
#include <stdio.h>

#include "testutils.h"
#include "../mini-mpq.h"

#define MAXBITS 300
#define COUNT 10000

static void
_mpq_set_zz (mpq_t q, mpz_t n, mpz_t d)
{
  if (mpz_fits_ulong_p (d) && mpz_fits_slong_p (n))
    {
      mpq_set_si (q, mpz_get_si (n), mpz_get_ui (d));
    }
  else if (mpz_fits_ulong_p (d) && mpz_fits_ulong_p (n))
    {
      mpq_set_ui (q, mpz_get_ui (n), mpz_get_ui (d));
    }
  else
    {
      mpq_set_num (q, n);
      mpq_set_den (q, d);
    }
  mpq_canonicalize (q);
}

void
testmain (int argc, char **argv)
{
  unsigned i;
  mpz_t a, b, t;
  mpq_t aq, rq, tq;
  mp_bitcnt_t e;
  long int e2, t1, t2;

  mpz_init (a);
  mpz_init (b);
  mpz_init (t);
  mpq_init (aq);
  mpq_init (rq);
  mpq_init (tq);

  for (i = 0; i < COUNT; i++)
    {
      do {
	mini_random_bit_op (OP_COMBIT, MAXBITS, a, &e, b);
      } while (mpz_sgn (a) == 0 || mpz_sgn (b) == 0);

      _mpq_set_zz (aq, a, b);
      e2 = mpz_scan1 (a, 0);
      e2-= mpz_scan1 (b, 0);

      mpq_mul_2exp (rq, aq, e);
      t1 = mpz_scan1 (mpq_numref (rq), 0);
      t2 = mpz_scan1 (mpq_denref (rq), 0);
      mpq_neg (tq, rq);
      mpq_div (tq, aq, tq);
      mpq_get_den (t, tq);

      if (e2 + e != t1 - t2 || (t2 != 0 && t1 != 0) || mpz_scan1 (t, 0) != e
	  || mpz_sizeinbase (t, 2) - 1 != e || mpz_cmp_si (mpq_numref (tq), -1) != 0)
	{
	  fprintf (stderr, "mpq_mul_2exp failed: %lu\n", e);
	  dump ("na", a);
	  dump ("da", b);
	  dump ("nr", mpq_numref (rq));
	  dump ("dr", mpq_denref (rq));
	  abort ();
	}

      mpq_div_2exp (rq, aq, e);
      t1 = mpz_scan1 (mpq_numref (rq), 0);
      t2 = mpz_scan1 (mpq_denref (rq), 0);
      mpq_div (aq, aq, rq);
      mpq_get_num (t, aq);

      if (e2 != t1 - t2 + e || (t2 != 0 && t1 != 0) || mpz_scan1 (t, 0) != e
	  || mpz_sizeinbase (t, 2) - 1 != e || mpz_cmp_ui (mpq_denref (aq), 1) != 0)
	{
	  fprintf (stderr, "mpq_div_2exp failed: %lu\n", e);
	  fprintf (stderr, "%li %li %lu %lu\n", e2, t2, mpz_scan1 (t, 0), (unsigned long) mpz_sizeinbase (t, 2));
	  dump ("na", a);
	  dump ("da", b);
	  dump ("nr", mpq_numref (rq));
	  dump ("dr", mpq_denref (rq));
	  abort ();
	}

      mpq_set_ui (aq, 0, 1);
      mpq_set_ui (rq, 6, 7);
      mpq_set (tq, aq);
      mpq_div_2exp (rq, aq, e);

      if (!mpq_equal (tq, rq))
	{
	  fprintf (stderr, "mpq_div_2exp failed on zero: %lu\n", e);
	  abort ();
	}

      mpq_set_ui (rq, 7, 6);
      mpq_mul_2exp (rq, aq, e);

      if (!mpq_equal (rq, tq))
	{
	  fprintf (stderr, "mpq_mul_2exp failed on zero: %lu\n", e);
	  abort ();
	}
    }

  mpz_clear (a);
  mpz_clear (b);
  mpz_clear (t);
  mpq_clear (aq);
  mpq_clear (rq);
  mpq_clear (tq);
}
