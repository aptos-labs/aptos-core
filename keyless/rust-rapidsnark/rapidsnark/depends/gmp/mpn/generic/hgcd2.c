/* hgcd2.c

   THE FUNCTIONS IN THIS FILE ARE INTERNAL WITH MUTABLE INTERFACES.  IT IS ONLY
   SAFE TO REACH THEM THROUGH DOCUMENTED INTERFACES.  IN FACT, IT IS ALMOST
   GUARANTEED THAT THEY'LL CHANGE OR DISAPPEAR IN A FUTURE GNU MP RELEASE.

Copyright 1996, 1998, 2000-2004, 2008, 2012, 2019 Free Software Foundation,
Inc.

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

#include "gmp-impl.h"
#include "longlong.h"

#ifndef HGCD2_DIV1_METHOD
#define HGCD2_DIV1_METHOD 3
#endif

#ifndef HGCD2_DIV2_METHOD
#define HGCD2_DIV2_METHOD 2
#endif

#if GMP_NAIL_BITS != 0
#error Nails not implemented
#endif

#if HAVE_NATIVE_mpn_div_11

#define div1 mpn_div_11
/* Single-limb division optimized for small quotients.
   Returned value holds d0 = r, d1 = q. */
mp_double_limb_t div1 (mp_limb_t, mp_limb_t);

#elif HGCD2_DIV1_METHOD == 1

static inline mp_double_limb_t
div1 (mp_limb_t n0, mp_limb_t d0)
{
  mp_double_limb_t res;
  res.d1 = n0 / d0;
  res.d0 = n0 - res.d1 * d0;

  return res;
}

#elif HGCD2_DIV1_METHOD == 2

static mp_double_limb_t
div1 (mp_limb_t n0, mp_limb_t d0)
{
  mp_double_limb_t res;
  int ncnt, dcnt, cnt;
  mp_limb_t q;
  mp_limb_t mask;

  ASSERT (n0 >= d0);

  count_leading_zeros (ncnt, n0);
  count_leading_zeros (dcnt, d0);
  cnt = dcnt - ncnt;

  d0 <<= cnt;

  q = -(mp_limb_t) (n0 >= d0);
  n0 -= d0 & q;
  d0 >>= 1;
  q = -q;

  while (--cnt >= 0)
    {
      mask = -(mp_limb_t) (n0 >= d0);
      n0 -= d0 & mask;
      d0 >>= 1;
      q = (q << 1) - mask;
    }

  res.d0 = n0;
  res.d1 = q;
  return res;
}

#elif HGCD2_DIV1_METHOD == 3

static inline mp_double_limb_t
div1 (mp_limb_t n0, mp_limb_t d0)
{
  mp_double_limb_t res;
  if (UNLIKELY ((d0 >> (GMP_LIMB_BITS - 3)) != 0)
      || UNLIKELY (n0 >= (d0 << 3)))
    {
      res.d1 = n0 / d0;
      res.d0 = n0 - res.d1 * d0;
    }
  else
    {
      mp_limb_t q, mask;

      d0 <<= 2;

      mask = -(mp_limb_t) (n0 >= d0);
      n0 -= d0 & mask;
      q = 4 & mask;

      d0 >>= 1;
      mask = -(mp_limb_t) (n0 >= d0);
      n0 -= d0 & mask;
      q += 2 & mask;

      d0 >>= 1;
      mask = -(mp_limb_t) (n0 >= d0);
      n0 -= d0 & mask;
      q -= mask;

      res.d0 = n0;
      res.d1 = q;
    }
  return res;
}

#elif HGCD2_DIV1_METHOD == 4

/* Table quotients.  We extract the NBITS most significant bits of the
   numerator limb, and the corresponding bits from the divisor limb, and use
   these to form an index into the table.  This method is probably only useful
   for short pipelines with slow multiplication.

   Possible improvements:

   * Perhaps extract the highest NBITS of the divisor instead of the same bits
     as from the numerator.  That would require another count_leading_zeros,
     and a post-multiply shift of the quotient.

   * Compress tables?  Their values are tiny, and there are lots of zero
     entries (which are never used).

   * Round the table entries more cleverly?
*/

#ifndef NBITS
#define NBITS 5
#endif

#if NBITS == 5
/* This needs full division about 13.2% of the time. */
static const unsigned char tab[512] = {
17, 9, 5,4,3,2,2,2,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
18, 9, 6,4,3,2,2,2,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
19,10, 6,4,3,3,2,2,2,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,
20,10, 6,5,3,3,2,2,2,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,
21,11, 7,5,4,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,
22,11, 7,5,4,3,3,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,
23,12, 7,5,4,3,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,
24,12, 8,6,4,3,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,
25,13, 8,6,5,4,3,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,
26,13, 8,6,5,4,3,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,
27,14, 9,6,5,4,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,
28,14, 9,7,5,4,3,3,3,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,
29,15,10,7,5,4,4,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,
30,15,10,7,6,5,4,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
31,16,10,7,6,5,4,3,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,
32,16,11,8,6,5,4,3,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
};
#elif NBITS == 6
/* This needs full division about 9.8% of the time. */
static const unsigned char tab[2048] = {
33,17,11, 8, 6, 5,4,4,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 0, 0, 0, 0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
34,17,11, 8, 6, 5,4,4,3,3,3,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 0, 0, 0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
35,18,12, 9, 7, 5,5,4,3,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 0, 0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
36,18,12, 9, 7, 6,5,4,3,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 0, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
37,19,13, 9, 7, 6,5,4,4,3,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
38,19,13, 9, 7, 6,5,4,4,3,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
39,20,13,10, 7, 6,5,4,4,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
40,20,14,10, 8, 6,5,5,4,3,3,3,3,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
41,21,14,10, 8, 6,5,5,4,4,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
42,21,14,10, 8, 7,6,5,4,4,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
43,22,15,11, 8, 7,6,5,4,4,3,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
44,22,15,11, 9, 7,6,5,4,4,3,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
45,23,15,11, 9, 7,6,5,5,4,4,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
46,23,16,11, 9, 7,6,5,5,4,4,3,3,3,3,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
47,24,16,12, 9, 7,6,5,5,4,4,3,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
48,24,16,12, 9, 8,6,6,5,4,4,3,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
49,25,17,12,10, 8,7,6,5,4,4,4,3,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
50,25,17,13,10, 8,7,6,5,5,4,4,3,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
51,26,18,13,10, 8,7,6,5,5,4,4,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,
52,26,18,13,10, 8,7,6,5,5,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,1,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,0,
53,27,18,13,10, 9,7,6,5,5,4,4,4,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,0,
54,27,19,14,11, 9,7,6,6,5,4,4,4,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,0,
55,28,19,14,11, 9,7,6,6,5,5,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0,
56,28,19,14,11, 9,8,7,6,5,5,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,
57,29,20,14,11, 9,8,7,6,5,5,4,4,4,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,
58,29,20,15,11, 9,8,7,6,5,5,4,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,1,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,
59,30,20,15,12,10,8,7,6,5,5,4,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,
60,30,21,15,12,10,8,7,6,6,5,5,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,
61,31,21,15,12,10,8,7,6,6,5,5,4,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,
62,31,22,16,12,10,9,7,6,6,5,5,4,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,
63,32,22,16,13,10,9,7,7,6,5,5,4,4,4,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,2,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,
64,32,22,16,13,10,9,8,7,6,5,5,4,4,4,3,3,3,3,3,3,2,2,2,2,2,2,2,2,2,2,1,
 1, 1, 1, 1, 1, 1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1
};
#else
#error No table for provided NBITS
#endif

/* Doing tabp with a #define makes compiler warnings about pointing outside an
   object go away.  We used to define this as a variable.  It is not clear if
   e.g.  (vector[100] - 10) + 10 is well- defined as per the C standard;
   (vector[100] + 10) - 10 surely is and there is no sequence point so the
   expressions should be equivalent.  To make this safe, we might want to
   define tabp as a macro with the index as an argument.  Depending on the
   platform, relocs might allow for assembly-time or linker-time resolution to
   take place. */
#define tabp (tab - (1 << (NBITS - 1) << NBITS))

static inline mp_double_limb_t
div1 (mp_limb_t n0, mp_limb_t d0)
{
  int ncnt;
  size_t nbi, dbi;
  mp_limb_t q0;
  mp_limb_t r0;
  mp_limb_t mask;
  mp_double_limb_t res;

  ASSERT (n0 >= d0);		/* Actually only msb position is critical. */

  count_leading_zeros (ncnt, n0);
  nbi = n0 << ncnt >> (GMP_LIMB_BITS - NBITS);
  dbi = d0 << ncnt >> (GMP_LIMB_BITS - NBITS);

  q0 = tabp[(nbi << NBITS) + dbi];
  r0 = n0 - q0 * d0;
  mask = -(mp_limb_t) (r0 >= d0);
  q0 -= mask;
  r0 -= d0 & mask;

  if (UNLIKELY (r0 >= d0))
    {
      q0 = n0 / d0;
      r0 = n0 - q0 * d0;
    }

  res.d1 = q0;
  res.d0 = r0;
  return res;
}

#elif HGCD2_DIV1_METHOD == 5

/* Table inverses of divisors.  We don't bother with suppressing the msb from
   the tables.  We index with the NBITS most significant divisor bits,
   including the always-set highest bit, but use addressing trickery via tabp
   to suppress it.

   Possible improvements:

   * Do first multiply using 32-bit operations on 64-bit computers.  At least
     on most Arm64 cores, that uses 3 times less resources.  It also saves on
     many x86-64 processors.
*/

#ifndef NBITS
#define NBITS 7
#endif

#if NBITS == 5
/* This needs full division about 1.63% of the time. */
static const unsigned char tab[16] = {
 63, 59, 55, 52, 50, 47, 45, 43, 41, 39, 38, 36, 35, 34, 33, 32
};
#elif NBITS == 6
/* This needs full division about 0.93% of the time. */
static const unsigned char tab[32] = {
127,123,119,116,112,109,106,104,101, 98, 96, 94, 92, 90, 88, 86,
 84, 82, 80, 79, 77, 76, 74, 73, 72, 70, 69, 68, 67, 66, 65, 64
};
#elif NBITS == 7
/* This needs full division about 0.49% of the time. */
static const unsigned char tab[64] = {
255,251,247,243,239,236,233,229,226,223,220,217,214,211,209,206,
203,201,198,196,194,191,189,187,185,183,181,179,177,175,173,171,
169,167,166,164,162,161,159,158,156,155,153,152,150,149,147,146,
145,143,142,141,140,139,137,136,135,134,133,132,131,130,129,128
};
#elif NBITS == 8
/* This needs full division about 0.26% of the time. */
static const unsigned short tab[128] = {
511,507,503,499,495,491,488,484,480,477,473,470,467,463,460,457,
454,450,447,444,441,438,435,433,430,427,424,421,419,416,413,411,
408,406,403,401,398,396,393,391,389,386,384,382,380,377,375,373,
371,369,367,365,363,361,359,357,355,353,351,349,347,345,343,342,
340,338,336,335,333,331,329,328,326,325,323,321,320,318,317,315,
314,312,311,309,308,306,305,303,302,301,299,298,296,295,294,292,
291,290,288,287,286,285,283,282,281,280,279,277,276,275,274,273,
272,270,269,268,267,266,265,264,263,262,261,260,259,258,257,256
};
#else
#error No table for provided NBITS
#endif

/* Doing tabp with a #define makes compiler warnings about pointing outside an
   object go away.  We used to define this as a variable.  It is not clear if
   e.g.  (vector[100] - 10) + 10 is well- defined as per the C standard;
   (vector[100] + 10) - 10 surely is and there is no sequence point so the
   expressions should be equivalent.  To make this safe, we might want to
   define tabp as a macro with the index as an argument.  Depending on the
   platform, relocs might allow for assembly-time or linker-time resolution to
   take place. */
#define tabp (tab - (1 << (NBITS - 1)))

static inline mp_double_limb_t
div1 (mp_limb_t n0, mp_limb_t d0)
{
  int ncnt, dcnt;
  size_t dbi;
  mp_limb_t inv;
  mp_limb_t q0;
  mp_limb_t r0;
  mp_limb_t mask;
  mp_double_limb_t res;

  count_leading_zeros (ncnt, n0);
  count_leading_zeros (dcnt, d0);

  dbi = d0 << dcnt >> (GMP_LIMB_BITS - NBITS);
  inv = tabp[dbi];
  q0 = ((n0 << ncnt) >> (NBITS + 1)) * inv >> (GMP_LIMB_BITS - 1 + ncnt - dcnt);
  r0 = n0 - q0 * d0;
  mask = -(mp_limb_t) (r0 >= d0);
  q0 -= mask;
  r0 -= d0 & mask;

  if (UNLIKELY (r0 >= d0))
    {
      q0 = n0 / d0;
      r0 = n0 - q0 * d0;
    }

  res.d1 = q0;
  res.d0 = r0;
  return res;
}

#else
#error Unknown HGCD2_DIV1_METHOD
#endif

#if HAVE_NATIVE_mpn_div_22

#define div2 mpn_div_22
/* Two-limb division optimized for small quotients.  */
mp_limb_t div2 (mp_ptr, mp_limb_t, mp_limb_t, mp_limb_t, mp_limb_t);

#elif HGCD2_DIV2_METHOD == 1

static mp_limb_t
div2 (mp_ptr rp,
      mp_limb_t n1, mp_limb_t n0,
      mp_limb_t d1, mp_limb_t d0)
{
  mp_double_limb_t rq = div1 (n1, d1);
  if (UNLIKELY (rq.d1 > d1))
    {
      mp_limb_t n2, q, t1, t0;
      int c;

      /* Normalize */
      count_leading_zeros (c, d1);
      ASSERT (c > 0);

      n2 = n1 >> (GMP_LIMB_BITS - c);
      n1 = (n1 << c) | (n0 >> (GMP_LIMB_BITS - c));
      n0 <<= c;
      d1 = (d1 << c) | (d0 >> (GMP_LIMB_BITS - c));
      d0 <<= c;

      udiv_qrnnd (q, n1, n2, n1, d1);
      umul_ppmm (t1, t0, q, d0);
      if (t1 > n1 || (t1 == n1 && t0 > n0))
	{
	  ASSERT (q > 0);
	  q--;
	  sub_ddmmss (t1, t0, t1, t0, d1, d0);
	}
      sub_ddmmss (n1, n0, n1, n0, t1, t0);

      /* Undo normalization */
      rp[0] = (n0 >> c) | (n1 << (GMP_LIMB_BITS - c));
      rp[1] = n1 >> c;

      return q;
    }
  else
    {
      mp_limb_t q, t1, t0;
      n1 = rq.d0;
      q = rq.d1;
      umul_ppmm (t1, t0, q, d0);
      if (UNLIKELY (t1 >= n1) && (t1 > n1 || t0 > n0))
	{
	  ASSERT (q > 0);
	  q--;
	  sub_ddmmss (t1, t0, t1, t0, d1, d0);
	}
      sub_ddmmss (rp[1], rp[0], n1, n0, t1, t0);
      return q;
    }
}

#elif HGCD2_DIV2_METHOD == 2

/* Bit-wise div2. Relies on fast count_leading_zeros. */
static mp_limb_t
div2 (mp_ptr rp,
      mp_limb_t n1, mp_limb_t n0,
      mp_limb_t d1, mp_limb_t d0)
{
  mp_limb_t q = 0;
  int ncnt;
  int dcnt;

  count_leading_zeros (ncnt, n1);
  count_leading_zeros (dcnt, d1);
  dcnt -= ncnt;

  d1 = (d1 << dcnt) + (d0 >> 1 >> (GMP_LIMB_BITS - 1 - dcnt));
  d0 <<= dcnt;

  do
    {
      mp_limb_t mask;
      q <<= 1;
      if (UNLIKELY (n1 == d1))
	mask = -(n0 >= d0);
      else
	mask = -(n1 > d1);

      q -= mask;

      sub_ddmmss (n1, n0, n1, n0, mask & d1, mask & d0);

      d0 = (d1 << (GMP_LIMB_BITS - 1)) | (d0 >> 1);
      d1 = d1 >> 1;
    }
  while (dcnt--);

  rp[0] = n0;
  rp[1] = n1;

  return q;
}
#else
#error Unknown HGCD2_DIV2_METHOD
#endif

/* Reduces a,b until |a-b| (almost) fits in one limb + 1 bit. Constructs
   matrix M. Returns 1 if we make progress, i.e. can perform at least
   one subtraction. Otherwise returns zero. */

/* FIXME: Possible optimizations:

   The div2 function starts with checking the most significant bit of
   the numerator. We can maintained normalized operands here, call
   hgcd with normalized operands only, which should make the code
   simpler and possibly faster.

   Experiment with table lookups on the most significant bits.

   This function is also a candidate for assembler implementation.
*/
int
mpn_hgcd2 (mp_limb_t ah, mp_limb_t al, mp_limb_t bh, mp_limb_t bl,
	   struct hgcd_matrix1 *M)
{
  mp_limb_t u00, u01, u10, u11;

  if (ah < 2 || bh < 2)
    return 0;

  if (ah > bh || (ah == bh && al > bl))
    {
      sub_ddmmss (ah, al, ah, al, bh, bl);
      if (ah < 2)
	return 0;

      u00 = u01 = u11 = 1;
      u10 = 0;
    }
  else
    {
      sub_ddmmss (bh, bl, bh, bl, ah, al);
      if (bh < 2)
	return 0;

      u00 = u10 = u11 = 1;
      u01 = 0;
    }

  if (ah < bh)
    goto subtract_a;

  for (;;)
    {
      ASSERT (ah >= bh);
      if (ah == bh)
	goto done;

      if (ah < (CNST_LIMB(1) << (GMP_LIMB_BITS / 2)))
	{
	  ah = (ah << (GMP_LIMB_BITS / 2) ) + (al >> (GMP_LIMB_BITS / 2));
	  bh = (bh << (GMP_LIMB_BITS / 2) ) + (bl >> (GMP_LIMB_BITS / 2));

	  break;
	}

      /* Subtract a -= q b, and multiply M from the right by (1 q ; 0
	 1), affecting the second column of M. */
      ASSERT (ah > bh);
      sub_ddmmss (ah, al, ah, al, bh, bl);

      if (ah < 2)
	goto done;

      if (ah <= bh)
	{
	  /* Use q = 1 */
	  u01 += u00;
	  u11 += u10;
	}
      else
	{
	  mp_limb_t r[2];
	  mp_limb_t q = div2 (r, ah, al, bh, bl);
	  al = r[0]; ah = r[1];
	  if (ah < 2)
	    {
	      /* A is too small, but q is correct. */
	      u01 += q * u00;
	      u11 += q * u10;
	      goto done;
	    }
	  q++;
	  u01 += q * u00;
	  u11 += q * u10;
	}
    subtract_a:
      ASSERT (bh >= ah);
      if (ah == bh)
	goto done;

      if (bh < (CNST_LIMB(1) << (GMP_LIMB_BITS / 2)))
	{
	  ah = (ah << (GMP_LIMB_BITS / 2) ) + (al >> (GMP_LIMB_BITS / 2));
	  bh = (bh << (GMP_LIMB_BITS / 2) ) + (bl >> (GMP_LIMB_BITS / 2));

	  goto subtract_a1;
	}

      /* Subtract b -= q a, and multiply M from the right by (1 0 ; q
	 1), affecting the first column of M. */
      sub_ddmmss (bh, bl, bh, bl, ah, al);

      if (bh < 2)
	goto done;

      if (bh <= ah)
	{
	  /* Use q = 1 */
	  u00 += u01;
	  u10 += u11;
	}
      else
	{
	  mp_limb_t r[2];
	  mp_limb_t q = div2 (r, bh, bl, ah, al);
	  bl = r[0]; bh = r[1];
	  if (bh < 2)
	    {
	      /* B is too small, but q is correct. */
	      u00 += q * u01;
	      u10 += q * u11;
	      goto done;
	    }
	  q++;
	  u00 += q * u01;
	  u10 += q * u11;
	}
    }

  /* NOTE: Since we discard the least significant half limb, we don't get a
     truly maximal M (corresponding to |a - b| < 2^{GMP_LIMB_BITS +1}). */
  /* Single precision loop */
  for (;;)
    {
      ASSERT (ah >= bh);

      ah -= bh;
      if (ah < (CNST_LIMB (1) << (GMP_LIMB_BITS / 2 + 1)))
	break;

      if (ah <= bh)
	{
	  /* Use q = 1 */
	  u01 += u00;
	  u11 += u10;
	}
      else
	{
	  mp_double_limb_t rq = div1 (ah, bh);
	  mp_limb_t q = rq.d1;
	  ah = rq.d0;

	  if (ah < (CNST_LIMB(1) << (GMP_LIMB_BITS / 2 + 1)))
	    {
	      /* A is too small, but q is correct. */
	      u01 += q * u00;
	      u11 += q * u10;
	      break;
	    }
	  q++;
	  u01 += q * u00;
	  u11 += q * u10;
	}
    subtract_a1:
      ASSERT (bh >= ah);

      bh -= ah;
      if (bh < (CNST_LIMB (1) << (GMP_LIMB_BITS / 2 + 1)))
	break;

      if (bh <= ah)
	{
	  /* Use q = 1 */
	  u00 += u01;
	  u10 += u11;
	}
      else
	{
	  mp_double_limb_t rq = div1 (bh, ah);
	  mp_limb_t q = rq.d1;
	  bh = rq.d0;

	  if (bh < (CNST_LIMB(1) << (GMP_LIMB_BITS / 2 + 1)))
	    {
	      /* B is too small, but q is correct. */
	      u00 += q * u01;
	      u10 += q * u11;
	      break;
	    }
	  q++;
	  u00 += q * u01;
	  u10 += q * u11;
	}
    }

 done:
  M->u[0][0] = u00; M->u[0][1] = u01;
  M->u[1][0] = u10; M->u[1][1] = u11;

  return 1;
}

/* Sets (r;b) = (a;b) M, with M = (u00, u01; u10, u11). Vector must
 * have space for n + 1 limbs. Uses three buffers to avoid a copy*/
mp_size_t
mpn_hgcd_mul_matrix1_vector (const struct hgcd_matrix1 *M,
			     mp_ptr rp, mp_srcptr ap, mp_ptr bp, mp_size_t n)
{
  mp_limb_t ah, bh;

  /* Compute (r,b) <-- (u00 a + u10 b, u01 a + u11 b) as

     r  = u00 * a
     r += u10 * b
     b *= u11
     b += u01 * a
  */

#if HAVE_NATIVE_mpn_addaddmul_1msb0
  ah = mpn_addaddmul_1msb0 (rp, ap, bp, n, M->u[0][0], M->u[1][0]);
  bh = mpn_addaddmul_1msb0 (bp, bp, ap, n, M->u[1][1], M->u[0][1]);
#else
  ah =     mpn_mul_1 (rp, ap, n, M->u[0][0]);
  ah += mpn_addmul_1 (rp, bp, n, M->u[1][0]);

  bh =     mpn_mul_1 (bp, bp, n, M->u[1][1]);
  bh += mpn_addmul_1 (bp, ap, n, M->u[0][1]);
#endif
  rp[n] = ah;
  bp[n] = bh;

  n += (ah | bh) > 0;
  return n;
}
