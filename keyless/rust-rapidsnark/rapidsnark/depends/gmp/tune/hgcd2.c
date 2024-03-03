/* mpn/generic/hgcd2.c for tuning

Copyright 2019 Free Software Foundation, Inc.

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

#define TUNE_PROGRAM_BUILD 1

#include "gmp-impl.h"

hgcd2_func_t mpn_hgcd2_default;

hgcd2_func_t *hgcd2_func = &mpn_hgcd2_default;

int
mpn_hgcd2 (mp_limb_t ah, mp_limb_t al, mp_limb_t bh, mp_limb_t bl,
	   struct hgcd_matrix1 *M)
{
  return hgcd2_func(ah, al, bh, bl, M);
}

#undef mpn_hgcd2
#define mpn_hgcd2 mpn_hgcd2_default

#include "mpn/generic/hgcd2.c"
