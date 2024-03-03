dnl  ARM64 mpn_rshift.

dnl  Copyright 2013, 2014, 2017 Free Software Foundation, Inc.

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

C	     cycles/limb   assumed optimal c/l
C Cortex-A53	3.5-4.0		 3.25
C Cortex-A57	 2.0		 2.0
C X-Gene	 2.67		 2.5

C TODO
C  * The feed-in code used 1 ldr for odd sized and 2 ldr for even sizes.  These
C    numbers should be 1 and 0, respectively.  The str in wind-down should also
C    go.
C  * Using extr and with 63 separate loops we might reach 1.25 c/l on A57.
C  * A53's speed depends on alignment, but not as simply as for lshift/lshiftc.

changecom(blah)

define(`rp_arg', `x0')
define(`up',     `x1')
define(`n',      `x2')
define(`cnt',    `x3')

define(`rp',     `x16')

define(`tnc',`x8')

define(`PSHIFT', lsr)
define(`NSHIFT', lsl)

ASM_START()
PROLOGUE(mpn_rshift)
	mov	rp, rp_arg
	sub	tnc, xzr, cnt
	lsr	x18, n, #2
	tbz	n, #0, L(bx0)

L(bx1):	ldr	x5, [up]
	tbnz	n, #1, L(b11)

L(b01):	NSHIFT	x0, x5, tnc
	PSHIFT	x2, x5, cnt
	cbnz	x18, L(gt1)
	str	x2, [rp]
	ret
L(gt1):	ldp	x4, x5, [up,#8]
	sub	up, up, #8
	sub	rp, rp, #32
	b	L(lo2)

L(b11):	NSHIFT	x0, x5, tnc
	PSHIFT	x2, x5, cnt
	ldp	x6, x7, [up,#8]!
	sub	rp, rp, #16
	b	L(lo3)

L(bx0):	ldp	x4, x5, [up]
	tbz	n, #1, L(b00)

L(b10):	NSHIFT	x0, x4, tnc
	PSHIFT	x13, x4, cnt
	NSHIFT	x10, x5, tnc
	PSHIFT	x2, x5, cnt
	cbnz	x18, L(gt2)
	orr	x10, x10, x13
	stp	x10, x2, [rp]
	ret
L(gt2):	ldp	x4, x5, [up,#16]
	orr	x10, x10, x13
	str	x10, [rp],#-24
	b	L(lo2)

L(b00):	NSHIFT	x0, x4, tnc
	PSHIFT	x13, x4, cnt
	NSHIFT	x10, x5, tnc
	PSHIFT	x2, x5, cnt
	ldp	x6, x7, [up,#16]!
	orr	x10, x10, x13
	str	x10, [rp],#-8
	b	L(lo0)

	ALIGN(16)
L(top):	ldp	x4, x5, [up,#16]
	orr	x10, x10, x13
	orr	x11, x12, x2
	stp	x11, x10, [rp,#16]
	PSHIFT	x2, x7, cnt
L(lo2):	NSHIFT	x10, x5, tnc
	NSHIFT	x12, x4, tnc
	PSHIFT	x13, x4, cnt
	ldp	x6, x7, [up,#32]!
	orr	x10, x10, x13
	orr	x11, x12, x2
	stp	x11, x10, [rp,#32]!
	PSHIFT	x2, x5, cnt
L(lo0):	sub	x18, x18, #1
L(lo3):	NSHIFT	x10, x7, tnc
	NSHIFT	x12, x6, tnc
	PSHIFT	x13, x6, cnt
	cbnz	x18, L(top)

L(end):	orr	x10, x10, x13
	orr	x11, x12, x2
	PSHIFT	x2, x7, cnt
	stp	x11, x10, [rp,#16]
	str	x2, [rp,#32]
	ret
EPILOGUE()
