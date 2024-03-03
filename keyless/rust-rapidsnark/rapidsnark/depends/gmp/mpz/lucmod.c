/* mpz_lucas_mod -- Helper function for the strong Lucas
   primality test.

   THE FUNCTIONS IN THIS FILE ARE FOR INTERNAL USE ONLY.  THEY'RE ALMOST
   CERTAIN TO BE SUBJECT TO INCOMPATIBLE CHANGES OR DISAPPEAR COMPLETELY IN
   FUTURE GNU MP RELEASES.

Copyright 2018 Free Software Foundation, Inc.

Contributed by Marco Bodrato.

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

/* Computes V_{k+1}, Q^{k+1} (mod n) for the Lucas' sequence	*/
/* with P=1, Q=Q; k = n>>b0.	*/
/* Requires n > 4; b0 > 0; -2*Q must not overflow a long.	*/
/* If U_{k+1}==0 (mod n) or V_{k+1}==0 (mod n), it returns 1,	*/
/* otherwise it returns 0 and sets V=V_{k+1} and Qk=Q^{k+1}.	*/
/* V will never grow beyond SIZ(n), Qk not beyond 2*SIZ(n).	*/
int
mpz_lucas_mod (mpz_ptr V, mpz_ptr Qk, long Q,
	       mp_bitcnt_t b0, mpz_srcptr n, mpz_ptr T1, mpz_ptr T2)
{
  mp_bitcnt_t bs;
  int res;

  ASSERT (b0 > 0);
  ASSERT (SIZ (n) > 1 || SIZ (n) > 0 && PTR (n) [0] > 4);

  mpz_set_ui (V, 1); /* U1 = 1 */
  bs = mpz_sizeinbase (n, 2) - 2;
  if (UNLIKELY (bs < b0))
    {
      /* n = 2^b0 - 1, should we use Lucas-Lehmer instead? */
      ASSERT (bs == b0 - 2);
      mpz_set_si (Qk, Q);
      return 0;
    }
  mpz_set_ui (Qk, 1); /* U2 = 1 */

  do
    {
      /* We use the iteration suggested in "Elementary Number Theory"	*/
      /* by Peter Hackman (November 1, 2009), section "L.XVII Scalar	*/
      /* Formulas", from http://hackmat.se/kurser/TATM54/booktot.pdf	*/
      /* U_{2k} = 2*U_{k+1}*U_k - P*U_k^2	*/
      /* U_{2k+1} = U_{k+1}^2  - Q*U_k^2	*/
      /* U_{2k+2} = P*U_{k+1}^2 - 2*Q*U_{k+1}*U_k	*/
      /* We note that U_{2k+2} = P*U_{2k+1} - Q*U_{2k}	*/
      /* The formulas are specialized for P=1, and only squares:	*/
      /* U_{2k}   = U_{k+1}^2 - |U_{k+1} - U_k|^2	*/
      /* U_{2k+1} = U_{k+1}^2 - Q*U_k^2		*/
      /* U_{2k+2} = U_{2k+1}  - Q*U_{2k}	*/
      mpz_mul (T1, Qk, Qk);	/* U_{k+1}^2		*/
      mpz_sub (Qk, V, Qk);	/* |U_{k+1} - U_k|	*/
      mpz_mul (T2, Qk, Qk);	/* |U_{k+1} - U_k|^2	*/
      mpz_mul (Qk, V, V);	/* U_k^2		*/
      mpz_sub (T2, T1, T2);	/* U_{k+1}^2 - (U_{k+1} - U_k)^2	*/
      if (Q > 0)		/* U_{k+1}^2 - Q U_k^2 = U_{2k+1}	*/
	mpz_submul_ui (T1, Qk, Q);
      else
	mpz_addmul_ui (T1, Qk, NEG_CAST (unsigned long, Q));

      /* A step k->k+1 is performed if the bit in $n$ is 1	*/
      if (mpz_tstbit (n, bs))
	{
	  /* U_{2k+2} = U_{2k+1} - Q*U_{2k}	*/
	  mpz_mul_si (T2, T2, Q);
	  mpz_sub (T2, T1, T2);
	  mpz_swap (T1, T2);
	}
      mpz_tdiv_r (Qk, T1, n);
      mpz_tdiv_r (V, T2, n);
    } while (--bs >= b0);

  res = SIZ (Qk) == 0;
  if (!res) {
    mpz_mul_si (T1, V, -2*Q);
    mpz_add (T1, Qk, T1);	/* V_k = U_k - 2Q*U_{k-1} */
    mpz_tdiv_r (V, T1, n);
    res = SIZ (V) == 0;
    if (!res && b0 > 1) {
      /* V_k and Q^k will be needed for further check, compute them.	*/
      /* FIXME: Here we compute V_k^2 and store V_k, but the former	*/
      /* will be recomputed by the calling function, shoul we store	*/
      /* that instead?							*/
      mpz_mul (T2, T1, T1);	/* V_k^2 */
      mpz_mul (T1, Qk, Qk);	/* P^2 U_k^2 = U_k^2 */
      mpz_sub (T2, T2, T1);
      ASSERT (SIZ (T2) == 0 || PTR (T2) [0] % 4 == 0);
      mpz_tdiv_q_2exp (T2, T2, 2);	/* (V_k^2 - P^2 U_k^2) / 4 */
      if (Q > 0)		/* (V_k^2 - (P^2 -4Q) U_k^2) / 4 = Q^k */
	mpz_addmul_ui (T2, T1, Q);
      else
	mpz_submul_ui (T2, T1, NEG_CAST (unsigned long, Q));
      mpz_tdiv_r (Qk, T2, n);
    }
  }

  return res;
}
