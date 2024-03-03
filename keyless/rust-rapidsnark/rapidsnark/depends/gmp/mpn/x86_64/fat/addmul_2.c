/* Fat binary fallback mpn_addmul_2.

Copyright 2016 Free Software Foundation, Inc.

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

mp_limb_t
mpn_addmul_2 (mp_ptr rp, mp_srcptr up, mp_size_t n, const mp_limb_t vp[2])
{
  rp[n] = mpn_addmul_1 (rp,     up, n, vp[0]);
  return  mpn_addmul_1 (rp + 1, up, n, vp[1]);
}
