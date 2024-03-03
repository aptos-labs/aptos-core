dnl  ARM64 mpn_and_n, mpn_andn_n. mpn_nand_n, etc.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2013 Free Software Foundation, Inc.

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

C	     cycles/limb     cycles/limb
C	      nand,nior	      all other
C Cortex-A53	3.25-3.5	2.75-3
C Cortex-A57	 2.0		 1.5
C X-Gene	 2.14		 2.0

changecom(blah)

define(`rp', `x0')
define(`up', `x1')
define(`vp', `x2')
define(`n',  `x3')

define(`POSTOP', `dnl')

ifdef(`OPERATION_and_n',`
  define(`func',    `mpn_and_n')
  define(`LOGOP',   `and	$1, $2, $3')')
ifdef(`OPERATION_andn_n',`
  define(`func',    `mpn_andn_n')
  define(`LOGOP',   `bic	$1, $2, $3')')
ifdef(`OPERATION_nand_n',`
  define(`func',    `mpn_nand_n')
  define(`POSTOP',  `mvn	$1, $1')
  define(`LOGOP',   `and	$1, $2, $3')')
ifdef(`OPERATION_ior_n',`
  define(`func',    `mpn_ior_n')
  define(`LOGOP',   `orr	$1, $2, $3')')
ifdef(`OPERATION_iorn_n',`
  define(`func',    `mpn_iorn_n')
  define(`LOGOP',   `orn	$1, $2, $3')')
ifdef(`OPERATION_nior_n',`
  define(`func',    `mpn_nior_n')
  define(`POSTOP',  `mvn	$1, $1')
  define(`LOGOP',   `orr	$1, $2, $3')')
ifdef(`OPERATION_xor_n',`
  define(`func',    `mpn_xor_n')
  define(`LOGOP',   `eor	$1, $2, $3')')
ifdef(`OPERATION_xnor_n',`
  define(`func',    `mpn_xnor_n')
  define(`LOGOP',   `eon	$1, $2, $3')')

MULFUNC_PROLOGUE(mpn_and_n mpn_andn_n mpn_nand_n mpn_ior_n mpn_iorn_n mpn_nior_n mpn_xor_n mpn_xnor_n)

ASM_START()
PROLOGUE(func)
	lsr	x18, n, #2
	tbz	n, #0, L(bx0)

L(bx1):	ldr	x7, [up]
	ldr	x11, [vp]
	LOGOP(	x15, x7, x11)
	POSTOP(	x15)
	str	x15, [rp],#8
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

L(b00):	ldp	x4, x5, [up],#-16
	ldp	x8, x9, [vp],#-16
	b	L(mid)

L(b10):	ldp	x6, x7, [up]
	ldp	x10, x11, [vp]
	cbz	x18, L(end)

	ALIGN(16)
L(top):	ldp	x4, x5, [up,#16]
	ldp	x8, x9, [vp,#16]
	LOGOP(	x12, x6, x10)
	LOGOP(	x13, x7, x11)
	POSTOP(	x12)
	POSTOP(	x13)
	stp	x12, x13, [rp],#16
L(mid):	ldp	x6, x7, [up,#32]!
	ldp	x10, x11, [vp,#32]!
	LOGOP(	x12, x4, x8)
	LOGOP(	x13, x5, x9)
	POSTOP(	x12)
	POSTOP(	x13)
	stp	x12, x13, [rp],#16
	sub	x18, x18, #1
	cbnz	x18, L(top)

L(end):	LOGOP(	x12, x6, x10)
	LOGOP(	x13, x7, x11)
	POSTOP(	x12)
	POSTOP(	x13)
	stp	x12, x13, [rp]
L(ret):	ret
EPILOGUE()
