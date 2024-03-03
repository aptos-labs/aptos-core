dnl  ARM64 mpn_add_n and mpn_sub_n

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2013, 2017 Free Software Foundation, Inc.

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
C Cortex-A53	2.75-3.25
C Cortex-A57	 1.5
C X-Gene	 2.0

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`vp', `x2')
define(`n',  `x3')

ifdef(`OPERATION_add_n', `
  define(`ADDSUBC',	adcs)
  define(`CLRCY',	`cmn	xzr, xzr')
  define(`SETCY',	`cmp	$1, #1')
  define(`RETVAL',	`cset	x0, cs')
  define(`func_n',	mpn_add_n)
  define(`func_nc',	mpn_add_nc)')
ifdef(`OPERATION_sub_n', `
  define(`ADDSUBC',	sbcs)
  define(`CLRCY',	`cmp	xzr, xzr')
  define(`SETCY',	`cmp	xzr, $1')
  define(`RETVAL',	`cset	x0, cc')
  define(`func_n',	mpn_sub_n)
  define(`func_nc',	mpn_sub_nc)')

MULFUNC_PROLOGUE(mpn_add_n mpn_add_nc mpn_sub_n mpn_sub_nc)

ASM_START()
PROLOGUE(func_nc)
	SETCY(	x4)
	b	L(ent)
EPILOGUE()
PROLOGUE(func_n)
	CLRCY
L(ent):	lsr	x18, n, #2
	tbz	n, #0, L(bx0)

L(bx1):	ldr	x7, [up]
	ldr	x11, [vp]
	ADDSUBC	x13, x7, x11
	str	x13, [rp],#8
	tbnz	n, #1, L(b11)

L(b01):	cbz	x18, L(ret)
	ldp	x4, x5, [up,#8]
	ldp	x8, x9, [vp,#8]
	sub	up, up, #8
	sub	vp, vp, #8
	b	L(mid)

L(b11):	ldp	x6, x7, [up,#8]
	ldp	x10, x11, [vp,#8]
	add	up, up, #8
	add	vp, vp, #8
	cbz	x18, L(end)
	b	L(top)

L(bx0):	tbnz	n, #1, L(b10)

L(b00):	ldp	x4, x5, [up]
	ldp	x8, x9, [vp]
	sub	up, up, #16
	sub	vp, vp, #16
	b	L(mid)

L(b10):	ldp	x6, x7, [up]
	ldp	x10, x11, [vp]
	cbz	x18, L(end)

	ALIGN(16)
L(top):	ldp	x4, x5, [up,#16]
	ldp	x8, x9, [vp,#16]
	ADDSUBC	x12, x6, x10
	ADDSUBC	x13, x7, x11
	stp	x12, x13, [rp],#16
L(mid):	ldp	x6, x7, [up,#32]!
	ldp	x10, x11, [vp,#32]!
	ADDSUBC	x12, x4, x8
	ADDSUBC	x13, x5, x9
	stp	x12, x13, [rp],#16
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	ADDSUBC	x12, x6, x10
	ADDSUBC	x13, x7, x11
	stp	x12, x13, [rp]
L(ret):	RETVAL
	ret
EPILOGUE()
