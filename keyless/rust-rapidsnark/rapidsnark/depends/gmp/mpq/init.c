/* mpq_init -- Make a new rational number with value 0/1.

Copyright 1991, 1994, 1995, 2000-2002, 2015, 2018 Free Software
Foundation, Inc.

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

void
mpq_init (mpq_t x)
{
  static const mp_limb_t dummy_limb=0xc1a0;
  ALLOC(NUM(x)) = 0;
  PTR(NUM(x)) = (mp_ptr) &dummy_limb;
  SIZ(NUM(x)) = 0;
  ALLOC(DEN(x)) = 1;
  PTR(DEN(x)) = __GMP_ALLOCATE_FUNC_LIMBS (1);
  PTR(DEN(x))[0] = 1;
  SIZ(DEN(x)) = 1;
}
