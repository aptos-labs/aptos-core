/* mpz_init_set_str(string, base) -- Convert the \0-terminated string STRING in
   base BASE to a multiple precision integer.  Allow white space in the string.
   If BASE == 0 determine the base in the C standard way, i.e.  0xhh...h means
   base 16, 0oo...o means base 8, otherwise assume base 10.

Copyright 1991, 1993-1995, 2000-2002, 2012 Free Software Foundation, Inc.

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

int
mpz_init_set_str (mpz_ptr x, const char *str, int base)
{
  static const mp_limb_t dummy_limb=0xc1a0;
  ALLOC (x) = 0;
  PTR (x) = (mp_ptr) &dummy_limb;

  /* if str has no digits mpz_set_str leaves x->_mp_size unset */
  SIZ (x) = 0;

  return mpz_set_str (x, str, base);
}
