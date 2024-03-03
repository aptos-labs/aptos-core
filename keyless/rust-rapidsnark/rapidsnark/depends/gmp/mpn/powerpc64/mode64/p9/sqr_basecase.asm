dnl  Power9 mpn_sqr_basecase.

dnl  Copyright 1999-2001, 2003-2006, 2008, 2017-2018 Free Software Foundation,
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

C                  cycles/limb
C POWER3/PPC630          -
C POWER4/PPC970          -
C POWER5                 -
C POWER6                 -
C POWER7                 -
C POWER8                 -
C POWER9                 1.62

C TODO
C  * Completely separate evn and odd code into two outer loops. Also consider
C    unrolling these two outer loops and thereby eliminate all branches.
C  * Avoid the reloading of u1 before every loop start.
C  * Reduce register usage.
C  * Consider getting rid of cy and instead load 3 u limbs, use addc+adde+adde.
C  * Consider skewing conditional adjustments to allow mask creation with subfe
C    like in the un=3 code. It might streamline the adjustments (or not).

C INPUT PARAMETERS
define(`rp', `r3')
define(`up', `r4')
define(`un', `r5')

define(`u0', `r0')
define(`u1', `r7')
define(`rp2', `r24')
define(`up2', `r25')
define(`cy',  `r6')

define(`LSHU1U0',`
	addc	u0, u0, u0
	adde	u1, u1, u1
	li	cy, 0
	addze	cy, cy
')
define(`LSHU1U',`
	addc	u0, u0, u0
	add	u0, u0, cy
	adde	u1, u1, u1
	li	cy, 0
	addze	cy, cy
')
define(`LSHU1UF',`
	addc	u0, u0, u0
	add	u0, u0, cy
	adde	u1, u1, u1
')
define(`LSHU1UHF',`
	add	u0, u0, u0
	add	u0, u0, cy
')
C These are cleverer replacements, but they tend to leave CA set, disturbing
C the main accumulation code! Breaking that false dependency might have a
C positive performance impact. Note that the subfe here results in a mask for
C our adjustments.
define(`xLSHU1U0',`
	addc	u0, u0, u0
	adde	u1, u1, u1
	subfe	cy, cy, cy
')
define(`xLSHU1U',`
	subfic	cy, cy, 0
	adde	u0, u0, u0
	adde	u1, u1, u1
	subfe	cy, cy, cy
')
define(`xLSHU1U',`
	subfic	cy, cy, 0
	adde	u0, u0, u0
')

ASM_START()
PROLOGUE(mpn_sqr_basecase)
	ld	r0, 0(up)	C n = 1
	mulld	r8, r0, r0	C weight 0
	mulhdu	r9, r0, r0	C weight 1
	std	r8, 0(rp)
	cmpdi	cr0, un, 2
	bge	cr0, L(ge2)
	std	r9, 8(rp)
	blr

L(ge2):	bgt	cr0, L(gt2)
	ld	r6, 8(up)
	mulld	r10, r6, r6	C u1 * u1
	mulhdu	r11, r6, r6	C u1 * u1
	mulld	r4, r6, r0	C u1 * u0
	mulhdu	r5, r6, r0	C u1 * u0
	addc	r4, r4, r4
	adde	r5, r5, r5
	addze	r11, r11
	addc	r9, r9, r4
	adde	r10, r10, r5
	addze	r11, r11
	std	r9, 8(rp)
	std	r10, 16(rp)
	std	r11, 24(rp)
	blr

L(gt2):	cmpdi	cr0, un, 3
	bgt	cr0, L(gt3)
	std	r30, -16(r1)
	std	r31, -8(r1)
	subfo	r12, r12, r12		C clear OV (and result register)
	ld	r8, 8(r4)
	mulld	r5, r8, r8		C W2
	mulhdu	r10, r8, r8		C W3
	sradi	r11, u0, 63		C CAUTION: clobbers CA
	and	r11, r11, r8		C W3
	addc	u0, u0, u0
	adde	u1, r8, r8
	subfe	r6, r6, r6		C	mask
	ld	r4, 16(r4)		C W2
	mulld	r12, r8, u0		C W1	u1 x u0
	mulhdu	r8, r8, u0		C W2	u1 x u0
	maddld(	r31, r4, u0, r11)	C W2
	maddhdu(r30, r4, u0, r11)	C W3
	andc	r6, r4, r6		C W4
	addc	r9, r12, r9		C W1
	std	r9, 8(rp)		C W1
	mulld	r9, r4, u1		C W3
	mulhdu	r11, r4, u1		C W4
	addex(	r5, r5, r8, 0)		C W2
	adde	r5, r31, r5		C W2
	std	r5, 16(rp)		C W2
	maddld(	r5, r4, r4, r6)		C W4	u2^2
	maddhdu(r6, r4, r4, r6)		C W5	u2^2
	addex(	r9, r9, r30, 0)		C W3
	adde	r9, r9, r10		C W3
	std	r9, 24(rp)		C W3
	adde	r5, r5, r11		C W4
	addze	r6, r6			C W5
	li	r8, 0
	addex(	r5, r5, r8, 0)		C W4
	std	r5, 32(rp)		C W4
	addex(	r6, r6, r8, 0)		C W5
	std	r6, 40(rp)		C W5
	ld	r30, -16(r1)
	ld	r31, -8(r1)
	blr

L(gt3):	std	r22, -80(r1)
	std	r23, -72(r1)
	std	r24, -64(r1)
	std	r25, -56(r1)
	std	r26, -48(r1)
	std	r27, -40(r1)
	std	r28, -32(r1)
	std	r29, -24(r1)
	std	r30, -16(r1)
	std	r31, -8(r1)

	mr	rp2, rp
	mr	up2, up
	addi	r22, un, -1		C count for loop FIXME: Adjust
	subfo	r0, r0, r0		C clear OV (and r0)
	rldicl	r0, un, 0, 63		C r0 = un & 1
	cmpdi	cr7, r0, 0

	ld	u0, 0(up2)
	ld	u1, 8(up2)

	cmpdi	cr5, r22, 4
	srdi	r31, r22, 2
	addi	r22, r22, -2
	mtctr	r31

	beq	cr7, L(m2_evn)
L(m2_odd):
	rldicl.	r31, r22, 63, 63	C r22 & 2
	mulld	r23, u0, u0
	mulhdu	r12, u0, u0
	mulld	r5, u1, u1
	mulhdu	r10, u1, u1

	sradi	r11, u0, 63
	and	r11, r11, u1

	LSHU1U0

	ld	r8, 8(up2)
	ld	r9, 16(up2)
	mulld	r28, r8, u0		C W	u1 x u0
	mulhdu	r31, r8, u0		C W	u1 x u0
	std	r23, 0(rp2)

	bne	cr0, L(m2_11)
L(m2_01):
	addi	up, up2, 16
	addi	rp, rp2, 0
	b	L(m2_lo2)
L(m2_11):
	addi	up, up2, 0
	addi	rp, rp2, -16
	b	L(m2_lo0)

L(m2_evn):
	rldicl.	r31, r22, 63, 63	C r22 & 2
	mulld	r23, u0, u0
	mulhdu	r5, u0, u0
	mulld	r12, u1, u1
	mulhdu	r11, u1, u1

	sradi	r10, u0, 63
	and	r10, r10, u1

	LSHU1U0

	ld	r9, 8(up2)
	ld	r8, 16(up2)
	mulld	r29, r9, u0		C W	u1 x u0
	mulhdu	r30, r9, u0		C W	u1 x u0
	std	r23, 0(rp2)

	beq	cr0, L(m2_10)
L(m2_00):
	addi	up, up2, 8
	addi	rp, rp2, -8
	b	L(m2_lo1)
L(m2_10):
	addi	up, up2, 24
	addi	rp, rp2, 8
	ble	cr5, L(m2_end)

L(m2_top):
	ld	r9, 0(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	mulld	r5, r8, u1
	mulhdu	r10, r8, u1
	addex(	r12, r12, r30, 0)
L(m2_lo2):
	ld	r8, 8(up)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	std	r12, 8(rp)
	mulld	r12, r9, u1
	mulhdu	r11, r9, u1
	addex(	r5, r5, r31, 0)
L(m2_lo1):
	ld	r9, 16(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	std	r5, 16(rp)
	mulld	r5, r8, u1
	mulhdu	r10, r8, u1
	addex(	r12, r12, r30, 0)
L(m2_lo0):
	ld	r8, 24(up)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	std	r12, 24(rp)
	mulld	r12, r9, u1
	mulhdu	r11, r9, u1
	addex(	r5, r5, r31, 0)
	addi	up, up, 32
	addi	rp, rp, 32
	bdnz	L(m2_top)

L(m2_end):
	ld	r9, 0(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	mulld	r5, r8, u1
	mulhdu	r10, r8, u1
	b	L(cj)			C jump to addmul_2 tail

L(outer):
	addi	up2, up2, 16
	addi	rp2, rp2, 32

	ld	u0, 0(up2)
	ld	u1, 8(up2)

	cmpdi	cr5, r22, 4
	srdi	r31, r22, 2
	addi	r22, r22, -2
	mtctr	r31

	ld	r26, 0(rp2)
	ld	r27, 16(rp2)

	rldicl.	r31, r22, 63, 63	C r22 & 2
	beq	cr7, L(evn)

L(odd):	maddld(	r23, u0, u0, r26)	C W	u2^2
	maddhdu(r12, u0, u0, r26)	C W	u2^2
	maddld(	r5, u1, u1, r27)	C W	u3^2
	maddhdu(r10, u1, u1, r27)	C W	u3^2
	ld	r26, 8(rp2)

	ld	r8, -8(up2)
	sradi	r8, r8, 63		C CAUTION: clobbers CA
	and	r8, r8, u0
	sradi	r11, u0, 63		C CAUTION: clobbers CA
	and	r11, r11, u1

	LSHU1U

	addc	r23, r23, r8

	ld	r8, 8(up2)
	ld	r9, 16(up2)
	maddld(	r28, r8, u0, r26)	C W	u3 x u2
	maddhdu(r31, r8, u0, r26)	C W	u3 x u2
	ld	r26, 24(rp2)
	std	r23, 0(rp2)		C W0

	bne	cr0, L(11)
L(01):
	addi	up, up2, 16
	addi	rp, rp2, 0
	b	L(lo2)
L(11):
	addi	up, up2, 0
	addi	rp, rp2, -16
	b	L(lo0)

L(evn):	maddld(	r23, u0, u0, r26)	C W	u2^2
	maddhdu(r5, u0, u0, r26)	C W	u2^2
	maddld(	r12, u1, u1, r27)	C W	u3^2
	maddhdu(r11, u1, u1, r27)	C W	u3^2
	ld	r27, 8(rp2)

	ld	r9, -8(up2)
	sradi	r9, r9, 63		C CAUTION: clobbers CA
	and	r9, r9, u0
	sradi	r10, u0, 63		C CAUTION: clobbers CA
	and	r10, r10, u1

	LSHU1U

	addc	r23, r23, r9

	ld	r9, 8(up2)
	ld	r8, 16(up2)
	maddld(	r29, r9, u0, r27)	C W	u3 x u2
	maddhdu(r30, r9, u0, r27)	C W	u3 x u2
	ld	r27, 24(rp2)
	std	r23, 0(rp2)		C W0

	beq	cr0, L(10)
L(00):
	addi	up, up2, 8
	addi	rp, rp2, -8
	b	L(lo1)
L(10):
	addi	up, up2, 24
	addi	rp, rp2, 8
	ble	cr5, L(end)

L(top):	ld	r9, 0(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	ld	r26, 24(rp)
	std	r5, 0(rp)
	maddld(	r5, r8, u1, r27)
	maddhdu(r10, r8, u1, r27)
	addex(	r12, r12, r30, 0)
L(lo2):	ld	r8, 8(up)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	ld	r27, 32(rp)
	std	r12, 8(rp)
	maddld(	r12, r9, u1, r26)
	maddhdu(r11, r9, u1, r26)
	addex(	r5, r5, r31, 0)
L(lo1):	ld	r9, 16(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	ld	r26, 40(rp)
	std	r5, 16(rp)
	maddld(	r5, r8, u1, r27)
	maddhdu(r10, r8, u1, r27)
	addex(	r12, r12, r30, 0)
L(lo0):	ld	r8, 24(up)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	ld	r27, 48(rp)
	std	r12, 24(rp)
	maddld(	r12, r9, u1, r26)
	maddhdu(r11, r9, u1, r26)
	addex(	r5, r5, r31, 0)
	addi	up, up, 32
	addi	rp, rp, 32
	bdnz	L(top)

L(end):	ld	r9, 0(up)
	maddld(	r28, r8, u0, r10)
	maddhdu(r31, r8, u0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	maddld(	r5, r8, u1, r27)
	maddhdu(r10, r8, u1, r27)
L(cj):	addex(	r12, r12, r30, 0)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	std	r12, 8(rp)
	mulld	r12, r9, u1
	mulhdu	r11, r9, u1
	addex(	r5, r5, r31, 0)
	adde	r5, r29, r5
	std	r5, 16(rp)
	addex(	r12, r12, r30, 0)
	adde	r12, r12, r10
	std	r12, 24(rp)
	li	r4, 0
	addze	r5, r11
	addex(	r5, r5, r4, 0)
	std	r5, 32(rp)
	bgt	cr5, L(outer)

L(corner):
	ld	u0, 16(up2)
	ld	u1, 24(up2)
	ld	r26, 32(rp2)
	bne	cr7, L(corner_odd)

L(corner_evn):
	ld	r27, 40(rp2)
	maddld(	r23, u0, u0, r26)	C W	u2^2
	maddhdu(r5, u0, u0, r26)	C W	u2^2
	mulld	r12, u1, u1		C W	u3^2
	mulhdu	r11, u1, u1		C W	u3^2

	ld	r9, 8(up2)
	sradi	r9, r9, 63		C CAUTION: clobbers CA
	and	r9, r9, u0
	sradi	r10, u0, 63		C CAUTION: clobbers CA
	and	r10, r10, u1

	LSHU1UHF

	addc	r23, r23, r9

	ld	r9, 24(up2)
	maddld(	r29, r9, u0, r27)	C W	u3 x u2
	maddhdu(r30, r9, u0, r27)	C W	u3 x u2
	std	r23, 32(rp2)
	adde	r5, r29, r5
	std	r5, 40(rp2)
	addex(	r12, r12, r30, 0)
	adde	r12, r12, r10		C W	FIXME can this co?
	std	r12, 48(rp2)
	li	r4, 0
	addex(	r5, r11, r4, 0)
	addze	r5, r5
	std	r5, 56(rp2)
	b	L(ret)

L(corner_odd):
	ld	r27, 48(rp2)
	maddld(	r23, u0, u0, r26)	C W	u2^2
	maddhdu(r12, u0, u0, r26)	C W	u2^2
	maddld(	r5, u1, u1, r27)	C W	u3^2
	maddhdu(r10, u1, u1, r27)	C W	u3^2
	ld	r26, 40(rp2)

	ld	r8, 8(up2)
	sradi	r8, r8, 63		C CAUTION: clobbers CA
	and	r8, r8, u0
	sradi	r11, u0, 63		C CAUTION: clobbers CA
	and	r11, r11, u1

	LSHU1UF

	addc	r23, r23, r8

	ld	r8, 24(up2)
	ld	r9, 32(up2)
	maddld(	r28, r8, u0, r26)	C W	u3 x u2
	maddhdu(r31, r8, u0, r26)	C W	u3 x u2
	std	r23, 32(rp2)
	maddld(	r29, r9, u0, r11)
	maddhdu(r30, r9, u0, r11)
	adde	r12, r28, r12
	std	r12, 40(rp2)
	mulld	r12, r9, u1
	mulhdu	r11, r9, u1
	addex(	r5, r5, r31, 0)
	adde	r5, r29, r5
	std	r5, 48(rp2)
	addex(	r12, r12, r30, 0)
	adde	r12, r12, r10
	std	r12, 56(rp2)
	mulld	r23, r9, r9		C W	u2^2
	mulhdu	r12, r9, r9		C W	u2^2
	adde	r23, r23, r11
	addze	r12, r12
	sradi	r4, r8, 63		C CAUTION: clobbers CA
	and	r4, r4, r9
	addex(	r23, r23, r4, 0)
	std	r23, 64(rp2)
	li	r4, 0
	addex(	r12, r12, r4, 0)
	std	r12, 72(rp2)

L(ret):	ld	r22, -80(r1)
	ld	r23, -72(r1)
	ld	r24, -64(r1)
	ld	r25, -56(r1)
	ld	r26, -48(r1)
	ld	r27, -40(r1)
	ld	r28, -32(r1)
	ld	r29, -24(r1)
	ld	r30, -16(r1)
	ld	r31, -8(r1)
	blr
EPILOGUE()
ASM_END()
