/* mpq_swap (U, V) -- Swap U and V.

Copyright 1997, 1998, 2000, 2001, 2018 Free Software Foundation, Inc.

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
mpq_swap (mpq_ptr u, mpq_ptr v) __GMP_NOTHROW
{
  MP_SIZE_T_SWAP (ALLOC(NUM(u)), ALLOC(NUM(v)));
  MP_SIZE_T_SWAP (ALLOC(DEN(u)), ALLOC(DEN(v)));
  MP_SIZE_T_SWAP (SIZ(NUM(u)), SIZ(NUM(v)));
  MP_SIZE_T_SWAP (SIZ(DEN(u)), SIZ(DEN(v)));
  MP_PTR_SWAP (PTR(NUM(u)), PTR(NUM(v)));
  MP_PTR_SWAP (PTR(DEN(u)), PTR(DEN(v)));
}
