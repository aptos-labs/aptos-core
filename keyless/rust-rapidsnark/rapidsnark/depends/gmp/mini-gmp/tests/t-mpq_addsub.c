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
testcmpui ()
{
  unsigned d1, d2, n1, n2;
  mpq_t q1, q2;

  mpq_init (q1);
  mpq_init (q2);

  for (d1 = 1; d1 < 6; d1 += 2)
    for (n1 = 1; n1 < 6; n1 *= 2)
      {
	mpq_set_ui (q1, n1, d1);
	for (d2 = 1; d2 < 6; d2 += 2)
	  for (n2 = 1; n2 < 6; n2 *= 2)
	    {
	      int fres = mpq_cmp_ui (q1, n2, d2);
	      int ref = (d1*n2 < d2*n1) - (d1*n2 > d2*n1);

	      mpq_set_ui (q2, n2, d2);

	      if ((!ref) != mpq_equal (q1, q2))
		{
		  fprintf (stderr, "mpz_equal failed: %i / %i = %i / %i ? %i\n", n1, d1, n2, d2, ref);
		  abort ();
		}

	      if (ref != fres)
		{
		  fprintf (stderr, "mpz_cmp_ui failed: %i / %i = %i / %i ? %i != %i\n", n1, d1, n2, d2, ref, fres);
		  abort ();
		}
	    }
      }

  mpq_clear (q1);
  mpq_clear (q2);
}

void
testmain (int argc, char **argv)
{
  unsigned i;
  mpz_t a, b, q, r, c;
  mpq_t rr, ii, ff;
  int tst;

  testcmpui ();
  mpz_init (a);
  mpz_init (b);
  mpz_init (r);
  mpz_init (q);
  mpz_init (c);
  mpq_init (rr);
  mpq_init (ff);
  mpq_init (ii);

  for (i = 0; i < COUNT; i++)
    {
      mini_random_op4 (OP_TDIV, MAXBITS, a, b, q, r);

      _mpq_set_zz (rr, a, b);
      _mpq_set_zz (ff, r, b);

      mpq_set_z (ii, q);

      mpz_set_q (c, rr);
      if (mpz_cmp (c, q))
	{
	  fprintf (stderr, "mpz_set_q failed:\n");
	  dump ("a", a);
	  dump ("b", b);
	  dump ("c", c);
	  dump ("q", q);
	  abort ();
	}

      if ((mpz_sgn (r) != 0) ^ (mpz_cmp_ui (mpq_denref (rr), 1) != 0))
	{
	  fprintf (stderr, "mpq_canonicalize failed:\n");
	  dump ("a", a);
	  dump ("b", b);
	  dump ("r", r);
	  dump ("D", mpq_denref (rr));
	  abort ();
	}

      if (i & 1)
	{
	  if (mpz_fits_slong_p (q))
	    tst = mpq_cmp_si (rr, mpz_get_si (q), 1);
	  else if (mpz_fits_ulong_p (q))
	    tst = mpq_cmp_ui (rr, mpz_get_ui (q), 1);
	  else
	    tst = mpq_cmp_z (rr, q);
	  if (mpz_sgn (b) < 0)
	    tst = - tst;
	  if ((tst != mpz_sgn (r)) && ((tst < 0 && mpz_sgn (r) >= 0) || (tst > 0 && mpz_sgn (r) <= 0)))
	    {
	      fprintf (stderr, "mpq_cmp ii failed: %i %i\n", tst, mpz_sgn (r));
	      dump ("a", a);
	      dump ("b", b);
	      dump ("r", r);
	      dump ("q", q);
	      abort ();
	    }
	}
      else
	{
	  if (mpz_fits_ulong_p (b) && mpz_fits_slong_p (r))
	    tst = mpq_cmp_si (rr, mpz_get_si (r), mpz_get_ui (b));
	  else if (mpz_fits_ulong_p (b) && mpz_fits_ulong_p (r))
	    tst = mpq_cmp_ui (rr, mpz_get_ui (r), mpz_get_ui (b));
	  else
	    tst = mpq_cmp (rr, ff);
	  if ((tst != mpz_sgn (q)) && ((tst < 0 && mpz_sgn (q) >= 0) || (tst > 0 && mpz_sgn (q) <= 0)))
	    {
	      fprintf (stderr, "mpq_cmp ff failed: %i %i\n", tst, mpz_sgn (q));
	      dump ("a", a);
	      dump ("b", b);
	      dump ("r", r);
	      dump ("q", q);
	      abort ();
	    }
	}

      if (i & 1)
	{
	  mpq_sub (rr, rr, ff);
	}
      else
	{
	  mpq_neg (ff, ff);
	  mpq_add (rr, ff, rr);
	}

      if (!mpq_equal (ii, rr))
	{
	  fprintf (stderr, "mpq_%s failed:\n", (i & 1) ? "sub" : "add");
	  dump ("a", a);
	  dump ("b", b);
	  dump ("r", r);
	  dump ("q", q);
	  abort ();
	}
    }

  mpz_clear (a);
  mpz_clear (b);
  mpz_clear (r);
  mpz_clear (q);
  mpz_clear (c);
  mpq_clear (rr);
  mpq_clear (ff);
  mpq_clear (ii);
}
