dnl  x86-64 mpn_lshift optimised for Conroe/Penryn and Nehalem.

dnl  Copyright 2007, 2009, 2011, 2012, 2017 Free Software Foundation, Inc.

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
C AMD K8,K9
C AMD K10
C AMD bd1
C AMD bd2
C AMD bd3
C AMD bd4
C AMD zen
C AMD bobcat
C AMD jaguar
C Intel P4
C Intel core2	 1.32
C Intel NHM	 1.30	(drops to 2.5 for n > 256)
C Intel SBR
C Intel IBR
C Intel HWL
C Intel BWL
C Intel SKL
C Intel atom
C Intel SLM
C VIA nano

C INPUT PARAMETERS
define(`rp',	`%rdi')
define(`up',	`%rsi')
define(`n',	`%rdx')
define(`cnt',	`%rcx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_lshift)
	FUNC_ENTRY(4)

	xor	R32(%rax), R32(%rax)

	test	$1, R8(n)
	jnz	L(bx1)
L(bx0):	test	$2, R8(n)
	jnz	L(b10)

L(b00):	lea	-8(up,n,8), up
	lea	16(rp,n,8), rp
	mov	(up), %r10
	mov	-8(up), %r11
	shld	R8(cnt), %r10, %rax
	mov	-16(up), %r8
	shr	$2, n
	jmp	L(00)

L(bx1):	test	$2, R8(n)
	jnz	L(b11)

L(b01):	lea	-16(up,n,8), up
	lea	8(rp,n,8), rp
	mov	8(up), %r9
	shld	R8(cnt), %r9, %rax
	shr	$2, n
	jz	L(1)
	mov	(up), %r10
	mov	-8(up), %r11
	jmp	L(01)

L(b10):	lea	-24(up,n,8), up
	lea	(rp,n,8), rp
	mov	16(up), %r8
	mov	8(up), %r9
	shld	R8(cnt), %r8, %rax
	shr	$2, n
	jz	L(2)
	mov	(up), %r10
	jmp	L(10)

	ALIGN(16)
L(b11):	lea	-32(up,n,8), up
	lea	-8(rp,n,8), rp
	mov	24(up), %r11
	mov	16(up), %r8
	mov	8(up), %r9
	shld	R8(cnt), %r11, %rax
	shr	$2, n
	jz	L(end)

	ALIGN(16)
L(top):	shld	R8(cnt), %r8, %r11
	mov	(up), %r10
	mov	%r11, (rp)
L(10):	shld	R8(cnt), %r9, %r8
	mov	-8(up), %r11
	mov	%r8, -8(rp)
L(01):	shld	R8(cnt), %r10, %r9
	mov	-16(up), %r8
	mov	%r9, -16(rp)
L(00):	shld	R8(cnt), %r11, %r10
	mov	-24(up), %r9
	add	$-32, up
	mov	%r10, -24(rp)
	add	$-32, rp
	dec	n
	jnz	L(top)

L(end):	shld	R8(cnt), %r8, %r11
	mov	%r11, (rp)
L(2):	shld	R8(cnt), %r9, %r8
	mov	%r8, -8(rp)
L(1):	shl	R8(cnt), %r9
	mov	%r9, -16(rp)
	FUNC_EXIT()
	ret
EPILOGUE()
