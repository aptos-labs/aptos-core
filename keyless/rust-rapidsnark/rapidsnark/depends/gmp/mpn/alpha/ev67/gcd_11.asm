dnl  Alpha ev67 mpn_gcd_11 -- Nx1 greatest common divisor.

dnl  Copyright 2003, 2004 Free Software Foundation, Inc.

dnl  This file is part of the GNU MP Library.
dnl
dnl  The GNU MP Library is free software; you can redistribute it and/or modify
dnl  it under the terms of either:
dnl
dnl    * the GNU Lesser General Public License as published by the Free
dnl      Software Foundation; either version 3 of the License, or (at your
dnl      option) any later version.
dnl
dnl  or
dnl
dnl    * the GNU General Public License as published by the Free Software
dnl      Foundation; either version 2 of the License, or (at your option) any
dnl      later version.
dnl
dnl  or both in parallel, as here.
dnl
dnl  The GNU MP Library is distributed in the hope that it will be useful, but
dnl  WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
dnl  or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
dnl  for more details.
dnl
dnl  You should have received copies of the GNU General Public License and the
dnl  GNU Lesser General Public License along with the GNU MP Library.  If not,
dnl  see https://www.gnu.org/licenses/.

include(`../config.m4')


C ev67: 3.4 cycles/bitpair for 1x1 part


C mp_limb_t mpn_gcd_1 (mp_srcptr xp, mp_size_t xsize, mp_limb_t y);
C
C In the 1x1 part, the algorithm is to change x,y to abs(x-y),min(x,y) and
C strip trailing zeros from abs(x-y) to maintain x and y both odd.
C
C The trailing zeros are calculated from just x-y, since in twos-complement
C there's the same number of trailing zeros on d or -d.  This means the cttz
C runs in parallel with abs(x-y).
C
C The loop takes 5 cycles, and at 0.68 iterations per bit for two N-bit
C operands with this algorithm gives the measured 3.4 c/l.
C
C The slottings shown are for SVR4 style systems, Unicos differs in the
C initial gp setup and the LEA.


ASM_START()
PROLOGUE(mpn_gcd_11)
	mov	r16, r0
	mov	r17, r1

	ALIGN(16)
L(top):	subq	r0, r1, r7		C l0  d = x - y
	cmpult	r0, r1, r16		C u0  test x >= y

	subq	r1, r0, r4		C l0  new_x = y - x
	cttz	r7, r8			C U0  d twos

	cmoveq	r16, r7, r4		C l0  new_x = d if x>=y
	cmovne	r16, r0, r1		C u0  y = x if x<y
	unop				C l   \ force cmoveq into l0
	unop				C u   /

	C				C cmoveq2 L0, cmovne2 U0

	srl	r4, r8, r0		C U0  x = new_x >> twos
	bne	r7, L(top)		C U1  stop when d==0


L(end):	mov	r1, r0			C U0  return y << common_twos
	ret	r31, (r26), 1		C L0
EPILOGUE()
ASM_END()
