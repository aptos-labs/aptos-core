dnl  Power9 mpn_mul_basecase.

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
C  * Check if (inner) loop alignment affects performance.
C  * Could we schedule loads less in addmul_2/mul_2? That would save some regs
C    and make the tail code more manageable.
C  * Postpone some register saves to main loop.
C  * Perhaps write more small operands (3x1, 3x2, 3x3) code.
C  * Consider restoring rp,up after loop using arithmetic, eliminating rp2, up2.
C    On the other hand, the current rp,up restore register are useful for OSP.
C  * Do OSP. This should save a lot with the current deep addmul_2 pipeline.

C INPUT PARAMETERS
define(`rp', `r3')
define(`up', `r4')
define(`un', `r5')
define(`vp', `r6')
define(`vn', `r7')

define(`v0', `r0')
define(`v1', `r7')
define(`rp2', `r24')
define(`up2', `r25')

ASM_START()
PROLOGUE(mpn_mul_basecase)
	cmpdi	cr0, un, 2
	bgt	cr0, L(un_gt2)
	cmpdi	cr6, vn, 1
	ld	r7, 0(vp)
	ld	r5, 0(up)
	mulld	r8, r5, r7	C weight 0
	mulhdu	r9, r5, r7	C weight 1
	std	r8, 0(rp)
	beq	cr0, L(2x)
	std	r9, 8(rp)
	blr
	ALIGN(16)
L(2x):	ld	r0, 8(up)
	mulld	r8, r0, r7	C weight 1
	mulhdu	r10, r0, r7	C weight 2
	addc	r9, r9, r8
	addze	r10, r10
	bne	cr6, L(2x2)
	std	r9, 8(rp)
	std	r10, 16(rp)
	blr
	ALIGN(16)
L(2x2):	ld	r6, 8(vp)
	mulld	r8, r5, r6	C weight 1
	mulhdu	r11, r5, r6	C weight 2
	addc	r9, r9, r8
	std	r9, 8(rp)
	adde	r11, r11, r10
	mulld	r12, r0, r6	C weight 2
	mulhdu	r0, r0, r6	C weight 3
	addze	r0, r0
	addc	r11, r11, r12
	addze	r0, r0
	std	r11, 16(rp)
	std	r0, 24(rp)
	blr

L(un_gt2):
	std	r22, -80(r1)
	std	r23, -72(r1)
	std	r24, -64(r1)
	std	r25, -56(r1)
	std	r26, -48(r1)
	std	r27, -40(r1)
	std	r28, -32(r1)
	std	r29, -24(r1)
	std	r30, -16(r1)
	std	r31, -8(r1)
	mr	rp2, r3			C rp
	mr	up2, r4			C up
	srdi	r22, r5, 2		C un
	subfic	r23, r7, 0		C -vn, clear CA
	subfo	r0, r0, r0		C clear OV (and r0)

	cmpdi	cr6, un, 3
	rldicl	r0, un, 0, 63		C r0 = un & 1
	cmpdi	cr7, r0, 0
	rldicl	r0, un, 63, 63		C FIXME: unused for vn = 1
	cmpdi	cr5, r0, 0		C FIXME: unused for vn = 1

	ld	v0, 0(vp)
	rldicl.	r9, vn, 0, 63
	beq	cr0, L(vn_evn)

L(vn_odd):
	addi	r10, un, -2
	ld	r5, 0(up)
	srdi	r10, r10, 1
	mtctr	r10
	bne	cr7, L(m1_b1)

L(m1_b0):
	ld	r10, 8(up)
	mulld	r9, r5, v0
	mulhdu	r11, r5, v0
	ld	r12, 16(up)
	mulld	r8, r10, v0
	mulhdu	r5, r10, v0
	addi	rp, rp, -8
	b	L(m1_mid)

L(m1_b1):
	ld	r12, 8(up)
	mulld	r8, r5, v0
	mulhdu	r5, r5, v0
	ld	r10, 16(up)
	mulld	r9, r12, v0
	mulhdu	r11, r12, v0
	addi	up, up, 8
	beq	cr6, L(m1_end)		C jump taken means un = 3, vn = {1,3}

	ALIGN(16)
L(m1_top):
	ld	r12, 16(up)
	std	r8, 0(rp)
	adde	r9, r5, r9
	mulld	r8, r10, v0
	mulhdu	r5, r10, v0
L(m1_mid):
	ld	r10, 24(up)
	std	r9, 8(rp)
	adde	r8, r11, r8
	mulld	r9, r12, v0
	mulhdu	r11, r12, v0
	addi	rp, rp, 16
	addi	up, up, 16
	bdnz	L(m1_top)

L(m1_end):
	std	r8, 0(rp)
	mulld	r8, r10, v0
	adde	r9, r5, r9
	mulhdu	r5, r10, v0
	std	r9, 8(rp)
	adde	r8, r11, r8
	std	r8, 16(rp)
	addze	r10, r5
	std	r10, 24(rp)

	addi	rp2, rp2, 8
	addi	vp, vp, 8
	addic.	r23, r23, 1
	b	L(do_outer)

L(vn_evn):
	ld	v1, 8(vp)
	addi	r23, r23, 2
	mtctr	r22
	bne	cr7, L(m2_bx1)

L(m2_bx0):
	ld	r8, 0(up)
	ld	r9, 8(up)
	li	r11, 0
	mulld	r28, r8, v0
	mulhdu	r31, r8, v0
	mulld	r5, r8, v1
	mulhdu	r10, r8, v1
	li	r12, 0
	bne	cr5, L(m2_b10)

L(m2_b00):
	addi	up, up, -8
	addi	rp, rp, -24
	b	L(m2_lo0)

L(m2_b10):
	addi	up, up, 8
	addi	rp, rp, -8
	b	L(m2_lo2)

L(m2_bx1):
	ld	r9, 0(up)
	ld	r8, 8(up)
	li	r10, 0
	mulld	r29, r9, v0
	mulhdu	r30, r9, v0
	mulld	r12, r9, v1
	mulhdu	r11, r9, v1
	li	r5, 0
	bne	cr5, L(m2_b11)

L(m2_b01):
	addi	rp, rp, -16
	b	L(m2_lo1)
L(m2_b11):
	addi	up, up, 16
	beq	cr6, L(m2_end)		C taken means un = 3, vn = 2. We're done.

L(m2_top):
	ld	r9, 0(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	mulld	r5, r8, v1
	mulhdu	r10, r8, v1
	addex(	r12, r12, r30, 0)
L(m2_lo2):
	ld	r8, 8(up)
	maddld(	r29, r9, v0, r11)
	maddhdu(r30, r9, v0, r11)
	adde	r12, r28, r12
	std	r12, 8(rp)
	mulld	r12, r9, v1
	mulhdu	r11, r9, v1
	addex(	r5, r5, r31, 0)
L(m2_lo1):
	ld	r9, 16(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	std	r5, 16(rp)
	mulld	r5, r8, v1
	mulhdu	r10, r8, v1
	addex(	r12, r12, r30, 0)
L(m2_lo0):
	ld	r8, 24(up)
	maddld(	r29, r9, v0, r11)
	maddhdu(r30, r9, v0, r11)
	adde	r12, r28, r12
	std	r12, 24(rp)
	mulld	r12, r9, v1
	mulhdu	r11, r9, v1
	addex(	r5, r5, r31, 0)
	addi	up, up, 32
	addi	rp, rp, 32
	bdnz	L(m2_top)

L(m2_end):
	ld	r9, 0(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	mulld	r5, r8, v1
	mulhdu	r10, r8, v1
	b	L(cj)

L(outer):
	ld	v0, 0(vp)
	ld	v1, 8(vp)
	addi	r23, r23, 2
	mtctr	r22
	bne	cr7, L(bx1)

L(bx0):	ld	r26, 0(rp2)
	ld	r8, 0(up2)
	ld	r11, 8(rp2)
	ld	r9, 8(up2)
	maddld(	r28, r8, v0, r26)
	maddhdu(r31, r8, v0, r26)
	ld	r26, 16(rp2)
	mulld	r5, r8, v1
	mulhdu	r10, r8, v1
	li	r12, 0
	bne	cr5, L(b10)

L(b00):	addi	up, up2, -8
	addi	rp, rp2, -24
	b	L(lo0)

L(b10):	addi	up, up2, 8
	addi	rp, rp2, -8
	b	L(lo2)

L(bx1):	ld	r27, 0(rp2)
	ld	r9, 0(up2)
	ld	r10, 8(rp2)
	ld	r8, 8(up2)
	maddld(	r29, r9, v0, r27)
	maddhdu(r30, r9, v0, r27)
	ld	r27, 16(rp2)
	mulld	r12, r9, v1
	mulhdu	r11, r9, v1
	li	r5, 0
	bne	cr5, L(b11)

L(b01):	addi	up, up2, 0
	addi	rp, rp2, -16
	b	L(lo1)
L(b11):	addi	up, up2, 16
	addi	rp, rp2, 0
	beq	cr6, L(end)		C taken means un = 3, vn = 3. We're done.

L(top):	ld	r9, 0(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	ld	r26, 24(rp)
	std	r5, 0(rp)
	maddld(	r5, r8, v1, r27)
	maddhdu(r10, r8, v1, r27)
	addex(	r12, r12, r30, 0)
L(lo2):	ld	r8, 8(up)
	maddld(	r29, r9, v0, r11)
	maddhdu(r30, r9, v0, r11)
	adde	r12, r28, r12
	ld	r27, 32(rp)
	std	r12, 8(rp)
	maddld(	r12, r9, v1, r26)
	maddhdu(r11, r9, v1, r26)
	addex(	r5, r5, r31, 0)
L(lo1):	ld	r9, 16(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	ld	r26, 40(rp)
	std	r5, 16(rp)
	maddld(	r5, r8, v1, r27)
	maddhdu(r10, r8, v1, r27)
	addex(	r12, r12, r30, 0)
L(lo0):	ld	r8, 24(up)
	maddld(	r29, r9, v0, r11)
	maddhdu(r30, r9, v0, r11)
	adde	r12, r28, r12
	ld	r27, 48(rp)
	std	r12, 24(rp)
	maddld(	r12, r9, v1, r26)
	maddhdu(r11, r9, v1, r26)
	addex(	r5, r5, r31, 0)
	addi	up, up, 32
	addi	rp, rp, 32
	bdnz	L(top)

L(end):	ld	r9, 0(up)
	maddld(	r28, r8, v0, r10)
	maddhdu(r31, r8, v0, r10)
	adde	r5, r29, r5
	std	r5, 0(rp)
	maddld(	r5, r8, v1, r27)
	maddhdu(r10, r8, v1, r27)
L(cj):	addex(	r12, r12, r30, 0)
	maddld(	r29, r9, v0, r11)
	maddhdu(r30, r9, v0, r11)
	adde	r12, r28, r12
	std	r12, 8(rp)
	mulld	r12, r9, v1
	mulhdu	r11, r9, v1
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

	cmpdi	cr0, r23, 0
	addi	rp2, rp2, 16
	addi	vp, vp, 16
L(do_outer):
	bne	cr0, L(outer)
L(ret):
	ld	r22, -80(r1)
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
