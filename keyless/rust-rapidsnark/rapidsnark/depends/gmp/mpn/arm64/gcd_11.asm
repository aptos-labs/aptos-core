dnl  ARM v8a mpn_gcd_11.

dnl  Based on the K7 gcd_1.asm, by Kevin Ryde.  Rehacked for ARM by Torbjorn
dnl  Granlund.

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

changecom(blah)

C	     cycles/bit (approx)
C Cortex-A35	 ?
C Cortex-A53	 ?
C Cortex-A55	 ?
C Cortex-A57	 ?
C Cortex-A72	 ?
C Cortex-A73	 ?
C Cortex-A75	 ?
C Cortex-A76	 ?
C Cortex-A77	 ?
C Numbers measured with: speed -CD -s16-64 -t48 mpn_gcd_1

define(`u0',    `x0')
define(`v0',    `x1')

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_gcd_11)
	subs	x3, u0, v0		C			0
	b.eq	L(end)			C

	ALIGN(16)
L(top):	rbit	x12, x3			C			1,5
	clz	x12, x12		C			2
	csneg	x3, x3, x3, cs		C v = abs(u-v), even	1
	csel	u0, v0, u0, cs		C u = min(u,v)		1
	lsr	v0, x3, x12		C			3
	subs	x3, u0, v0		C			4
	b.ne	L(top)			C

L(end):	ret
EPILOGUE()
