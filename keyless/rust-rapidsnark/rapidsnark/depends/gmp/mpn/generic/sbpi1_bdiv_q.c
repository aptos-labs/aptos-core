/* mpn_sbpi1_bdiv_q -- schoolbook Hensel division with precomputed inverse,
   returning quotient only.

   Contributed to the GNU project by Niels Möller and Torbjörn Granlund.

   THE FUNCTIONS IN THIS FILE ARE INTERNAL FUNCTIONS WITH MUTABLE INTERFACES.
   IT IS ONLY SAFE TO REACH THEM THROUGH DOCUMENTED INTERFACES.  IN FACT, IT IS
   ALMOST GUARANTEED THAT THEY'LL CHANGE OR DISAPPEAR IN A FUTURE GMP RELEASE.

Copyright 2005, 2006, 2009, 2011, 2012, 2017 Free Software Foundation, Inc.

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

/* Computes Q = - U / D mod B^un, destroys U.

   D must be odd. dinv is (-D)^-1 mod B.

*/

void
mpn_sbpi1_bdiv_q (mp_ptr qp,
		  mp_ptr up, mp_size_t un,
		  mp_srcptr dp, mp_size_t dn,
		  mp_limb_t dinv)
{
  mp_size_t i;
  mp_limb_t q;

  ASSERT (dn > 0);
  ASSERT (un >= dn);
  ASSERT ((dp[0] & 1) != 0);
  ASSERT (-(dp[0] * dinv) == 1);
  ASSERT (up == qp || !MPN_OVERLAP_P (up, un, qp, un - dn));

  if (un > dn)
    {
      mp_limb_t cy, hi;
      for (i = un - dn - 1, cy = 0; i > 0; i--)
	{
	  q = dinv * up[0];
	  hi = mpn_addmul_1 (up, dp, dn, q);

	  ASSERT (up[0] == 0);
	  *qp++ = q;
	  hi += cy;
	  cy = hi < cy;
	  hi += up[dn];
	  cy += hi < up[dn];
	  up[dn] = hi;
	  up++;
	}
      q = dinv * up[0];
      hi = cy + mpn_addmul_1 (up, dp, dn, q);
      ASSERT (up[0] == 0);
      *qp++ = q;
      up[dn] += hi;
      up++;
    }
  for (i = dn; i > 1; i--)
    {
      mp_limb_t q = dinv * up[0];
      mpn_addmul_1 (up, dp, i, q);
      ASSERT (up[0] == 0);
      *qp++ = q;
      up++;
    }

  /* Final limb */
  *qp = dinv * up[0];
}
