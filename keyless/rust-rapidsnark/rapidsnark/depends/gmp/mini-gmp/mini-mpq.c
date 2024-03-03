/* mini-mpq, a minimalistic implementation of a GNU GMP subset.

   Contributed to the GNU project by Marco Bodrato

   Acknowledgment: special thanks to Bradley Lucier for his comments
   to the preliminary version of this code.

Copyright 2018-2020 Free Software Foundation, Inc.

This file is part of the GNU MP Library.

The GNU MP Library is free software; you can redistribute it and/or modify
it under the terms of either:

  * the GNU Lesser General Public License as published by the Free
    Software Foundation; either version 3 of the License, or (at your
    option) any later version.

or

  * the GNU General Public License as published by the Free Software
    Foundation; either version 2 of the License, or (at your option) any
    later version.

or both in parallel, as here.

The GNU MP Library is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

You should have received copies of the GNU General Public License and the
GNU Lesser General Public License along with the GNU MP Library.  If not,
see https://www.gnu.org/licenses/.  */

#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "mini-mpq.h"

#ifndef GMP_LIMB_HIGHBIT
/* Define macros and static functions already defined by mini-gmp.c */
#define GMP_LIMB_BITS (sizeof(mp_limb_t) * CHAR_BIT)
#define GMP_LIMB_HIGHBIT ((mp_limb_t) 1 << (GMP_LIMB_BITS - 1))
#define GMP_NEG_CAST(T,x) (-((T)((x) + 1) - 1))
#define GMP_MIN(a, b) ((a) < (b) ? (a) : (b))

static mpz_srcptr
mpz_roinit_normal_n (mpz_t x, mp_srcptr xp, mp_size_t xs)
{
  x->_mp_alloc = 0;
  x->_mp_d = (mp_ptr) xp;
  x->_mp_size = xs;
  return x;
}

static void
gmp_die (const char *msg)
{
  fprintf (stderr, "%s\n", msg);
  abort();
}
#endif


/* MPQ helper functions */
static mpq_srcptr
mpq_roinit_normal_nn (mpq_t x, mp_srcptr np, mp_size_t ns,
		     mp_srcptr dp, mp_size_t ds)
{
  mpz_roinit_normal_n (mpq_numref(x), np, ns);
  mpz_roinit_normal_n (mpq_denref(x), dp, ds);
  return x;
}

static mpq_srcptr
mpq_roinit_zz (mpq_t x, mpz_srcptr n, mpz_srcptr d)
{
  return mpq_roinit_normal_nn (x, n->_mp_d, n->_mp_size,
			       d->_mp_d, d->_mp_size);
}

static void
mpq_nan_init (mpq_t x)
{
  mpz_init (mpq_numref (x));
  mpz_init (mpq_denref (x));
}

void
mpq_init (mpq_t x)
{
  mpz_init (mpq_numref (x));
  mpz_init_set_ui (mpq_denref (x), 1);
}

void
mpq_clear (mpq_t x)
{
  mpz_clear (mpq_numref (x));
  mpz_clear (mpq_denref (x));
}

static void
mpq_canonical_sign (mpq_t r)
{
  mp_size_t ds = mpq_denref (r)->_mp_size;
  if (ds <= 0)
    {
      if (ds == 0)
	gmp_die("mpq: Fraction with zero denominator.");
      mpz_neg (mpq_denref (r), mpq_denref (r));
      mpz_neg (mpq_numref (r), mpq_numref (r));
    }
}

static void
mpq_helper_canonicalize (mpq_t r, const mpz_t num, const mpz_t den, mpz_t g)
{
  if (num->_mp_size == 0)
    mpq_set_ui (r, 0, 1);
  else
    {
      mpz_gcd (g, num, den);
      mpz_tdiv_q (mpq_numref (r), num, g);
      mpz_tdiv_q (mpq_denref (r), den, g);
      mpq_canonical_sign (r);
    }
}

void
mpq_canonicalize (mpq_t r)
{
  mpz_t t;

  mpz_init (t);
  mpq_helper_canonicalize (r, mpq_numref (r), mpq_denref (r), t);
  mpz_clear (t);
}

void
mpq_swap (mpq_t a, mpq_t b)
{
  mpz_swap (mpq_numref (a), mpq_numref (b));
  mpz_swap (mpq_denref (a), mpq_denref (b));
}


/* MPQ assignment and conversions. */
void
mpz_set_q (mpz_t r, const mpq_t q)
{
  mpz_tdiv_q (r, mpq_numref (q), mpq_denref (q));
}

void
mpq_set (mpq_t r, const mpq_t q)
{
  mpz_set (mpq_numref (r), mpq_numref (q));
  mpz_set (mpq_denref (r), mpq_denref (q));
}

void
mpq_set_ui (mpq_t r, unsigned long n, unsigned long d)
{
  mpz_set_ui (mpq_numref (r), n);
  mpz_set_ui (mpq_denref (r), d);
}

void
mpq_set_si (mpq_t r, signed long n, unsigned long d)
{
  mpz_set_si (mpq_numref (r), n);
  mpz_set_ui (mpq_denref (r), d);
}

void
mpq_set_z (mpq_t r, const mpz_t n)
{
  mpz_set_ui (mpq_denref (r), 1);
  mpz_set (mpq_numref (r), n);
}

void
mpq_set_num (mpq_t r, const mpz_t z)
{
  mpz_set (mpq_numref (r), z);
}

void
mpq_set_den (mpq_t r, const mpz_t z)
{
  mpz_set (mpq_denref (r), z);
}

void
mpq_get_num (mpz_t r, const mpq_t q)
{
  mpz_set (r, mpq_numref (q));
}

void
mpq_get_den (mpz_t r, const mpq_t q)
{
  mpz_set (r, mpq_denref (q));
}


/* MPQ comparisons and the like. */
int
mpq_cmp (const mpq_t a, const mpq_t b)
{
  mpz_t t1, t2;
  int res;

  mpz_init (t1);
  mpz_init (t2);
  mpz_mul (t1, mpq_numref (a), mpq_denref (b));
  mpz_mul (t2, mpq_numref (b), mpq_denref (a));
  res = mpz_cmp (t1, t2);
  mpz_clear (t1);
  mpz_clear (t2);

  return res;
}

int
mpq_cmp_z (const mpq_t a, const mpz_t b)
{
  mpz_t t;
  int res;

  mpz_init (t);
  mpz_mul (t, b, mpq_denref (a));
  res = mpz_cmp (mpq_numref (a), t);
  mpz_clear (t);

  return res;
}

int
mpq_equal (const mpq_t a, const mpq_t b)
{
  return (mpz_cmp (mpq_numref (a), mpq_numref (b)) == 0) &&
    (mpz_cmp (mpq_denref (a), mpq_denref (b)) == 0);
}

int
mpq_cmp_ui (const mpq_t q, unsigned long n, unsigned long d)
{
  mpq_t t;
  assert (d != 0);
  if (ULONG_MAX <= GMP_LIMB_MAX) {
    mp_limb_t nl = n, dl = d;
    return mpq_cmp (q, mpq_roinit_normal_nn (t, &nl, n != 0, &dl, 1));
  } else {
    int ret;

    mpq_init (t);
    mpq_set_ui (t, n, d);
    ret = mpq_cmp (q, t);
    mpq_clear (t);

    return ret;
  }
}

int
mpq_cmp_si (const mpq_t q, signed long n, unsigned long d)
{
  assert (d != 0);

  if (n >= 0)
    return mpq_cmp_ui (q, n, d);
  else
    {
      mpq_t t;

      if (ULONG_MAX <= GMP_LIMB_MAX)
	{
	  mp_limb_t nl = GMP_NEG_CAST (unsigned long, n), dl = d;
	  return mpq_cmp (q, mpq_roinit_normal_nn (t, &nl, -1, &dl, 1));
	}
      else
	{
	  unsigned long l_n = GMP_NEG_CAST (unsigned long, n);

	  mpq_roinit_normal_nn (t, mpq_numref (q)->_mp_d, - mpq_numref (q)->_mp_size,
				mpq_denref (q)->_mp_d, mpq_denref (q)->_mp_size);
	  return - mpq_cmp_ui (t, l_n, d);
	}
    }
}

int
mpq_sgn (const mpq_t a)
{
  return mpz_sgn (mpq_numref (a));
}


/* MPQ arithmetic. */
void
mpq_abs (mpq_t r, const mpq_t q)
{
  mpz_abs (mpq_numref (r), mpq_numref (q));
  mpz_set (mpq_denref (r), mpq_denref (q));
}

void
mpq_neg (mpq_t r, const mpq_t q)
{
  mpz_neg (mpq_numref (r), mpq_numref (q));
  mpz_set (mpq_denref (r), mpq_denref (q));
}

void
mpq_add (mpq_t r, const mpq_t a, const mpq_t b)
{
  mpz_t t;

  mpz_init (t);
  mpz_gcd (t, mpq_denref (a), mpq_denref (b));
  if (mpz_cmp_ui (t, 1) == 0)
    {
      mpz_mul (t, mpq_numref (a), mpq_denref (b));
      mpz_addmul (t, mpq_numref (b), mpq_denref (a));
      mpz_mul (mpq_denref (r), mpq_denref (a), mpq_denref (b));
      mpz_swap (mpq_numref (r), t);
    }
  else
    {
      mpz_t x, y;
      mpz_init (x);
      mpz_init (y);

      mpz_tdiv_q (x, mpq_denref (b), t);
      mpz_tdiv_q (y, mpq_denref (a), t);
      mpz_mul (x, mpq_numref (a), x);
      mpz_addmul (x, mpq_numref (b), y);

      mpz_gcd (t, x, t);
      mpz_tdiv_q (mpq_numref (r), x, t);
      mpz_tdiv_q (x, mpq_denref (b), t);
      mpz_mul (mpq_denref (r), x, y);

      mpz_clear (x);
      mpz_clear (y);
    }
  mpz_clear (t);
}

void
mpq_sub (mpq_t r, const mpq_t a, const mpq_t b)
{
  mpq_t t;

  mpq_roinit_normal_nn (t, mpq_numref (b)->_mp_d, - mpq_numref (b)->_mp_size,
			mpq_denref (b)->_mp_d, mpq_denref (b)->_mp_size);
  mpq_add (r, a, t);
}

void
mpq_div (mpq_t r, const mpq_t a, const mpq_t b)
{
  mpq_t t;
  mpq_mul (r, a, mpq_roinit_zz (t, mpq_denref (b), mpq_numref (b)));
}

void
mpq_mul (mpq_t r, const mpq_t a, const mpq_t b)
{
  mpq_t t;
  mpq_nan_init (t);

  if (a != b) {
    mpz_t g;

    mpz_init (g);
    mpq_helper_canonicalize (t, mpq_numref (a), mpq_denref (b), g);
    mpq_helper_canonicalize (r, mpq_numref (b), mpq_denref (a), g);
    mpz_clear (g);

    a = r;
    b = t;
  }

  mpz_mul (mpq_numref (r), mpq_numref (a), mpq_numref (b));
  mpz_mul (mpq_denref (r), mpq_denref (a), mpq_denref (b));
  mpq_clear (t);
}

void
mpq_div_2exp (mpq_t r, const mpq_t q, mp_bitcnt_t e)
{
  mp_bitcnt_t z = mpz_scan1 (mpq_numref (q), 0);
  z = GMP_MIN (z, e);
  mpz_mul_2exp (mpq_denref (r), mpq_denref (q), e - z);
  mpz_tdiv_q_2exp (mpq_numref (r), mpq_numref (q), z);
}

void
mpq_mul_2exp (mpq_t r, const mpq_t q, mp_bitcnt_t e)
{
  mp_bitcnt_t z = mpz_scan1 (mpq_denref (q), 0);
  z = GMP_MIN (z, e);
  mpz_mul_2exp (mpq_numref (r), mpq_numref (q), e - z);
  mpz_tdiv_q_2exp (mpq_denref (r), mpq_denref (q), z);
}

void
mpq_inv (mpq_t r, const mpq_t q)
{
  mpq_set (r, q);
  mpz_swap (mpq_denref (r), mpq_numref (r));
  mpq_canonical_sign (r);
}


/* MPQ to/from double. */
void
mpq_set_d (mpq_t r, double x)
{
  mpz_set_ui (mpq_denref (r), 1);

  /* x != x is true when x is a NaN, and x == x * 0.5 is true when x is
     zero or infinity. */
  if (x == x * 0.5 || x != x)
    mpq_numref (r)->_mp_size = 0;
  else
    {
      double B;
      mp_bitcnt_t e;

      B = 4.0 * (double) (GMP_LIMB_HIGHBIT >> 1);
      for (e = 0; x != x + 0.5; e += GMP_LIMB_BITS)
	x *= B;

      mpz_set_d (mpq_numref (r), x);
      mpq_div_2exp (r, r, e);
    }
}

double
mpq_get_d (const mpq_t u)
{
  mp_bitcnt_t ne, de, ee;
  mpz_t z;
  double B, ret;

  ne = mpz_sizeinbase (mpq_numref (u), 2);
  de = mpz_sizeinbase (mpq_denref (u), 2);

  ee = CHAR_BIT * sizeof (double);
  if (de == 1 || ne > de + ee)
    ee = 0;
  else
    ee = (ee + de - ne) / GMP_LIMB_BITS + 1;

  mpz_init (z);
  mpz_mul_2exp (z, mpq_numref (u), ee * GMP_LIMB_BITS);
  mpz_tdiv_q (z, z, mpq_denref (u));
  ret = mpz_get_d (z);
  mpz_clear (z);

  B = 4.0 * (double) (GMP_LIMB_HIGHBIT >> 1);
  for (B = 1 / B; ee != 0; --ee)
    ret *= B;

  return ret;
}


/* MPQ and strings/streams. */
char *
mpq_get_str (char *sp, int base, const mpq_t q)
{
  char *res;
  char *rden;
  size_t len;

  res = mpz_get_str (sp, base, mpq_numref (q));
  if (res == NULL || mpz_cmp_ui (mpq_denref (q), 1) == 0)
    return res;

  len = strlen (res) + 1;
  rden = sp ? sp + len : NULL;
  rden = mpz_get_str (rden, base, mpq_denref (q));
  assert (rden != NULL);

  if (sp == NULL) {
    void * (*gmp_reallocate_func) (void *, size_t, size_t);
    void (*gmp_free_func) (void *, size_t);
    size_t lden;

    mp_get_memory_functions (NULL, &gmp_reallocate_func, &gmp_free_func);
    lden = strlen (rden) + 1;
    res = (char *) gmp_reallocate_func (res, 0, (lden + len) * sizeof (char));
    memcpy (res + len, rden, lden);
    gmp_free_func (rden, 0);
  }

  res [len - 1] = '/';
  return res;
}

size_t
mpq_out_str (FILE *stream, int base, const mpq_t x)
{
  char * str;
  size_t len;
  void (*gmp_free_func) (void *, size_t);

  str = mpq_get_str (NULL, base, x);
  if (!str)
    return 0;
  len = strlen (str);
  len = fwrite (str, 1, len, stream);
  mp_get_memory_functions (NULL, NULL, &gmp_free_func);
  gmp_free_func (str, 0);
  return len;
}

int
mpq_set_str (mpq_t r, const char *sp, int base)
{
  const char *slash;

  slash = strchr (sp, '/');
  if (slash == NULL) {
    mpz_set_ui (mpq_denref(r), 1);
    return mpz_set_str (mpq_numref(r), sp, base);
  } else {
    char *num;
    size_t numlen;
    int ret;
    void * (*gmp_allocate_func) (size_t);
    void (*gmp_free_func) (void *, size_t);

    mp_get_memory_functions (&gmp_allocate_func, NULL, &gmp_free_func);
    numlen = slash - sp;
    num = (char *) gmp_allocate_func ((numlen + 1) * sizeof (char));
    memcpy (num, sp, numlen);
    num[numlen] = '\0';
    ret = mpz_set_str (mpq_numref(r), num, base);
    gmp_free_func (num, 0);

    if (ret != 0)
      return ret;

    return mpz_set_str (mpq_denref(r), slash + 1, base);
  }
}
