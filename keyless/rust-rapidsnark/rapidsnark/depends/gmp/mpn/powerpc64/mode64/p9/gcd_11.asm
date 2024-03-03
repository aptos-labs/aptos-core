dnl  PowerPC-64 mpn_gcd_11.

dnl  Copyright 2000-2002, 2005, 2009, 2011-2013, 2019 Free Software Foundation,
dnl  Inc.

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
C POWER3/PPC630		 -
C POWER4/PPC970		 -
C POWER5		 -
C POWER6		 -
C POWER7		 -
C POWER8		 -
C POWER9		 5.75
C Numbers measured with: speed -CD -s16-64 -t48 mpn_gcd_1

define(`u0',    `r3')
define(`v0',    `r4')

define(`cnt',  `r9')dnl

ASM_START()
PROLOGUE(mpn_gcd_11)
	b	L(odd)

	ALIGN(16)
L(top):	isel	v0, u0, v0, 29		C v = min(u,v)
	isel	u0, r10, r11, 29	C u = |v - u|
	srd	u0, u0, cnt
L(odd):	subf	r10, u0, v0		C r10 = v - u
	subf	r11, v0, u0		C r11 = u - v
	cmpld	cr7, v0, u0
	cnttzd	cnt, r10
	bne	cr7, L(top)

L(end):	blr
EPILOGUE()
