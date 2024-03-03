dnl  POWER9 mpn_addmul_1 and mpn_submul_1.

dnl  Copyright 2018 Free Software Foundation, Inc.

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

C                   mpn_addmul_1    mpn_submul_1
C                   cycles/limb     cycles/limb
C POWER3/PPC630		 -		 -
C POWER4/PPC970		 -		 -
C POWER5		 -		 -
C POWER6		 -		 -
C POWER7		 -		 -
C POWER8		 -		 -
C POWER9		 2.63		 2.63

C INPUT PARAMETERS
define(`rp', `r3')
define(`up', `r4')
define(`n',  `r5')
define(`v0', `r6')


ifdef(`OPERATION_addmul_1',`
  define(`ADDSUBC',	adde)
  define(`ADDSUB',	addc)
  define(`func',	mpn_addmul_1)
  define(`AM',		`$1')
  define(`SM',		`')
')
ifdef(`OPERATION_submul_1',`
  define(`ADDSUBC',	subfe)
  define(`ADDSUB',	subfc)
  define(`func',	mpn_submul_1)
  define(`AM',		`')
  define(`SM',		`$1')
')

MULFUNC_PROLOGUE(mpn_addmul_1 mpn_submul_1)

ASM_START()
PROLOGUE(func)
	cmpdi	cr7, n, 3
	srdi	r10, n, 2
	mtctr	r10
	rldicl.	r9, n, 0, 63
	ld	r11, 0(up)
	bne	cr0, L(bx1)

L(bx0):	rldicl. r9, n, 63, 63
AM(`	subfzeo	r12, n		')	C ov = 0, ca = 0
AM(`	li	r12, 0		')
SM(`	subfco	r12, r12, r12	')	C r12 = 0, ov = 0, ca = 1
	ld	r9, 8(up)
	mulld	r0, r11, v0
	mulhdu	r5, r11, v0
	blt	cr7, L(2)
	ld	r8, 16(up)
	bne	cr0, L(b10)

L(b00):	addi	rp, rp, -24
	b	L(lo0)
L(b10):	addi	rp, rp, -8
	addi	up, up, 16
	b	L(lo2)

L(2):	addi	rp, rp, -8
	b	L(cj2)

L(bx1):	rldicl. r9, n, 63, 63
AM(`	subfzeo	r5, n		')	C ov = 0, ca = 0
AM(`	li	r5, 0		')
SM(`	subfco	r5, r5, r5	')	C r5 = 0, ov = 0, ca = 1
	blt	cr7, L(1)
	ld	r8, 8(up)
	mulld	r7, r11, v0
	mulhdu	r12, r11, v0
	ld	r9, 16(up)
	bne	cr0, L(b11)

L(b01):	addi	rp, rp, -16
	addi	up, up, 8
	b	L(lo1)

L(1):	mulld	r7, r11, v0
	mulhdu	r12, r11, v0
	ld	r11, 0(rp)
	ADDSUB	r10, r7, r11
	std	r10, 0(rp)
AM(`	addze	r3, r12		')
SM(`	subfe	r0, r0, r0	')
SM(`	sub	r3, r12, r0	')
	blr

L(b11):	addi	up, up, 24
	ble	cr7, L(end)

	ALIGN(16)
L(top):	ld	r11, 0(rp)
	mulld	r0, r8, v0
	addex(	r7, r7, r5, 0)
	mulhdu	r5, r8, v0
	ld	r8, 0(up)
	ADDSUBC	r10, r7, r11
	std	r10, 0(rp)
L(lo2):	ld	r11, 8(rp)
	mulld	r7, r9, v0
	addex(	r0, r0, r12, 0)
	mulhdu	r12, r9, v0
	ld	r9, 8(up)
	ADDSUBC	r10, r0, r11
	std	r10, 8(rp)
L(lo1):	ld	r11, 16(rp)
	mulld	r0, r8, v0
	addex(	r7, r7, r5, 0)
	mulhdu	r5, r8, v0
	ld	r8, 16(up)
	ADDSUBC	r10, r7, r11
	std	r10, 16(rp)
L(lo0):	ld	r11, 24(rp)
	mulld	r7, r9, v0
	addex(	r0, r0, r12, 0)
	mulhdu	r12, r9, v0
	ld	r9, 24(up)
	ADDSUBC	r10, r0, r11
	std	r10, 24(rp)
	addi	up, up, 32
	addi	rp, rp, 32
	bdnz	L(top)

L(end):	ld	r11, 0(rp)
	mulld	r0, r8, v0
	addex(	r7, r7, r5, 0)
	mulhdu	r5, r8, v0
	ADDSUBC	r10, r7, r11
	std	r10, 0(rp)
L(cj2):	ld	r11, 8(rp)
	mulld	r7, r9, v0
	addex(	r0, r0, r12, 0)
	mulhdu	r12, r9, v0
	ADDSUBC	r10, r0, r11
	std	r10, 8(rp)
	ld	r11, 16(rp)
	addex(	r7, r7, r5, 0)
	ADDSUBC	r10, r7, r11
	std	r10, 16(rp)
	li	r0, 0
	addex(	r3, r12, r0, 0)
AM(`	addze	r3, r3		')
SM(`	subfe	r0, r0, r0	')
SM(`	sub	r3, r3, r0	')
	blr
EPILOGUE()
