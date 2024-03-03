/* mpn_gcd_11 -- limb greatest common divisor.

Copyright 1994, 1996, 2000, 2001, 2009, 2012, 2019 Free Software Foundation,
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

mp_limb_t
mpn_gcd_11 (mp_limb_t u, mp_limb_t v)
{
  ASSERT (u & v & 1);

  /* In this loop, we represent the odd numbers ulimb and vlimb
     without the redundant least significant one bit. This reduction
     in size by one bit ensures that the high bit of t, below, is set
     if and only if vlimb > ulimb. */

  u >>= 1;
  v >>= 1;

  while (u != v)
    {
      mp_limb_t t;
      mp_limb_t vgtu;
      int c;

      t = u - v;
      vgtu = LIMB_HIGHBIT_TO_MASK (t);

      /* v <-- min (u, v) */
      v += (vgtu & t);

      /* u <-- |u - v| */
      u = (t ^ vgtu) - vgtu;

      count_trailing_zeros (c, t);
      /* We have c <= GMP_LIMB_BITS - 2 here, so that

	   ulimb >>= (c + 1);

	 would be safe. But unlike the addition c + 1, a separate
	 shift by 1 is independent of c, and can be executed in
	 parallel with count_trailing_zeros. */
      u = (u >> 1) >> c;
    }
  return (u << 1) + 1;
}
