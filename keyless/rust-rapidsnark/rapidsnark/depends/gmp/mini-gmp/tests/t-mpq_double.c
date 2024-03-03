/* Test mpq_set_d.

Copyright 2001-2003, 2005, 2013, 2018 Free Software Foundation, Inc.

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

#include <math.h>
#include <float.h>
#include <limits.h>

#include "testutils.h"
#include "../mini-mpq.h"

#define COUNT 2000

mp_bitcnt_t
mpz_mantissasizeinbits (const mpz_t z)
{
  return ! mpz_cmp_ui (z, 0) ? 0 :
    mpz_sizeinbase (z, 2) - mpz_scan1 (z, 0);
}

int
mpz_abspow2_p (const mpz_t z)
{
  return mpz_mantissasizeinbits (z) == 1;
}

mp_bitcnt_t
mpq_mantissasizeinbits (const mpq_t q)
{
  if (! mpz_abspow2_p (mpq_denref (q)))
    return ~ (mp_bitcnt_t) 0;

  return mpz_mantissasizeinbits (mpq_numref (q));
}

#if defined(DBL_MANT_DIG) && FLT_RADIX == 2
int
mpz_get_d_exact_p (const mpz_t z)
{
  return mpz_mantissasizeinbits (z) <= DBL_MANT_DIG;
}

int
mpq_get_d_exact_p (const mpq_t q)
{
  return mpq_mantissasizeinbits (q) <= DBL_MANT_DIG;
}
#define HAVE_EXACT_P 1
#endif

void
check_random (void)
{
  unsigned i;
  mpz_t x;
  mpq_t y, z;

  mpz_init (x);
  mpq_init (y);
  mpq_init (z);

  for (i = 0; i < COUNT; i++)
    {
      /* Use volatile, to avoid extended precision in floating point
	 registers, e.g., on m68k and 80387. */
      volatile double d, f;
      unsigned long m;
      int e, c;

      mini_rrandomb (x, CHAR_BIT * sizeof (unsigned long));
      m = mpz_get_ui (x);
      mini_urandomb (x, 8);
      e = mpz_get_ui (x) - 128;

      d = ldexp ((double) m, e);
      mpq_set_d (y, d);
      f = mpq_get_d (y);
      if (f != d)
	{
	  fprintf (stderr, "mpq_set_d/mpq_get_d failed:\n");
	  goto dumperror;
	}

      d = - d;
      mpq_neg (y, y);

      mpq_set_d (z, d);
      f = mpq_get_d (z);
      if (f != d || !mpq_equal (y, z))
	{
	  fprintf (stderr, "mpq_set_d/mpq_get_d failed:\n");
	dumperror:
	  dump ("ny", mpq_numref (y));
	  dump ("dy", mpq_denref (y));
	  fprintf (stderr, "m = %lx, e = %i\n", m, e);
	  fprintf (stderr, "d = %.35g\n", d);
	  fprintf (stderr, "f = %.35g\n", f);
	  fprintf (stderr, "f - d = %.35g\n", f - d);
	  abort ();
	}

      mini_rrandomb (x, CHAR_BIT * sizeof (unsigned long));
      m = mpz_get_ui (x);
      mini_urandomb (x, 8);
      e = mpz_get_ui (x) - 128;

      d = ldexp ((double) m, e);
      mpq_set_d (y, d);

      if (i == 0)
	mpq_neg (z, y);

      mpq_add (y, y, z);
      mpq_set_d (z, mpq_get_d (y));
      f = mpq_get_d (z);
      c = mpq_cmp (y, z);

#if defined(HAVE_EXACT_P)
      if (mpq_get_d_exact_p (y) ? c != 0 : (f > 0 ? c <= 0 : c >= 0))
#else
      if (f > 0 ? c < 0 : c > 0)
#endif
	{
	  fprintf (stderr, "mpq_get_d/mpq_set_d failed: %i %i\n", i, c);
	  goto dumperror;
	}
    }

  mpz_clear (x);
  mpq_clear (y);
  mpq_clear (z);
}


void
check_data (void)
{
  static const struct {
    double        y;
    long int      n;
    unsigned long d;
  } data[] = {
    {  0.0,  0, 1 },
    {  1.0,  1, 1 },
    { -1.0, -1, 1 },
    { -1.5, -3, 2 },
    {-1.25, -5, 4 },
    {0.125,  1, 8 },

    {24685,24685,1},
    {-9876,-9876,1},
    {463.5,  927,2},

    {1234.5/8192,  2469, 16384 },
    {-543.0/1024,  -543,  1024 },
    {9876.5/ 512, 19753,  1024 },
    {9753.0/ 128,  9753,   128 },
    {-789.0/  32,  -789,    32 },
    {4.580078125,  2345,   512 },
  };

  mpq_t    x, r;
  unsigned i;
  double d;

  mpq_init (x);
  mpq_init (r);

  for (i = 0; i < numberof (data); i++)
    {
      mpq_set_d (x, data[i].y);
      mpq_set_si (r, data[i].n, data[i].d);
      mpq_canonicalize (r);
      if (!mpq_equal (x, r))
	{
	  fprintf (stderr, "mpq_set_d failed: %li / %lu != %g\n", data[i].n, data[i].d, data[i].y);
	  abort ();
	}
      d = mpq_get_d (r);
      if (d != data[i].y)
	{
	  fprintf (stderr, "mpq_get_d failed: %li / %lu != %g\n", data[i].n, data[i].d, data[i].y);
	  abort ();
	}
    }

  mpq_clear (x);
  mpq_clear (r);
}

void
testmain (int argc, char *argv[])
{
  check_data ();
  check_random ();
}
