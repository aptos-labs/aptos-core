dnl  PowerPC-64 mpn_gcd_11.

dnl  Copyright 2000-2002, 2005, 2009, 2011-2013 Free Software Foundation, Inc.

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

C		    cycles/bit (approx)
C POWER3/PPC630		 ?
C POWER4/PPC970		 8.5	obsolete
C POWER5		 ?
C POWER6		 ?
C POWER7		 9.4	obsolete
C POWER8		 ?
C POWER9		 ?
C Numbers measured with: speed -CD -s16-64 -t48 mpn_gcd_1

define(`u0',    `r3')
define(`v0',    `r4')

define(`mask', `r0')dnl
define(`a1',   `r4')dnl
define(`a2',   `r5')dnl
define(`d1',   `r6')dnl
define(`d2',   `r7')dnl
define(`cnt',  `r9')dnl

ASM_START()
PROLOGUE(mpn_gcd_11)
	li	r12, 63
	mr	r8, v0
	subf.	r10, u0, v0		C r10 = d - a
	beq	L(end)

	ALIGN(16)
L(top):	subfc	r11, r8, r3		C r11 = a - d
	and	d2, r11, r10
	subfe	mask, mask, mask
	cntlzd	cnt, d2
	and	a1, r10, mask		C d - a
	andc	a2, r11,  mask		C a - d
	and	d1, r3, mask		C a
	andc	d2, r8, mask		C d
	or	r3, a1, a2		C new a
	subf	cnt, cnt, r12
	or	r8, d1, d2		C new d
	srd	r3, r3, cnt
	subf.	r10, r3, r8		C r10 = d - a
	bne	L(top)

L(end):	blr
EPILOGUE()
