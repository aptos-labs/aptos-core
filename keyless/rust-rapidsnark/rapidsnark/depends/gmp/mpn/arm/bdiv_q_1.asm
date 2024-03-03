dnl  ARM v4 mpn_bdiv_q_1, mpn_pi1_bdiv_q_1 -- Hensel division by 1-limb divisor.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2012, 2017 Free Software Foundation, Inc.

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
C 1176		13	18
C Cortex-A5	 8	12
C Cortex-A7	10.5	18
C Cortex-A8	14	15
C Cortex-A9	10	12		not measured since latest edits
C Cortex-A15	 9	 9
C Cortex-A53	14	20

C Architecture requirements:
C v5	-
C v5t	-
C v5te	-
C v6	-
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
define(`tnc', `r8')

ASM_START()
PROLOGUE(mpn_bdiv_q_1)
	tst	d, #1
	push	{r6-r11}
	mov	cnt, #0
	bne	L(inv)

C count trailing zeros
	movs	r10, d, lsl #16
	moveq	d, d, lsr #16
	moveq	cnt, #16
	tst	d, #0xff
	moveq	d, d, lsr #8
	addeq	cnt, cnt, #8
	LEA(	r10, ctz_tab)
	and	r11, d, #0xff
	ldrb	r10, [r10, r11]
	mov	d, d, lsr r10
	add	cnt, cnt, r10

C binvert limb
L(inv):	LEA(	r10, binvert_limb_table)
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

L(pi1):	ldr	r11, [up], #4		C up[0]
	cmp	cnt, #0
	mov	cy, #0
	bne	L(unorm)

L(norm):
	subs	n, n, #1		C set carry as side-effect
	beq	L(edn)

	ALIGN(16)
L(tpn):	sbcs	cy, r11, cy
	ldr	r11, [up], #4
	sub	n, n, #1
	mul	r9, r10, cy
	tst	n, n
	umull	r12, cy, d, r9
	str	r9, [rp], #4
	bne	L(tpn)

L(edn):	sbc	cy, r11, cy
	mul	r9, r10, cy
	str	r9, [rp]
	pop	{r6-r11}
	return	r14

L(unorm):
	rsb	tnc, cnt, #32
	mov	r11, r11, lsr cnt
	subs	n, n, #1		C set carry as side-effect
	beq	L(edu)

	ALIGN(16)
L(tpu):	ldr	r12, [up], #4
	orr	r9, r11, r12, lsl tnc
	mov	r11, r12, lsr cnt
	sbcs	cy, r9, cy		C critical path ->cy->cy->
	sub	n, n, #1
	mul	r9, r10, cy		C critical path ->cy->r9->
	tst	n, n
	umull	r12, cy, d, r9		C critical path ->r9->cy->
	str	r9, [rp], #4
	bne	L(tpu)

L(edu):	sbc	cy, r11, cy
	mul	r9, r10, cy
	str	r9, [rp]
	pop	{r6-r11}
	return	r14
EPILOGUE()

	RODATA
ctz_tab:
	.byte	8,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	5,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	6,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	5,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	7,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	5,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	6,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
	.byte	5,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0,4,0,1,0,2,0,1,0,3,0,1,0,2,0,1,0
