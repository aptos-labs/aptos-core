dnl  ARM64 mpn_sqr_diag_addlsh1.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2016, 2017 Free Software Foundation, Inc.

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

C	     cycles/limb
C Cortex-A53	 5.65
C Cortex-A57	 3.5
C X-Gene	 3.38

changecom(blah)

define(`rp', `x0')
define(`tp', `x1')
define(`up', `x2')
define(`n',  `x3')

ASM_START()
PROLOGUE(mpn_sqr_diag_addlsh1)
	ldr	x15, [up],#8
	lsr	x18, n, #1
	tbz	n, #0, L(bx0)

L(bx1):	adds	x7, xzr, xzr
	mul	x12, x15, x15
	ldr	x16, [up],#8
	ldp	x4, x5, [tp],#16
	umulh	x11, x15, x15
	b	L(mid)

L(bx0):	adds	x5, xzr, xzr
	mul	x12, x15, x15
	ldr	x17, [up],#16
	ldp	x6, x7, [tp],#32
	umulh	x11, x15, x15
	sub	x18, x18, #1
	cbz	x18, L(end)

	ALIGN(16)
L(top):	extr	x9, x6, x5, #63
	mul	x10, x17, x17
	ldr	x16, [up,#-8]
	adcs	x13, x9, x11
	ldp	x4, x5, [tp,#-16]
	umulh	x11, x17, x17
	extr	x8, x7, x6, #63
	stp	x12, x13, [rp],#16
	adcs	x12, x8, x10
L(mid):	extr	x9, x4, x7, #63
	mul	x10, x16, x16
	ldr	x17, [up],#16
	adcs	x13, x9, x11
	ldp	x6, x7, [tp],#32
	umulh	x11, x16, x16
	extr	x8, x5, x4, #63
	stp	x12, x13, [rp],#16
	adcs	x12, x8, x10
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	extr	x9, x6, x5, #63
	mul	x10, x17, x17
	adcs	x13, x9, x11
	umulh	x11, x17, x17
	extr	x8, x7, x6, #63
	stp	x12, x13, [rp]
	adcs	x12, x8, x10
	extr	x9, xzr, x7, #63
	adcs	x13, x9, x11
	stp	x12, x13, [rp,#16]

	ret
EPILOGUE()
