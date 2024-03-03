dnl  ARM64 mpn_addlshC_n, mpn_sublshC_n, mpn_rsblshC_n.

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

C	     cycles/limb
C Cortex-A53	3.25-3.75
C Cortex-A57	 2.18
C X-Gene	 2.5

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`vp', `x2')
define(`n',  `x3')

ifdef(`DO_add', `
  define(`ADDSUB',	`adds	$1, $2, $3')
  define(`ADDSUBC',	`adcs	$1, $2, $3')
  define(`CLRRCY',	`adds	$1, xzr, xzr')
  define(`RETVAL',	`adc	x0, $1, xzr')
  define(`func_n',	mpn_addlsh`'LSH`'_n)')
ifdef(`DO_sub', `
  define(`ADDSUB',	`subs	$1, $3, $2')
  define(`ADDSUBC',	`sbcs	$1, $3, $2')
  define(`CLRRCY',	`subs	$1, xzr, xzr')
  define(`RETVAL',	`cinc	x0, $1, cc')
  define(`func_n',	mpn_sublsh`'LSH`'_n)')
ifdef(`DO_rsb', `
  define(`ADDSUB',	`subs	$1, $2, $3')
  define(`ADDSUBC',	`sbcs	$1, $2, $3')
  define(`CLRRCY',	`subs	$1, xzr, xzr')
  define(`RETVAL',	`sbc	x0, $1, xzr')
  define(`func_n',	mpn_rsblsh`'LSH`'_n)')

ASM_START()
PROLOGUE(func_n)
	lsr	x18, n, #2
	tbz	n, #0, L(bx0)

L(bx1):	ldr	x5, [up]
	tbnz	n, #1, L(b11)

L(b01):	ldr	x11, [vp]
	cbz	x18, L(1)
	ldp	x8, x9, [vp,#8]
	lsl	x13, x11, #LSH
	ADDSUB(	x15, x13, x5)
	str	x15, [rp],#8
	sub	up, up, #24
	sub	vp, vp, #8
	b	L(mid)

L(1):	lsl	x13, x11, #LSH
	ADDSUB(	x15, x13, x5)
	str	x15, [rp]
	lsr	x0, x11, RSH
	RETVAL(	 x0, x1)
	ret

L(b11):	ldr	x9, [vp]
	ldp	x10, x11, [vp,#8]!
	lsl	x13, x9, #LSH
	ADDSUB(	x17, x13, x5)
	str	x17, [rp],#8
	sub	up, up, #8
	cbz	x18, L(end)
	b	L(top)

L(bx0):	tbnz	n, #1, L(b10)

L(b00):	CLRRCY(	x11)
	ldp	x8, x9, [vp],#-16
	sub	up, up, #32
	b	L(mid)

L(b10):	CLRRCY(	x9)
	ldp	x10, x11, [vp]
	sub	up, up, #16
	cbz	x18, L(end)

	ALIGN(16)
L(top):	ldp	x4, x5, [up,#16]
	extr	x12, x10, x9, #RSH
	ldp	x8, x9, [vp,#16]
	extr	x13, x11, x10, #RSH
	ADDSUBC(x14, x12, x4)
	ADDSUBC(x15, x13, x5)
	stp	x14, x15, [rp],#16
L(mid):	ldp	x4, x5, [up,#32]!
	extr	x12, x8, x11, #RSH
	ldp	x10, x11, [vp,#32]!
	extr	x13, x9, x8, #RSH
	ADDSUBC(x16, x12, x4)
	ADDSUBC(x17, x13, x5)
	stp	x16, x17, [rp],#16
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	ldp	x4, x5, [up,#16]
	extr	x12, x10, x9, #RSH
	extr	x13, x11, x10, #RSH
	ADDSUBC(x14, x12, x4)
	ADDSUBC(x15, x13, x5)
	stp	x14, x15, [rp]
	lsr	x0, x11, RSH
	RETVAL(	 x0, x1)
	ret
EPILOGUE()
