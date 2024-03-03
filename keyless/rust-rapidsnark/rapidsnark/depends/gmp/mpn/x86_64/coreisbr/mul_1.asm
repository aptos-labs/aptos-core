dnl  X86-64 mpn_mul_1 optimised for Intel Sandy Bridge.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2003-2005, 2007, 2008, 2011-2013, 2017 Free Software Foundation,
dnl  Inc.

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
C AMD bull
C AMD pile
C AMD steam
C AMD excavator
C AMD bobcat
C AMD jaguar
C Intel P4
C Intel core2
C Intel NHM
C Intel SBR      2.49
C Intel IBR      2.32
C Intel HWL      2.44
C Intel BWL      2.43
C Intel SKL      2.47
C Intel atom
C Intel SLM
C VIA nano

C The loop of this code is the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

define(`rp',      `%rdi')   C rcx
define(`up_param',`%rsi')   C rdx
define(`n_param', `%rdx')   C r8
define(`v0',      `%rcx')   C r9
define(`cin',     `%r8')    C stack

define(`up',      `%rsi')   C same as rp_param
define(`n',	  `%r9')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

IFDOS(`	define(`rp',      `%rcx')')
IFDOS(`	define(`up_param',`%rdx')')
IFDOS(`	define(`n_param', `%r8')')
IFDOS(`	define(`v0',      `%r9')')
IFDOS(`	define(`cin',     `48(%rsp)')')

IFDOS(`	define(`up',      `%rsi')')
IFDOS(`	define(`n',       `%r8')')

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_mul_1)
IFDOS(`	push	%rsi		')
	mov	(up_param), %rax
IFSTD(`	mov	n_param, n	')
	lea	(up_param,n_param,8), up
	lea	-8(rp,n_param,8), rp
	neg	n
	mul	v0

	test	$1, R8(n)
	jz	L(x0)
L(x1):	mov	%rax, %r11
	mov	%rdx, %r10
	test	$2, R8(n)
	jnz	L(01)

L(11):	mov	8(up,n,8), %rax
	dec	n
	jmp	L(L3)

L(01):	inc	n
	jnz	L(L1)
	mov	%rax, (rp)
	mov	%rdx, %rax
IFDOS(`	pop	%rsi		')
	ret

L(x0):	mov	%rax, %r10
	mov	%rdx, %r11
	mov	8(up,n,8), %rax
	test	$2, R8(n)
	jz	L(L0)

L(10):	add	$-2, n
	jmp	L(L2)

	ALIGN(8)
L(top):	mov	%rdx, %r10
	add	%rax, %r11
L(L1):	mov	0(up,n,8), %rax
	adc	$0, %r10
	mul	v0
	add	%rax, %r10
	mov	%r11, 0(rp,n,8)
	mov	8(up,n,8), %rax
	mov	%rdx, %r11
L(L0c):	adc	$0, %r11
L(L0):	mul	v0
	mov	%r10, 8(rp,n,8)
	add	%rax, %r11
	mov	%rdx, %r10
L(L3c):	mov	16(up,n,8), %rax
	adc	$0, %r10
L(L3):	mul	v0
	mov	%r11, 16(rp,n,8)
	mov	%rdx, %r11
	add	%rax, %r10
L(L2c):	mov	24(up,n,8), %rax
	adc	$0, %r11
L(L2):	mul	v0
	mov	%r10, 24(rp,n,8)
	add	$4, n
	jnc	L(top)

L(end):	add	%rax, %r11
	mov	%rdx, %rax
	adc	$0, %rax
	mov	%r11, (rp)

IFDOS(`	pop	%rsi		')
	ret
EPILOGUE()

	ALIGN(16)
PROLOGUE(mpn_mul_1c)
IFDOS(`	push	%rsi		')
	mov	(up_param), %rax
IFSTD(`	mov	n_param, n	')
	lea	(up_param,n_param,8), up
	lea	-8(rp,n_param,8), rp
	neg	n
	mul	v0

	test	$1, R8(n)
	jz	L(x0c)
L(x1c):	mov	%rax, %r11
	mov	%rdx, %r10
	test	$2, R8(n)
	jnz	L(01c)

L(11c):	add	cin, %r11
	dec	n
	jmp	L(L3c)

L(01c):	add	cin, %r11
	inc	n
	jnz	L(L1)
	mov	%r11, (rp)
	mov	%rdx, %rax
	adc	$0, %rax
IFDOS(`	pop	%rsi		')
	ret

L(x0c):	mov	%rax, %r10
	mov	%rdx, %r11
	test	$2, R8(n)
	jz	L(00c)

L(10c):	add	$-2, n
	add	cin, %r10
	jmp	L(L2c)

L(00c):	add	cin, %r10
	mov	8(up,n,8), %rax
	jmp	L(L0c)
EPILOGUE()
