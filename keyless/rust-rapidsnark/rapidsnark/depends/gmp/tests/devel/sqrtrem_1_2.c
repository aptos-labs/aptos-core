/*
Copyright 2017 Free Software Foundation, Inc.

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

/* Usage:

   ./sqrtrem_1_2 x

     Checks mpn_sqrtrem() exhaustively, starting from 0, incrementing
     the operand by a single unit, until all values handled by
     mpn_sqrtrem{1,2} are tested. SLOW.

   ./sqrtrem_1_2 s 1

     Checks some special cases for mpn_sqrtrem(). I.e. values of the form
     2^k*i and 2^k*(i+1)-1, with k=2^n and 0<i<2^k, until all such values,
     handled by mpn_sqrtrem{1,2}, are tested.
     Currently supports only the test of values that fits in one limb.
     Less slow than the exhaustive test.

   ./sqrtrem_1_2 c

     Checks all corner cases for mpn_sqrtrem(). I.e. values of the form
     i*i and (i+1)*(i+1)-1, for each value of i, until all such values,
     handled by mpn_sqrtrem{1,2}, are tested.
     Slightly faster than the special cases test.

   For larger values, use
   ./try mpn_sqrtrem

 */

#include <stdlib.h>
#include <stdio.h>
#include "gmp-impl.h"
#include "longlong.h"
#include "tests.h"
#define STOP(x) return (x)
/* #define STOP(x) x */
#define SPINNER(v)					\
  do {							\
    MPN_SIZEINBASE_2EXP (spinner_count, q, v, 1);	\
    --spinner_count;					\
    spinner();						\
  } while (0)

int something_wrong (mp_limb_t er, mp_limb_t ec, mp_limb_t es)
{
  fprintf (stderr, "root = %lu , rem = {%lu , %lu}\n", (long unsigned) es,(long unsigned) ec,(long unsigned) er);
  return -1;
}

int
check_all_values (int justone, int quick)
{
  mp_limb_t es, mer, er, s[1], r[2], q[2];
  mp_size_t x;
  unsigned bits;

  es=1;
  if (quick) {
    printf ("Quick, skipping some... (%u)\n", GMP_NUMB_BITS - 2);
    es <<= GMP_NUMB_BITS / 2 - 1;
  }
  er=0;
  mer= es << 1;
  *q = es * es;
  printf ("All values tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 1);
    if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 1) && (er != *r)))
      STOP (something_wrong (er, 0, es));

    if (UNLIKELY (er == mer)) {
      ++es;
      if (UNLIKELY ((es & 0xff) == 0))
	SPINNER(1);
      mer +=2; /* mer = es * 2 */
      er = 0;
    } else
      ++er;
    ++*q;
  } while (*q != 0);
  q[1] = 1;
  SPINNER(2);
  printf ("\nValues of a single limb, tested.\n");
  if (justone) return 0;
  printf ("All values tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 2);
    if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 1) && (er != *r)))
      STOP (something_wrong (er, 0, es));

    if (UNLIKELY (er == mer)) {
      ++es;
      if (UNLIKELY ((es & 0x7f) == 0))
	SPINNER(2);
      mer +=2; /* mer = es * 2 */
      if (UNLIKELY (mer == 0))
	break;
      er = 0;
    } else
      ++er;
    q[1] += (++*q == 0);
  } while (1);
  SPINNER(2);
  printf ("\nValues with at most a limb for reminder, tested.\n");
  printf ("Testing more values not supported, jet.\n");
  return 0;
}

mp_limb_t
upd (mp_limb_t *s, mp_limb_t k)
{
  mp_limb_t _s = *s;

  while (k > _s * 2)
    {
      k -= _s * 2 + 1;
      ++_s;
    }
  *s = _s;
  return k;
}

mp_limb_t
upd1 (mp_limb_t *s, mp_limb_t k)
{
  mp_limb_t _s = *s;

  if (LIKELY (k < _s * 2)) return k + 1;
  *s = _s + 1;
  return k - _s * 2;
}

int
check_some_values (int justone, int quick)
{
  mp_limb_t es, her, er, k, s[1], r[2], q[2];
  mp_size_t x;
  unsigned bits;

  es = 1 << 1;
  if (quick) {
    es <<= GMP_NUMB_BITS / 4 - 1;
    printf ("Quick, skipping some... (%u)\n", GMP_NUMB_BITS / 2);
  }
  er = 0;
  *q = es * es;
  printf ("High-half values tested, up to bits:\n");
  do {
    k  = *q - 1;
    do {
      x = mpn_sqrtrem (s, r, q, 1);
      if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	  || UNLIKELY ((x == 1) && (er != *r)))
	STOP (something_wrong (er, 0, es));

      if (UNLIKELY ((es & 0xffff) == 0))
	SPINNER(1);
      if ((*q & k) == 0) {
	*q |= k;
	er = upd (&es, k + er);
      } else {
	++*q;
	er = upd1 (&es, er);
      }
    } while (es & k);
  } while (*q != 0);
  q[1] = 1;
  SPINNER(2);
  printf ("\nValues of a single limb, tested.\n");
  if (justone) return 0;
  if (quick) {
    es <<= GMP_NUMB_BITS / 2 - 1;
    q[1] <<= GMP_NUMB_BITS - 2;
    printf ("Quick, skipping some... (%u)\n", GMP_NUMB_BITS - 2);
  }
  printf ("High-half values tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 2);
    if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 1) && (er != *r)))
      STOP (something_wrong (er, 0, es));

    if (*q == 0) {
      *q = GMP_NUMB_MAX;
      if (UNLIKELY ((es & 0xffff) == 0)) {
	if (UNLIKELY (es == GMP_NUMB_HIGHBIT))
	  break;
	SPINNER(2);
      }
      /* er = er + GMP_NUMB_MAX - 1 - es*2 // postponed */
      ++es;
      /* er = er + GMP_NUMB_MAX - 1 - 2*(es-1) =
            = er +(GMP_NUMB_MAX + 1)- 2* es = er - 2*es */
      er = upd (&es, er - 2 * es);
    } else {
      *q = 0;
      ++q[1];
      er = upd1 (&es, er);
    }
  } while (1);
  SPINNER(2);
  printf ("\nValues with at most a limb for reminder, tested.\n");
  er = GMP_NUMB_MAX; her = 0;

  printf ("High-half values tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 2);
    if (UNLIKELY (x != (her?2:(er != 0))) || UNLIKELY (*s != es)
	|| UNLIKELY ((x != 0) && ((er != *r) || ((x == 2) && (r[1] != 1)))))
      STOP (something_wrong (er, her, es));

    if (*q == 0) {
      *q = GMP_NUMB_MAX;
      if (UNLIKELY ((es & 0xffff) == 0)) {
	SPINNER(2);
      }
      if (her) {
	++es;
	her = 0;
	er = er - 2 * es;
      } else {
	her = --er != GMP_NUMB_MAX;
	if (her & (er > es * 2)) {
	  er -= es * 2 + 1;
	  her = 0;
	  ++es;
	}
      }
    } else {
      *q = 0;
      if (++q[1] == 0) break;
      if ((her == 0) | (er < es * 2)) {
	her += ++er == 0;
      }	else {
	  er -= es * 2;
	  her = 0;
	  ++es;
      }
    }
  } while (1);
  printf ("| %u\nValues of at most two limbs, tested.\n", GMP_NUMB_BITS*2);
  return 0;
}

int
check_corner_cases (int justone, int quick)
{
  mp_limb_t es, er, s[1], r[2], q[2];
  mp_size_t x;
  unsigned bits;

  es = 1;
  if (quick) {
    es <<= GMP_NUMB_BITS / 2 - 1;
    printf ("Quick, skipping some... (%u)\n", GMP_NUMB_BITS - 2);
  }
  er = 0;
  *q = es*es;
  printf ("Corner cases tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 1);
    if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 1) && (er != *r)))
      STOP (something_wrong (er, 0, es));

    if (er != 0) {
      ++es;
      if (UNLIKELY ((es & 0xffff) == 0))
	SPINNER(1);
      er = 0;
      ++*q;
    } else {
      er = es * 2;
      *q += er;
    }
  } while (*q != 0);
  q[1] = 1;
  SPINNER(2);
  printf ("\nValues of a single limb, tested.\n");
  if (justone) return 0;
  if (quick) {
    es <<= GMP_NUMB_BITS / 2 - 1;
    q[1] <<= GMP_NUMB_BITS - 2;
    printf ("Quick, skipping some... (%u)\n", GMP_NUMB_BITS - 2);
    --es;
    --q[1];
    q[0] -= es*2+1;
  }
  printf ("Corner cases tested, up to bits:\n");
  do {
    x = mpn_sqrtrem (s, r, q, 2);
    if (UNLIKELY (x != (er != 0)) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 1) && (er != *r)))
      STOP (something_wrong (er, 0, es));

    if (er != 0) {
      ++es;
      if (UNLIKELY ((es & 0xff) == 0))
	SPINNER(2);
      er = 0;
      q[1] += (++*q == 0);
      if (UNLIKELY (es == GMP_NUMB_HIGHBIT))
	break;
    } else {
      er = es * 2;
      add_ssaaaa (q[1], *q, q[1], *q, 0, er);
    }
  } while (1);
  SPINNER(2);
  printf ("\nValues with at most a limb for reminder, tested.\nCorner cases tested, up to bits:\n");
  x = mpn_sqrtrem (s, r, q, 2);
  if ((*s != es) || (x != 0))
    STOP (something_wrong (0, 0, es));
  q[1] += 1;
  x = mpn_sqrtrem (s, r, q, 2);
  if ((*s != es) || (x != 2) || (*r != 0) || (r[1] != 1))
    STOP (something_wrong (0, 1, es));
  ++es;
  q[1] += (++*q == 0);
  do {
    x = mpn_sqrtrem (s, r, q, 2);
    if (UNLIKELY (x != (er != 0) * 2) || UNLIKELY (*s != es)
	|| UNLIKELY ((x == 2) && ((er != *r) || (r[1] != 1))))
      STOP (something_wrong (er, er != 0, es));

    if (er != 0) {
      ++es;
      if (UNLIKELY (es == 0))
	break;
      if (UNLIKELY ((es & 0xff) == 0))
	SPINNER(2);
      er = 0;
      q[1] += (++*q == 0);
    } else {
      er = es * 2;
      add_ssaaaa (q[1], *q, q[1], *q, 1, er);
    }
  } while (1);
  printf ("| %u\nValues of at most two limbs, tested.\n", GMP_NUMB_BITS*2);
  return 0;
}

int
main (int argc, char **argv)
{
  int mode = 0;
  int justone = 0;
  int quick = 0;

  for (;argc > 1;--argc,++argv)
    switch (*argv[1]) {
    default:
      fprintf (stderr, "usage: sqrtrem_1_2 [x|c|s] [1|2] [q]\n");
      exit (1);
    case 'x':
      mode = 0;
      break;
    case 'c':
      mode = 1;
      break;
    case 's':
      mode = 2;
      break;
    case 'q':
      quick = 1;
      break;
    case '1':
      justone = 1;
      break;
    case '2':
      justone = 0;
    }

  switch (mode) {
  default:
    return check_all_values (justone, quick);
  case 1:
    return check_corner_cases (justone, quick);
  case 2:
    return check_some_values (justone, quick);
  }
}
