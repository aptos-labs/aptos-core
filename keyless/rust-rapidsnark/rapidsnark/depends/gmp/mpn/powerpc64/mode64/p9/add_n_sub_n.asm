dnl  PowerPC-64 mpn_add_n_sub_n optimised for POWER9.

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

C		   cycles/limb
C POWER3/PPC630		 -
C POWER4/PPC970		 -
C POWER5		 -
C POWER6		 -
C POWER7		 -
C POWER8		 -
C POWER9		 2.25


C INPUT PARAMETERS
define(`arp',	`r3')
define(`srp',	`r4')
define(`up',	`r5')
define(`vp',	`r6')
define(`n',	`r7')

ASM_START()
PROLOGUE(mpn_add_n_sub_n)
	cmpdi	cr7, n, 2
	subfo	r0, r0, r0		C clear OV
	rldicl.	r9, n, 0, 63		C n & 1
	beq	cr0, L(bx0)

L(bx1):	ld	r10, 0(up)
	ld	r11, 0(vp)
	ble	cr7, L(1)
	srdi	r7, r7, 1
	mtctr	r7
	ld	r8, 8(up)
	ld	r9, 8(vp)
	addex(	r0, r10, r11, 0)
	subfc	r12, r11, r10
	addi	up, up, -8
	addi	vp, vp, -8
	b	L(lo1)

L(bx0):	ld	r8, 0(up)
	ld	r9, 0(vp)
	ld	r10, 8(up)
	ld	r11, 8(vp)
	addex(	r0, r8, r9, 0)
	subfc	r12, r9, r8
	addi	arp, arp, 8
	addi	srp, srp, 8
	ble	cr7, L(end)
	addi	r7, r7, -1
	srdi	r7, r7, 1
	mtctr	r7

L(top):	ld	r8, 16(up)
	ld	r9, 16(vp)
	std	r0, -8(arp)
	std	r12, -8(srp)
	addex(	r0, r10, r11, 0)
	subfe	r12, r11, r10
L(lo1):	ld	r10, 24(up)
	ld	r11, 24(vp)
	std	r0, 0(arp)
	std	r12, 0(srp)
	addex(	r0, r8, r9, 0)
	subfe	r12, r9, r8
	addi	up, up, 16
	addi	vp, vp, 16
	addi	arp, arp, 16
	addi	srp, srp, 16
	bdnz	L(top)

L(end):	std	r0, -8(arp)
	std	r12, -8(srp)
L(1):	addex(	r0, r10, r11, 0)
	subfe	r12, r11, r10
	std	r0, 0(arp)
	std	r12, 0(srp)
	subfe	r3, r3, r3
	addex(	r3, r3, r3, 0)
	rldicl	r3, r3, 1, 62
	blr
EPILOGUE()
ASM_END()
