/* mpz_xor -- Logical xor.

Copyright 1991, 1993, 1994, 1996, 1997, 2000, 2001, 2005, 2012,
2015-2018 Free Software Foundation, Inc.

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
mpz_xor (mpz_ptr res, mpz_srcptr op1, mpz_srcptr op2)
{
  mp_srcptr op1_ptr, op2_ptr;
  mp_size_t op1_size, op2_size;
  mp_ptr res_ptr;
  mp_size_t res_size;

  op1_size = SIZ(op1);
  op2_size = SIZ(op2);

  if (op1_size < op2_size)
    {
      MPZ_SRCPTR_SWAP (op1, op2);
      MP_SIZE_T_SWAP (op1_size, op2_size);
    }

  op1_ptr = PTR(op1);
  res_ptr = PTR(res);

  if (op2_size >= 0)
    {
      if (res_ptr != op1_ptr)
	{
	  res_ptr = MPZ_REALLOC (res, op1_size);
	  MPN_COPY (res_ptr + op2_size, op1_ptr + op2_size,
		    op1_size - op2_size);
	}
      if (LIKELY (op2_size != 0))
	mpn_xor_n (res_ptr, op1_ptr, PTR(op2), op2_size);
      res_size = op1_size;

      MPN_NORMALIZE (res_ptr, res_size);
      SIZ(res) = res_size;
    }
  else
    {
      mp_ptr opx;
      TMP_DECL;

      op2_size = -op2_size;
      TMP_MARK;
      if (op1_size < 0)
	{
	  mp_ptr opy;

	  /* Both operands are negative, the result will be positive.
	      (-OP1) ^ (-OP2) =
	     = ~(OP1 - 1) ^ ~(OP2 - 1) =
	     = (OP1 - 1) ^ (OP2 - 1)  */

	  op1_size = -op1_size;

	  /* Possible optimization: Decrease mpn_sub precision,
	     as we won't use the entire res of both.  */
	  TMP_ALLOC_LIMBS_2 (opx, op1_size, opy, op2_size);
	  mpn_sub_1 (opx, op1_ptr, op1_size, (mp_limb_t) 1);
	  op1_ptr = opx;

	  mpn_sub_1 (opy, PTR(op2), op2_size, (mp_limb_t) 1);
	  op2_ptr = opy;

	  res_ptr = MPZ_NEWALLOC (res, op2_size);
	  /* Don't re-read OP1_PTR and OP2_PTR.  They point to temporary
	     space--never to the space PTR(res) used to point to before
	     reallocation.  */

	  MPN_COPY (res_ptr + op1_size, op2_ptr + op1_size,
		    op2_size - op1_size);
	  mpn_xor_n (res_ptr, op1_ptr, op2_ptr, op1_size);
	  TMP_FREE;
	  res_size = op2_size;

	  MPN_NORMALIZE (res_ptr, res_size);
	  SIZ(res) = res_size;
	}
      else
	{
	  /* Operand 2 negative, so will be the result.
	     -(OP1 ^ (-OP2)) = -(OP1 ^ ~(OP2 - 1)) =
	     = ~(OP1 ^ ~(OP2 - 1)) + 1 =
	     = (OP1 ^ (OP2 - 1)) + 1      */

	  res_size = MAX (op1_size, op2_size);
	  res_ptr = MPZ_REALLOC (res, res_size + 1);
	  op1_ptr = PTR(op1);

	  opx = TMP_ALLOC_LIMBS (op2_size);
	  mpn_sub_1 (opx, PTR(op2), op2_size, (mp_limb_t) 1);
	  op2_ptr = opx;

	  if (res_size == op1_size)
	    {
	      MPN_COPY (res_ptr + op2_size, op1_ptr + op2_size, op1_size - op2_size);
	      mpn_xor_n (res_ptr, op1_ptr, op2_ptr, op2_size);
	    }
	  else
	    {
	      MPN_COPY (res_ptr + op1_size, op2_ptr + op1_size, op2_size - op1_size);
	      if (LIKELY (op1_size != 0))
		mpn_xor_n (res_ptr, op1_ptr, op2_ptr, op1_size);
	    }
	  TMP_FREE;

	  res_ptr[res_size] = 0;
	  MPN_INCR_U (res_ptr, res_size + 1, (mp_limb_t) 1);
	  res_size += res_ptr[res_size];

	  MPN_NORMALIZE_NOT_ZERO (res_ptr, res_size);
	  SIZ(res) = -res_size;
	}
    }
}
