dnl  ARM v6 mpn_bdiv_q_1, mpn_pi1_bdiv_q_1 -- Hensel division by 1-limb divisor.
dnl  This is v6 code but it runs well on just the v7a Cortex-A8, A9, and A15.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2012, 2013, 2017 Free Software Foundation, Inc.

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

C               cycles/limb
C               norm   unorm
C 1176		 -	 -
C Cortex-A5	 9	13
C Cortex-A7	12	18
C Cortex-A8	13	14
C Cortex-A9	 9	10		not measured since latest edits
C Cortex-A15	 7	 7
C Cortex-A53	16	24

C Architecture requirements:
C v5	-
C v5t	clz
C v5te	-
C v6	umaal
C v6t2	-
C v7a	-

define(`rp',  `r0')
define(`up',  `r1')
define(`n',   `r2')
define(`d',   `r3')
define(`di_arg',  `sp[0]')		C	just mpn_pi1_bdiv_q_1
define(`cnt_arg', `sp[4]')		C	just mpn_pi1_bdiv_q_1

define(`cy',  `r7')
define(`cnt', `r6')
define(`tnc', `r4')

ASM_START()
PROLOGUE(mpn_bdiv_q_1)
	push	{r6-r11}

	rsb	r10, d, #0
	and	r10, r10, d
	clz	r10, r10
	rsbs	cnt, r10, #31		C count_trailing_zeros
	mov	d, d, lsr cnt

C binvert limb
	LEA(	r10, binvert_limb_table)
	and	r12, d, #254
	ldrb	r10, [r10, r12, lsr #1]
	mul	r12, r10, r10
	mul	r12, d, r12
	rsb	r12, r12, r10, lsl #1
	mul	r10, r12, r12
	mul	r10, d, r10
	rsb	r10, r10, r12, lsl #1	C r10 = inverse
	b	L(pi1)
EPILOGUE()

PROLOGUE(mpn_pi1_bdiv_q_1)
	push	{r6-r11}

	ldr	cnt, [sp, #28]
	ldr	r10, [sp, #24]
	cmp	cnt, #0

L(pi1):	ldr	r11, [up], #4		C up[0]
	mov	cy, #0
	rsb	r8, r10, #0		C r8 = -inverse
	bne	L(unorm)

L(norm):
	subs	n, n, #1
	mul	r11, r11, r10
	beq	L(edn)

	ALIGN(16)
L(tpn):	ldr	r9, [up], #4
	mov	r12, #0
	str	r11, [rp], #4
	umaal	r12, cy, r11, d
	mul	r11, r9, r10
	mla	r11, cy, r8, r11
	subs	n, n, #1
	bne	L(tpn)

L(edn):	str	r11, [rp]
	pop	{r6-r11}
	bx	r14

L(unorm):
	push	{r4-r5}
	rsb	tnc, cnt, #32
	mov	r5, r11, lsr cnt
	subs	n, n, #1
	beq	L(ed1)

	ldr	r12, [up], #4
	orr	r9, r5, r12, lsl tnc
	mov	r5, r12, lsr cnt
	mul	r11, r9, r10
	subs	n, n, #1
	beq	L(edu)

	ALIGN(16)
L(tpu):	ldr	r12, [up], #4
	orr	r9, r5, r12, lsl tnc
	mov	r5, r12, lsr cnt
	mov	r12, #0
	str	r11, [rp], #4
	umaal	r12, cy, r11, d
	mul	r11, r9, r10
	mla	r11, cy, r8, r11
	subs	n, n, #1
	bne	L(tpu)

L(edu):	str	r11, [rp], #4
	mov	r12, #0
	umaal	r12, cy, r11, d
	mul	r11, r5, r10
	mla	r11, cy, r8, r11
	str	r11, [rp]
	pop	{r4-r11}
	bx	r14

L(ed1):	mul	r11, r5, r10
	str	r11, [rp]
	pop	{r4-r11}
	bx	r14
EPILOGUE()
