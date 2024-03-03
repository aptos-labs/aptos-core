dnl  ARM v6t2 mpn_gcd_11.

dnl  Copyright 2000-2002, 2005, 2009, 2011, 2012, 2019 Free Software Foundation,
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

C	     cycles/bit (approx)
C StrongARM	 -
C XScale	 -
C Cortex-A5	 5.2
C Cortex-A7	 5.04
C Cortex-A8	 3.59
C Cortex-A9	 9.5
C Cortex-A15	 3.2
C Cortex-A17	 5.25
C Cortex-A53	 3.57

define(`u0',    `r0')
define(`v0',    `r1')

ASM_START()
	TEXT
	ALIGN(64)
PROLOGUE(mpn_gcd_11)
	subs	r3, u0, v0	C			0
	beq	L(end)		C

	ALIGN(16)
L(top):	rbit	r12, r3		C			1,5
	clz	r12, r12	C			2
	rsbcc	r3, r3, #0	C v = abs(u-v), even	1
	movcs	u0, v0		C u = min(u,v)		1
	lsr	v0, r3, r12	C			3
	subs	r3, u0, v0	C			4
	bne	L(top)		C

L(end):	bx	lr
EPILOGUE()
