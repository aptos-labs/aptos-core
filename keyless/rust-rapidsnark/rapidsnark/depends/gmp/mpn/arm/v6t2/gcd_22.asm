dnl  ARM v6t2 mpn_gcd_22.

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


C	     cycles/bit (approx)
C StrongARM	 -
C XScale	 -
C Cortex-A5	10.1
C Cortex-A7	 9.1
C Cortex-A8	 6.3
C Cortex-A9	 ?
C Cortex-A12	 7.7
C Cortex-A15	 5.7
C Cortex-A17	 ?
C Cortex-A53	 7.0


define(`gp',    `r0')

define(`u1',    `r1')
define(`u0',    `r2')
define(`v1',    `r3')
define(`v0',    `r4')

define(`t0',    `r5')
define(`t1',    `r6')
define(`cnt',   `r7')

ASM_START()
PROLOGUE(mpn_gcd_22)
	push	{ r4-r7 }

	ldr	v0, [sp,#16]		C

L(top):	subs	t0, u0, v0		C 0 7
	beq	L(lowz)
	sbcs	t1, u1, v1		C 1 8

	rbit	cnt, t0			C 1

	negcc	t0, t0
	mvncc	t1, t1
L(bck):	movcc	v0, u0
	movcc	v1, u1

	clz	cnt, cnt		C 2
	rsb	r12, cnt, #32		C 3

	lsr	u0, t0, cnt		C 3
	lsl	r12, t1, r12		C 4
	lsr	u1, t1, cnt		C 3
	orr	u0, u0, r12		C 5

	orrs	r12, u1, v1
	bne	L(top)


	str	r12, [gp,#4]		C high result limb <= 0

	mov	r6, gp
	mov	r0, u0			C pass 1st argument
	mov	r1, v0			C pass 2nd argument
	mov	r7, r14			C preserve link register
	bl	mpn_gcd_11
	str	r0, [r6,#0]
	mov	r14, r7
	pop	{ r4-r7 }
	bx	r14

L(lowz):C We come here when v0 - u0 = 0
	C 1. If v1 - u1 = 0, then gcd is u = v.
	C 2. Else compute gcd_21({v1,v0}, |u1-v1|)
	subs	t0, u1, v1
	beq	L(end)
	mov	t1, #0
	rbit	cnt, t0			C 1
	negcc	t0, t0
	b	L(bck)

L(end):	str	v0, [gp,#0]
	str	v1, [gp,#4]
	pop	{ r4-r7 }
	bx	r14
EPILOGUE()
