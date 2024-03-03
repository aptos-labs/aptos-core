/* mpn_gcd_22 -- double limb greatest common divisor.

Copyright 1994, 1996, 2000, 2001, 2009, 2012, 2019 Free Software Foundation, Inc.

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

#if GMP_NAIL_BITS > 0
#error Nails not supported.
#endif

mp_double_limb_t
mpn_gcd_22 (mp_limb_t u1, mp_limb_t u0, mp_limb_t v1, mp_limb_t v0)
{
  mp_double_limb_t g;
  ASSERT (u0 & v0 & 1);

  /* Implicit least significant bit */
  u0 = (u0 >> 1) | (u1 << (GMP_LIMB_BITS - 1));
  u1 >>= 1;

  v0 = (v0 >> 1) | (v1 << (GMP_LIMB_BITS - 1));
  v1 >>= 1;

  while (u1 || v1) /* u1 == 0 can happen at most twice per call */
    {
      mp_limb_t vgtu, t1, t0;
      sub_ddmmss (t1, t0, u1, u0, v1, v0);
      vgtu = LIMB_HIGHBIT_TO_MASK(t1);

      if (UNLIKELY (t0 == 0))
	{
	  int c;
	  if (t1 == 0)
	    {
	      g.d1 = (u1 << 1) | (u0 >> (GMP_LIMB_BITS - 1));
	      g.d0 = (u0 << 1) | 1;
	      return g;
	    }
	  count_trailing_zeros (c, t1);

	  /* v1 = min (u1, v1) */
	  v1 += (vgtu & t1);
	  /* u0 = |u1 - v1| */
	  u0 = (t1 ^ vgtu) - vgtu;
	  ASSERT (c < GMP_LIMB_BITS - 1);
	  u0 >>= c + 1;
	  u1 = 0;
	}
      else
	{
	  int c;
	  count_trailing_zeros (c, t0);
	  c++;
	  /* V <-- min (U, V).

	     Assembly version should use cmov. Another alternative,
	     avoiding carry propagation, would be

	     v0 += vgtu & t0; v1 += vtgu & (u1 - v1);
	  */
	  add_ssaaaa (v1, v0, v1, v0, vgtu & t1, vgtu & t0);
	  /* U  <--  |U - V|
	     No carry handling needed in this conditional negation,
	     since t0 != 0. */
	  u0 = (t0 ^ vgtu) - vgtu;
	  u1 = t1 ^ vgtu;
	  if (UNLIKELY (c == GMP_LIMB_BITS))
	    {
	      u0 = u1;
	      u1 = 0;
	    }
	  else
	    {
	      u0 = (u0 >> c) | (u1 << (GMP_LIMB_BITS - c));
	      u1 >>= c;
	    }
	}
    }
  while ((v0 | u0) & GMP_LIMB_HIGHBIT)
    { /* At most two iterations */
      mp_limb_t vgtu, t0;
      int c;
      sub_ddmmss (vgtu, t0, 0, u0, 0, v0);
      if (UNLIKELY (t0 == 0))
	{
	  g.d1 = u0 >> (GMP_LIMB_BITS - 1);
	  g.d0 = (u0 << 1) | 1;
	  return g;
	}

      /* v <-- min (u, v) */
      v0 += (vgtu & t0);

      /* u <-- |u - v| */
      u0 = (t0 ^ vgtu) - vgtu;

      count_trailing_zeros (c, t0);
      u0 = (u0 >> 1) >> c;
    }

  g.d0 = mpn_gcd_11 ((u0 << 1) + 1, (v0 << 1) + 1);
  g.d1 = 0;
  return g;
}
