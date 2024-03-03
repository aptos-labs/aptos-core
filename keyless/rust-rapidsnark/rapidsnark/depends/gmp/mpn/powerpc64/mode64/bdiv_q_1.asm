dnl  PowerPC-64 mpn_bdiv_q_1, mpn_pi1_bdiv_q_1 -- Hensel division by 1-limb
dnl  divisor.

dnl  Copyright 2006, 2010, 2017 Free Software Foundation, Inc.

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

C			cycles/limb
C			norm	unorm
C POWER3/PPC630	       13-19
C POWER4/PPC970		16
C POWER5		16	16
C POWER6		37	46
C POWER7		12	12
C POWER8		12	12

C INPUT PARAMETERS
define(`rp', `r3')
define(`up', `r4')
define(`n',  `r5')
define(`d',  `r6')
define(`di', `r7')
define(`cnt',`r8')

define(`tnc',`r10')

ASM_START()

EXTERN(binvert_limb_table)

PROLOGUE(mpn_bdiv_q_1,toc)
	addi	r7, n, -1
	cmpdi	cr1, n, 1
	ld	r12, 0(up)
	li	cnt, 0
	neg	r0, d
	and	r0, d, r0
	cntlzd	r0, r0
	subfic	cnt, r0, 63
	srd	d, d, cnt
L(7):
	mtctr	r7
	LEA(	r10, binvert_limb_table)
	rldicl	r11, d, 63, 57
	lbzx	r0, r10, r11
	mulld	r9, r0, r0
	sldi	r0, r0, 1
	mulld	r9, d, r9
	subf	r0, r9, r0
	mulld	r10, r0, r0
	sldi	r0, r0, 1
	mulld	r10, d, r10
	subf	r0, r10, r0
	mulld	r9, r0, r0
	sldi	r0, r0, 1
	mulld	r9, d, r9
	subf	di, r9, r0		C di = 1/d mod 2^64
ifdef(`AIX',
`	C For AIX it is not clear how to jump into another function.
	b	.mpn_pi1_bdiv_q_1
',`
	C For non-AIX, dispatch into the pi1 variant.
	bne	cr0, L(norm)
	b	L(unorm)
')
EPILOGUE()

PROLOGUE(mpn_pi1_bdiv_q_1)
	cmpdi	cr0, cnt, 0
	ld	r12, 0(up)
	addic	r0, n, -1		C set carry as side effect
	cmpdi	cr1, n, 1
	mtctr	r0
	beq	cr0, L(norm)

L(unorm):
	subfic	tnc, cnt, 64		C set carry as side effect
	li	r5, 0
	srd	r11, r12, cnt
	beq	cr1, L(ed1)

	ALIGN(16)
L(tpu):	ld	r12, 8(up)
	nop
	addi	up, up, 8
	sld	r0, r12, tnc
	or	r11, r11, r0
	subfe	r9, r5, r11
	srd	r11, r12, cnt
	mulld	r0, di, r9
	mulhdu	r5, r0, d
	std	r0, 0(rp)
	addi	rp, rp, 8
	bdnz	L(tpu)

	subfe	r11, r5, r11
L(ed1):	mulld	r0, di, r11
	std	r0, 0(rp)
	blr

	ALIGN(16)
L(norm):
	mulld	r11, r12, di
	mulhdu	r5, r11, d
	std	r11, 0(rp)
	beqlr	cr1

	ALIGN(16)
L(tpn):	ld	r9, 8(up)
	addi	up, up, 8
	subfe	r5, r5, r9
	mulld	r11, di, r5
	mulhdu	r5, r11, d	C result not used in last iteration
	std	r11, 8(rp)
	addi	rp, rp, 8
	bdnz	L(tpn)

	blr
EPILOGUE()
ASM_END()
