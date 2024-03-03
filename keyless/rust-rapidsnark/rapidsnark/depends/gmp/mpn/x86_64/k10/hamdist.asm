dnl  AMD64 mpn_hamdist -- hamming distance.

dnl  Copyright 2008, 2010-2012, 2017 Free Software Foundation, Inc.

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
C AMD K8,K9	 -
C AMD K10	 2.0		=
C AMD bd1	~4.4		=
C AMD bd2	~4.4		=
C AMD bd3
C AMD bd4
C AMD bobcat	 7.55		=
C AMD jaguar	 2.52		-
C Intel P4	 -
C Intel core2	 -
C Intel NHM	 2.03		+
C Intel SBR	 2.01		+
C Intel IBR	 1.96		+
C Intel HWL	 1.64		=
C Intel BWL	 1.56		-
C Intel SKL	 1.52		=
C Intel atom
C Intel SLM	 3.0		-
C VIA nano

define(`ap',		`%rdi')
define(`bp',		`%rsi')
define(`n',		`%rdx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_hamdist)
	FUNC_ENTRY(3)
	mov	(ap), %r8
	xor	(bp), %r8

	lea	(ap,n,8), ap			C point at A operand end
	lea	(bp,n,8), bp			C point at B operand end
	neg	n

	test	$1, R8(n)
	jz	L(2)

L(1):	.byte	0xf3,0x49,0x0f,0xb8,0xc0	C popcnt %r8, %rax
	xor	R32(%r10), R32(%r10)
	inc	n
	js	L(top)
	FUNC_EXIT()
	ret

	ALIGN(16)
L(2):	mov	8(ap,n,8), %r9
	.byte	0xf3,0x49,0x0f,0xb8,0xc0	C popcnt %r8, %rax
	xor	8(bp,n,8), %r9
	.byte	0xf3,0x4d,0x0f,0xb8,0xd1	C popcnt %r9, %r10
	add	$2, n
	js	L(top)
	lea	(%r10, %rax), %rax
	FUNC_EXIT()
	ret

	ALIGN(16)
L(top):	mov	(ap,n,8), %r8
	lea	(%r10, %rax), %rax
	mov	8(ap,n,8), %r9
	xor	(bp,n,8), %r8
	xor	8(bp,n,8), %r9
	.byte	0xf3,0x49,0x0f,0xb8,0xc8	C popcnt %r8, %rcx
	lea	(%rcx, %rax), %rax
	.byte	0xf3,0x4d,0x0f,0xb8,0xd1	C popcnt %r9, %r10
	add	$2, n
	js	L(top)

	lea	(%r10, %rax), %rax
	FUNC_EXIT()
	ret
EPILOGUE()
