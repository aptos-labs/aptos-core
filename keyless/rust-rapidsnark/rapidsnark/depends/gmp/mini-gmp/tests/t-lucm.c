/* Tests the (internal) function mpz_lucas_mod

Copyright 2018, Free Software Foundation, Inc.

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
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "testutils.h"

#define MAXBITS 100
#define COUNT 1000

void
testmain (int argc, char **argv)
{
  unsigned i;
  mpz_t m, vr, qr, vm, qm, vt;
  int resm, resr;
  long Q;
  unsigned long b0;

  mpz_init (m);
  mpz_init (vr);
  mpz_init (qr);
  mpz_init (vm);
  mpz_init (qm);
  mpz_init (vt);

  for (i = 0; i < COUNT; i++)
    {
      mini_random_lucm_op (MAXBITS, vr, qr, m, &Q, &b0, &resr);
      if (b0 == 0)
	{
	  fprintf (stderr, "lucas_mod: test disabled (%u tests done).\n", i);
	  break;
	}
      resm = mpz_lucas_mod (vm, qm, Q, b0, m);

      if (resr != resm)
	{
	  if (resm != 0 || mpz_cmp_ui (vm, 0) != 0)
	    {
	      fprintf (stderr, "mpz_lucas_mod wrong return value (%d != %d):\n", resr, resm);
	      fprintf (stderr, "Q = %ld , b0 = %lu\n", Q, b0);
	      dump ("m", m);
	      dump ("vm", vm);
	      dump ("qm", qm);
	      abort ();
	    }
	}
      else if (resm == 0)
	{
	  mpz_abs (vr, vr);
	  mpz_sub (vt, m, vr);
	  mpz_abs (vm, vm);
	  mpz_mod (qm, qm, m);
	  if (mpz_cmp_ui (qr, 0) < 0)
	    mpz_add (qr, qr, m);
	  if (mpz_cmp (qm, qr) != 0 ||
	      (mpz_cmp (vm, vr) != 0 && mpz_cmp (vm, vt) != 0))
	    {
	      fprintf (stderr, "mpz_lucas_mod error:\n");
	      fprintf (stderr, "Q = %ld , b0 = %lu\n", Q, b0);
	      dump ("m", m);
	      dump ("vm", vm);
	      dump ("vr", vr);
	      dump ("vt", vt);
	      dump ("qm", qm);
	      dump ("qr", qr);
	      abort ();
	    }

	}
    }
  mpz_clear (m);
  mpz_clear (vr);
  mpz_clear (qr);
  mpz_clear (vm);
  mpz_clear (qm);
  mpz_clear (vt);
}
