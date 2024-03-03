dnl  ARM64 mpn_rsh1add_n and mpn_rsh1sub_n.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2017 Free Software Foundation, Inc.

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
C Cortex-A53	3.25-3.75	 3.0 steady
C Cortex-A57	 2.15		 1.75
C X-Gene	 2.75		 2.5

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`vp', `x2')
define(`n',  `x3')

ifdef(`OPERATION_rsh1add_n', `
  define(`ADDSUB',	adds)
  define(`ADDSUBC',	adcs)
  define(`COND',	`cs')
  define(`func_n',	mpn_rsh1add_n)')
ifdef(`OPERATION_rsh1sub_n', `
  define(`ADDSUB',	subs)
  define(`ADDSUBC',	sbcs)
  define(`COND',	`cc')
  define(`func_n',	mpn_rsh1sub_n)')

MULFUNC_PROLOGUE(mpn_rsh1add_n mpn_rsh1sub_n)

ASM_START()
PROLOGUE(func_n)
	lsr	x18, n, #2

	tbz	n, #0, L(bx0)

L(bx1):	ldr	x5, [up],#8
	ldr	x9, [vp],#8
	tbnz	n, #1, L(b11)

L(b01):	ADDSUB	x13, x5, x9
	and	x10, x13, #1
	cbz	x18, L(1)
	ldp	x4, x5, [up],#48
	ldp	x8, x9, [vp],#48
	ADDSUBC	x14, x4, x8
	ADDSUBC	x15, x5, x9
	ldp	x4, x5, [up,#-32]
	ldp	x8, x9, [vp,#-32]
	extr	x17, x14, x13, #1
	ADDSUBC	x12, x4, x8
	ADDSUBC	x13, x5, x9
	str	x17, [rp], #24
	sub	x18, x18, #1
	cbz	x18, L(end)
	b	L(top)

L(1):	cset	x14, COND
	extr	x17, x14, x13, #1
	str	x17, [rp]
	mov	x0, x10
	ret

L(b11):	ADDSUB	x15, x5, x9
	and	x10, x15, #1

	ldp	x4, x5, [up],#32
	ldp	x8, x9, [vp],#32
	ADDSUBC	x12, x4, x8
	ADDSUBC	x13, x5, x9
	cbz	x18, L(3)
	ldp	x4, x5, [up,#-16]
	ldp	x8, x9, [vp,#-16]
	extr	x17, x12, x15, #1
	ADDSUBC	x14, x4, x8
	ADDSUBC	x15, x5, x9
	str	x17, [rp], #8
	b	L(mid)

L(3):	extr	x17, x12, x15, #1
	str	x17, [rp], #8
	b	L(2)

L(bx0):	tbz	n, #1, L(b00)

L(b10):	ldp	x4, x5, [up],#32
	ldp	x8, x9, [vp],#32
	ADDSUB	x12, x4, x8
	ADDSUBC	x13, x5, x9
	and	x10, x12, #1
	cbz	x18, L(2)
	ldp	x4, x5, [up,#-16]
	ldp	x8, x9, [vp,#-16]
	ADDSUBC	x14, x4, x8
	ADDSUBC	x15, x5, x9
	b	L(mid)

L(b00):	ldp	x4, x5, [up],#48
	ldp	x8, x9, [vp],#48
	ADDSUB	x14, x4, x8
	ADDSUBC	x15, x5, x9
	and	x10, x14, #1
	ldp	x4, x5, [up,#-32]
	ldp	x8, x9, [vp,#-32]
	ADDSUBC	x12, x4, x8
	ADDSUBC	x13, x5, x9
	add	rp, rp, #16
	sub	x18, x18, #1
	cbz	x18, L(end)

	ALIGN(16)
L(top):	ldp	x4, x5, [up,#-16]
	ldp	x8, x9, [vp,#-16]
	extr	x16, x15, x14, #1
	extr	x17, x12, x15, #1
	ADDSUBC	x14, x4, x8
	ADDSUBC	x15, x5, x9
	stp	x16, x17, [rp,#-16]
L(mid):	ldp	x4, x5, [up],#32
	ldp	x8, x9, [vp],#32
	extr	x16, x13, x12, #1
	extr	x17, x14, x13, #1
	ADDSUBC	x12, x4, x8
	ADDSUBC	x13, x5, x9
	stp	x16, x17, [rp],#32
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	extr	x16, x15, x14, #1
	extr	x17, x12, x15, #1
	stp	x16, x17, [rp,#-16]
L(2):	cset	x14, COND
	extr	x16, x13, x12, #1
	extr	x17, x14, x13, #1
	stp	x16, x17, [rp]

L(ret):	mov	x0, x10
	ret
EPILOGUE()
