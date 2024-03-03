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
  mpz_t an, bn, rn, ad, bd, rd;
  mpq_t aq, bq, refq, resq;

  mpz_init (an);
  mpz_init (bn);
  mpz_init (rn);
  mpz_init (ad);
  mpz_init (bd);
  mpz_init (rd);
  mpq_init (aq);
  mpq_init (bq);
  mpq_init (refq);
  mpq_init (resq);

  for (i = 0; i < COUNT; i++)
    {
      mini_random_op3 (OP_MUL, MAXBITS, an, bn, rn);
      do {
	mini_random_op3 (OP_MUL, MAXBITS, ad, bd, rd);
      } while (mpz_sgn (rd) == 0);

      _mpq_set_zz (aq, an, ad);
      _mpq_set_zz (bq, bn, bd);
      _mpq_set_zz (refq, rn, rd);

      mpq_mul (resq, aq, bq);
      if (!mpq_equal (resq, refq))
	{
	  fprintf (stderr, "mpq_mul failed [%i]:\n", i);
	  dump ("an", an);
	  dump ("ad", ad);
	  dump ("bn", bn);
	  dump ("bd", bd);
	  dump ("refn", rn);
	  dump ("refd", rd);
	  dump ("resn", mpq_numref (resq));
	  dump ("resd", mpq_denref (resq));
	  abort ();
	}

      if (mpq_sgn (refq) != 0)
	{
	  mpq_set_ui (resq, ~6, 8);
	  mpq_inv (aq, aq);
	  mpq_div (resq, aq, bq);
	  mpq_inv (resq, resq);
	  if (!mpq_equal (resq, refq))
	    {
	      fprintf (stderr, "mpq_div failed [%i]:\n", i);
	      dump ("an", an);
	      dump ("ad", ad);
	      dump ("bn", bn);
	      dump ("bd", bd);
	      dump ("refn", rn);
	      dump ("refd", rd);
	      dump ("resn", mpq_numref (resq));
	      dump ("resd", mpq_denref (resq));
	      abort ();
	    }

	  mpq_swap (bq, aq);
	  mpq_div (resq, aq, bq);
	  if (!mpq_equal (resq, refq))
	    {
	      fprintf (stderr, "mpq_swap failed [%i]:\n", i);
	      dump ("an", an);
	      dump ("ad", ad);
	      dump ("bn", bn);
	      dump ("bd", bd);
	      dump ("refn", rn);
	      dump ("refd", rd);
	      dump ("resn", mpq_numref (resq));
	      dump ("resd", mpq_denref (resq));
	      abort ();
	    }
	}

      mpq_set (resq, aq);
      mpq_neg (bq, aq);
      mpq_abs (refq, aq);
      if (mpq_equal (refq, resq))
	mpq_add (resq, refq, bq);
      else
	mpq_add (resq, refq, resq);
      mpq_set_ui (refq, 0, 1);
      if (!mpq_equal (resq, refq))
	{
	  fprintf (stderr, "mpq_abs failed [%i]:\n", i);
	      dump ("an", an);
	      dump ("ad", ad);
	      dump ("resn", mpq_numref (resq));
	      dump ("resd", mpq_denref (resq));
	      abort ();
	}

      mpq_mul (resq, aq, aq);
      mpq_mul (refq, aq, bq); /* now bq = - aq */
      mpq_neg (refq, refq);
      if (!mpq_equal (resq, refq))
	{
	  fprintf (stderr, "mpq_mul(sqr) failed [%i]:\n", i);
	  dump ("an", an);
	  dump ("ad", ad);
	  dump ("bn", bn);
	  dump ("bd", bd);
	  dump ("refn", rn);
	  dump ("refd", rd);
	  dump ("resn", mpq_numref (resq));
	  dump ("resd", mpq_denref (resq));
	  abort ();
	}
    }

  mpz_clear (an);
  mpz_clear (bn);
  mpz_clear (rn);
  mpz_clear (ad);
  mpz_clear (bd);
  mpz_clear (rd);
  mpq_clear (aq);
  mpq_clear (bq);
  mpq_clear (refq);
  mpq_clear (resq);
}
