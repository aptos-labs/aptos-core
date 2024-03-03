dnl  ARM64 mpn_mul_1

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2013, 2015, 2017 Free Software Foundation, Inc.

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
C Cortex-A53	7.5-8
C Cortex-A57	 7
C Cortex-A72
C X-Gene	 4

C TODO
C  * Start first multiply earlier.

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`n',  `x2')
define(`v0', `x3')


PROLOGUE(mpn_mul_1c)
	adds	xzr, xzr, xzr		C clear cy flag
	b	L(com)
EPILOGUE()

PROLOGUE(mpn_mul_1)
	adds	x4, xzr, xzr		C clear register and cy flag
L(com):	lsr	x18, n, #2
	tbnz	n, #0, L(bx1)

L(bx0):	mov	x11, x4
	tbz	n, #1, L(b00)

L(b10):	ldp	x4, x5, [up]
	mul	x8, x4, v0
	umulh	x10, x4, v0
	cbz	x18, L(2)
	ldp	x6, x7, [up,#16]!
	mul	x9, x5, v0
	b	L(mid)-8

L(2):	mul	x9, x5, v0
	b	L(2e)

L(bx1):	ldr	x7, [up],#8
	mul	x9, x7, v0
	umulh	x11, x7, v0
	adds	x9, x9, x4
	str	x9, [rp],#8
	tbnz	n, #1, L(b10)

L(b01):	cbz	x18, L(1)

L(b00):	ldp	x6, x7, [up]
	mul	x8, x6, v0
	umulh	x10, x6, v0
	ldp	x4, x5, [up,#16]
	mul	x9, x7, v0
	adcs	x12, x8, x11
	umulh	x11, x7, v0
	add	rp, rp, #16
	sub	x18, x18, #1
	cbz	x18, L(end)

	ALIGN(16)
L(top):	mul	x8, x4, v0
	ldp	x6, x7, [up,#32]!
	adcs	x13, x9, x10
	umulh	x10, x4, v0
	mul	x9, x5, v0
	stp	x12, x13, [rp,#-16]
	adcs	x12, x8, x11
	umulh	x11, x5, v0
L(mid):	mul	x8, x6, v0
	ldp	x4, x5, [up,#16]
	adcs	x13, x9, x10
	umulh	x10, x6, v0
	mul	x9, x7, v0
	stp	x12, x13, [rp],#32
	adcs	x12, x8, x11
	umulh	x11, x7, v0
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	mul	x8, x4, v0
	adcs	x13, x9, x10
	umulh	x10, x4, v0
	mul	x9, x5, v0
	stp	x12, x13, [rp,#-16]
L(2e):	adcs	x12, x8, x11
	umulh	x11, x5, v0
	adcs	x13, x9, x10
	stp	x12, x13, [rp]
L(1):	adc	x0, x11, xzr
	ret
EPILOGUE()
