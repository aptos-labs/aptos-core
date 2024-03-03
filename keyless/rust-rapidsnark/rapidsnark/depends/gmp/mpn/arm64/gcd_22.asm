dnl  ARM v8a mpn_gcd_22.

dnl  Copyright 2019 Free Software Foundation, Inc.

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
C Cortex-A53	 7.26
C Cortex-A55	 ?
C Cortex-A57	 ?
C Cortex-A72	 5.72
C Cortex-A73	 6.43
C Cortex-A75	 ?
C Cortex-A76	 ?
C Cortex-A77	 ?


define(`u1',    `x0')
define(`u0',    `x1')
define(`v1',    `x2')
define(`v0',    `x3')

define(`t0',    `x5')
define(`t1',    `x6')
define(`cnt',   `x7')
define(`tnc',   `x8')

ASM_START()
PROLOGUE(mpn_gcd_22)

	ALIGN(16)
L(top):	subs	t0, u0, v0		C 0 6
	cbz	t0, L(lowz)
	sbcs	t1, u1, v1		C 1 7

	rbit	cnt, t0			C 1

	cneg	t0, t0, cc		C 2
	cinv	t1, t1, cc		C 2 u = |u - v|
L(bck):	csel	v0, v0, u0, cs		C 2
	csel	v1, v1, u1, cs		C 2 v = min(u,v)

	clz	cnt, cnt		C 2
	sub	tnc, xzr, cnt		C 3

	lsr	u0, t0, cnt		C 3
	lsl	x14, t1, tnc		C 4
	lsr	u1, t1, cnt		C 3
	orr	u0, u0, x14		C 5

	orr	x11, u1, v1
	cbnz	x11, L(top)


	subs	x4, u0, v0		C			0
	b.eq	L(end1)			C

	ALIGN(16)
L(top1):rbit	x12, x4			C			1,5
	clz	x12, x12		C			2
	csneg	x4, x4, x4, cs		C v = abs(u-v), even	1
	csel	u0, v0, u0, cs		C u = min(u,v)		1
	lsr	v0, x4, x12		C			3
	subs	x4, u0, v0		C			4
	b.ne	L(top1)			C
L(end1):mov	x0, u0
	mov	x1, #0
	ret

L(lowz):C We come here when v0 - u0 = 0
	C 1. If v1 - u1 = 0, then gcd is u = v.
	C 2. Else compute gcd_21({v1,v0}, |u1-v1|)
	subs	t0, u1, v1
	b.eq	L(end)
	mov	t1, #0
	rbit	cnt, t0			C 1
	cneg	t0, t0, cc		C 2
	b	L(bck)			C FIXME: make conditional

L(end):	mov	x0, v0
	mov	x1, v1
	ret
EPILOGUE()
