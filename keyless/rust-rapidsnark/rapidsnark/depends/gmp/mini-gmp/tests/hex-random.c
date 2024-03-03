/*

Copyright 2011, 2016, 2018 Free Software Foundation, Inc.

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

#include <time.h>

#ifdef __unix__
# include <unistd.h>
# include <sys/time.h>
#endif

#include "gmp.h"

#include "hex-random.h"

/* FIXME: gmp-impl.h included only for mpz_lucas_mod */
/* #include "gmp-impl.h" */
#if defined (__cplusplus)
extern "C" {
#endif

#define mpz_lucas_mod  __gmpz_lucas_mod
__GMP_DECLSPEC int mpz_lucas_mod (mpz_ptr, mpz_ptr, long, mp_bitcnt_t, mpz_srcptr, mpz_ptr, mpz_ptr);

#if defined (__cplusplus)
}
#endif

static gmp_randstate_t state;

static void
mkseed (mpz_t seed)
{
  FILE *f = fopen ("/dev/urandom", "rb");
  if (f)
    {
      unsigned char buf[6];
      size_t res;

      setbuf (f, NULL);
      res = fread (buf, sizeof(buf), 1, f);
      fclose (f);

      if (res == 1)
	{
	  mpz_import (seed, sizeof(buf), 1, 1, 0, 0, buf);
	  return;
	}
    }

#ifdef __unix__
  {
    struct timeval tv;
    mpz_t usec;
    mpz_init (usec);

    gettimeofday (&tv, NULL);
    mpz_set_ui (seed, tv.tv_sec);
    mpz_set_ui (usec, tv.tv_usec);
    /* usec fits in 20 bits, shift left to make it 48 bits. */
    mpz_mul_2exp (usec, usec, 28);
    mpz_xor (seed, seed, usec);

    mpz_clear (usec);
  }
#else
  mpz_set_ui (seed, time (NULL));
#endif
}

void
hex_random_init (void)
{
  mpz_t seed;
  char *env_seed;

  mpz_init (seed);

  env_seed = getenv ("GMP_CHECK_RANDOMIZE");
  if (env_seed && env_seed[0])
    {
      mpz_set_str (seed, env_seed, 0);
      if (mpz_cmp_ui (seed, 0) != 0)
	gmp_printf ("Re-seeding with GMP_CHECK_RANDOMIZE=%Zd\n", seed);
      else
	{
	  mkseed (seed);
	  gmp_printf ("Seed GMP_CHECK_RANDOMIZE=%Zd (include this in bug reports)\n", seed);
	}
      fflush (stdout);
    }
  else
    mpz_set_ui (seed, 4711);

  gmp_randinit_default (state);
  gmp_randseed (state, seed);

  mpz_clear (seed);
}

char *
hex_urandomb (unsigned long bits)
{
  char *res;
  mpz_t x;

  mpz_init (x);
  mpz_urandomb (x, state, bits);
  gmp_asprintf (&res, "%Zx", x);
  mpz_clear (x);
  return res;
}

char *
hex_rrandomb (unsigned long bits)
{
  char *res;
  mpz_t x;

  mpz_init (x);
  mpz_rrandomb (x, state, bits);
  gmp_asprintf (&res, "%Zx", x);
  mpz_clear (x);
  return res;
}

char *
hex_rrandomb_export (void *dst, size_t *countp,
		     int order, size_t size, int endian, unsigned long bits)
{
  char *res;
  mpz_t x;
  mpz_init (x);
  mpz_rrandomb (x, state, bits);
  gmp_asprintf (&res, "%Zx", x);
  mpz_export (dst, countp, order, size, endian, 0, x);
  mpz_clear (x);
  return res;
}

void hex_random_op2 (enum hex_random_op op,  unsigned long maxbits,
		     char **ap, char **rp)
{
  mpz_t a, r;
  unsigned long abits;
  unsigned signs;

  mpz_init (a);
  mpz_init (r);

  abits = gmp_urandomb_ui (state, 32) % maxbits;

  mpz_rrandomb (a, state, abits);

  signs = gmp_urandomb_ui (state, 1);
  if (signs & 1)
    mpz_neg (a, a);

  switch (op)
    {
    default:
      abort ();
    case OP_SQR:
      mpz_mul (r, a, a);
      break;
    }

  gmp_asprintf (ap, "%Zx", a);
  gmp_asprintf (rp, "%Zx", r);

  mpz_clear (a);
  mpz_clear (r);
}

void
hex_random_op3 (enum hex_random_op op,  unsigned long maxbits,
		char **ap, char **bp, char **rp)
{
  mpz_t a, b, r;
  unsigned long abits, bbits;
  unsigned signs;

  mpz_init (a);
  mpz_init (b);
  mpz_init (r);

  abits = gmp_urandomb_ui (state, 32) % maxbits;
  bbits = gmp_urandomb_ui (state, 32) % maxbits;

  mpz_rrandomb (a, state, abits);
  mpz_rrandomb (b, state, bbits);

  signs = gmp_urandomb_ui (state, 3);
  if (signs & 1)
    mpz_neg (a, a);
  if (signs & 2)
    mpz_neg (b, b);

  switch (op)
    {
    default:
      abort ();
    case OP_ADD:
      mpz_add (r, a, b);
      break;
    case OP_SUB:
      mpz_sub (r, a, b);
      break;
    case OP_MUL:
      mpz_mul (r, a, b);
      break;
    case OP_GCD:
      if (signs & 4)
	{
	  /* Produce a large gcd */
	  unsigned long gbits = gmp_urandomb_ui (state, 32) % maxbits;
	  mpz_rrandomb (r, state, gbits);
	  mpz_mul (a, a, r);
	  mpz_mul (b, b, r);
	}
      mpz_gcd (r, a, b);
      break;
    case OP_LCM:
      if (signs & 4)
	{
	  /* Produce a large gcd */
	  unsigned long gbits = gmp_urandomb_ui (state, 32) % maxbits;
	  mpz_rrandomb (r, state, gbits);
	  mpz_mul (a, a, r);
	  mpz_mul (b, b, r);
	}
      mpz_lcm (r, a, b);
      break;
    case OP_AND:
      mpz_and (r, a, b);
      break;
    case OP_IOR:
      mpz_ior (r, a, b);
      break;
    case OP_XOR:
      mpz_xor (r, a, b);
      break;
    }

  gmp_asprintf (ap, "%Zx", a);
  gmp_asprintf (bp, "%Zx", b);
  gmp_asprintf (rp, "%Zx", r);

  mpz_clear (a);
  mpz_clear (b);
  mpz_clear (r);
}

void
hex_random_op4 (enum hex_random_op op, unsigned long maxbits,
		char **ap, char **bp, char **cp, char **dp)
{
  mpz_t a, b, c, d;
  unsigned long abits, bbits;
  unsigned signs;

  mpz_init (a);
  mpz_init (b);
  mpz_init (c);
  mpz_init (d);

  if (op == OP_POWM)
    {
      unsigned long cbits;
      abits = gmp_urandomb_ui (state, 32) % maxbits;
      bbits = 1 + gmp_urandomb_ui (state, 32) % maxbits;
      cbits = 2 + gmp_urandomb_ui (state, 32) % maxbits;

      mpz_rrandomb (a, state, abits);
      mpz_rrandomb (b, state, bbits);
      mpz_rrandomb (c, state, cbits);

      signs = gmp_urandomb_ui (state, 3);
      if (signs & 1)
	mpz_neg (a, a);
      if (signs & 2)
	{
	  mpz_t g;

	  /* If we negate the exponent, must make sure that gcd(a, c) = 1 */
	  if (mpz_sgn (a) == 0)
	    mpz_set_ui (a, 1);
	  else
	    {
	      mpz_init (g);

	      for (;;)
		{
		  mpz_gcd (g, a, c);
		  if (mpz_cmp_ui (g, 1) == 0)
		    break;
		  mpz_divexact (a, a, g);
		}
	      mpz_clear (g);
	    }
	  mpz_neg (b, b);
	}
      if (signs & 4)
	mpz_neg (c, c);

      mpz_powm (d, a, b, c);
    }
  else
    {
      unsigned long qbits;
      bbits = 1 + gmp_urandomb_ui (state, 32) % maxbits;
      qbits = gmp_urandomb_ui (state, 32) % maxbits;
      abits = bbits + qbits;
      if (abits > 30)
	abits -= 30;
      else
	abits = 0;

      mpz_rrandomb (a, state, abits);
      mpz_rrandomb (b, state, bbits);

      signs = gmp_urandomb_ui (state, 2);
      if (signs & 1)
	mpz_neg (a, a);
      if (signs & 2)
	mpz_neg (b, b);

      switch (op)
	{
	default:
	  abort ();
	case OP_CDIV:
	  mpz_cdiv_qr (c, d, a, b);
	  break;
	case OP_FDIV:
	  mpz_fdiv_qr (c, d, a, b);
	  break;
	case OP_TDIV:
	  mpz_tdiv_qr (c, d, a, b);
	  break;
	}
    }
  gmp_asprintf (ap, "%Zx", a);
  gmp_asprintf (bp, "%Zx", b);
  gmp_asprintf (cp, "%Zx", c);
  gmp_asprintf (dp, "%Zx", d);

  mpz_clear (a);
  mpz_clear (b);
  mpz_clear (c);
  mpz_clear (d);
}

void
hex_random_bit_op (enum hex_random_op op, unsigned long maxbits,
		   char **ap, unsigned long *b, char **rp)
{
  mpz_t a, r;
  unsigned long abits, bbits;
  unsigned signs;

  mpz_init (a);
  mpz_init (r);

  abits = gmp_urandomb_ui (state, 32) % maxbits;
  bbits = gmp_urandomb_ui (state, 32) % (maxbits + 100);

  mpz_rrandomb (a, state, abits);

  signs = gmp_urandomb_ui (state, 1);
  if (signs & 1)
    mpz_neg (a, a);

  switch (op)
    {
    default:
      abort ();

    case OP_SETBIT:
      mpz_set (r, a);
      mpz_setbit (r, bbits);
      break;
    case OP_CLRBIT:
      mpz_set (r, a);
      mpz_clrbit (r, bbits);
      break;
    case OP_COMBIT:
      mpz_set (r, a);
      mpz_combit (r, bbits);
      break;
    case OP_CDIV_Q_2:
      mpz_cdiv_q_2exp (r, a, bbits);
      break;
    case OP_CDIV_R_2:
      mpz_cdiv_r_2exp (r, a, bbits);
      break;
    case OP_FDIV_Q_2:
      mpz_fdiv_q_2exp (r, a, bbits);
      break;
    case OP_FDIV_R_2:
      mpz_fdiv_r_2exp (r, a, bbits);
      break;
    case OP_TDIV_Q_2:
      mpz_tdiv_q_2exp (r, a, bbits);
      break;
    case OP_TDIV_R_2:
      mpz_tdiv_r_2exp (r, a, bbits);
      break;
    }

  gmp_asprintf (ap, "%Zx", a);
  *b = bbits;
  gmp_asprintf (rp, "%Zx", r);

  mpz_clear (a);
  mpz_clear (r);
}

void
hex_random_scan_op (enum hex_random_op op, unsigned long maxbits,
		    char **ap, unsigned long *b, unsigned long *r)
{
  mpz_t a;
  unsigned long abits, bbits;
  unsigned signs;

  mpz_init (a);

  abits = gmp_urandomb_ui (state, 32) % maxbits;
  bbits = gmp_urandomb_ui (state, 32) % (maxbits + 100);

  mpz_rrandomb (a, state, abits);

  signs = gmp_urandomb_ui (state, 1);
  if (signs & 1)
    mpz_neg (a, a);

  switch (op)
    {
    default:
      abort ();

    case OP_SCAN0:
      *r = mpz_scan0 (a, bbits);
      break;
    case OP_SCAN1:
      *r = mpz_scan1 (a, bbits);
      break;
    }
  gmp_asprintf (ap, "%Zx", a);
  *b = bbits;

  mpz_clear (a);
}

void
hex_random_str_op (unsigned long maxbits,
		   int base, char **ap, char **rp)
{
  mpz_t a;
  unsigned long abits;
  unsigned signs;

  mpz_init (a);

  abits = gmp_urandomb_ui (state, 32) % maxbits;

  mpz_rrandomb (a, state, abits);

  signs = gmp_urandomb_ui (state, 2);
  if (signs & 1)
    mpz_neg (a, a);

  *ap = mpz_get_str (NULL, 16, a);
  *rp = mpz_get_str (NULL, base, a);

  mpz_clear (a);
}

void hex_random_lucm_op (unsigned long maxbits,
			 char **vp, char **qp, char **mp,
			 long *Q, unsigned long *b0, int *res)
{
  mpz_t m, v, q, t1, t2;
  unsigned long mbits;

  mpz_init (m);
  mpz_init (v);
  mpz_init (q);
  mpz_init (t1);
  mpz_init (t2);

  *Q = gmp_urandomb_ui (state, 14) + 1;

  do
    {
      mbits = gmp_urandomb_ui (state, 32) % maxbits + 5;

      mpz_rrandomb (m, state, mbits);
      *b0 = gmp_urandomb_ui (state, 32) % (mbits - 3) + 2;
      /* The GMP  implementation uses the exponent (m >> b0) + 1. */
      /* mini-gmp implementation uses the exponent (m >> b0) | 1. */
      /* They are the same (and are used) only when (m >> b0) is even */
      mpz_clrbit (m, *b0);
      /* mini-gmp implementation only works if the modulus is odd. */
      mpz_setbit (m, 0);
    }
  while (mpz_gcd_ui (NULL, m, *Q) != 1);

  if (*Q == 1 || gmp_urandomb_ui (state, 1))
    *Q = - *Q;

#if (__GNU_MP_VERSION == 6 && (__GNU_MP_VERSION_MINOR > 1 || __GNU_MP_VERSION_PATCHLEVEL > 9))
  *res = mpz_lucas_mod (v, q, *Q, *b0, m, t1, t2);
#else
  *b0 = 0;
#endif

  gmp_asprintf (vp, "%Zx", v);
  gmp_asprintf (qp, "%Zx", q);
  gmp_asprintf (mp, "%Zx", m);

  mpz_clear (m);
  mpz_clear (v);
  mpz_clear (q);
  mpz_clear (t1);
  mpz_clear (t2);
}

void
hex_mpq_random_str_op (unsigned long maxbits,
		       int base, char **ap, char **rp)
{
  mpq_t a;
  unsigned long abits;
  unsigned signs;

  mpq_init (a);

  abits = gmp_urandomb_ui (state, 32) % maxbits;

  mpz_rrandomb (mpq_numref (a), state, abits);
  mpz_rrandomb (mpq_denref (a), state, abits);
  mpz_add_ui (mpq_denref (a), mpq_denref (a), 1);

  mpq_canonicalize (a);
  signs = gmp_urandomb_ui (state, 2);
  if (signs & 1)
    mpq_neg (a, a);

  *ap = mpq_get_str (NULL, 16, a);
  *rp = mpq_get_str (NULL, base, a);

  mpq_clear (a);
}
