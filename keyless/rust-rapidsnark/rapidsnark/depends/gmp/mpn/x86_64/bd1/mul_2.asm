dnl  AMD64 mpn_mul_2 optimised for AMD Bulldozer.

dnl  Contributed to the GNU project by Torbjörn Granlund.

dnl  Copyright 2008, 2011-2013 Free Software Foundation, Inc.

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
C AMD K8,K9	 6.78
C AMD K10	 6.78
C AMD bd1	 8.39	 8.65
C AMD bd2	 8.47
C AMD bd3
C AMD bd4
C AMD zen
C AMD bt1	12.1
C AMD bt2	11.5
C Intel P4	24.0
C Intel PNR	 8.14
C Intel NHM	 7.78
C Intel SBR	 6.34
C Intel IBR	 6.15
C Intel HWL	 6.04
C Intel BWL	 4.33
C Intel SKL	 4.41
C Intel atom	39.5
C Intel SLM	27.8
C VIA nano

C The loop of this code is the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

define(`rp',      `%rdi')   C rcx
define(`up',      `%rsi')   C rdx
define(`n_param', `%rdx')   C r8
define(`vp',      `%rcx')   C r9

define(`v0', `%r8')
define(`v1', `%r9')
define(`w0', `%rbx')
define(`w1', `%rcx')
define(`w2', `%rbp')
define(`w3', `%r10')
define(`n',  `%r11')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_mul_2)
	FUNC_ENTRY(4)
	push	%rbx
	push	%rbp

	mov	(up), %rax

	mov	(vp), v0
	mov	8(vp), v1

	lea	(up,n_param,8), up
	lea	(rp,n_param,8), rp

	mov	n_param, n
	mul	v0
	neg	n

	test	$1, R8(n)
	jnz	L(bx1)

L(bx0):	test	$2, R8(n)
	jnz	L(b10)

L(b00):	mov	%rax, w0
	mov	%rdx, w1
	xor	R32(w2), R32(w2)
	mov	(up,n,8), %rax
	jmp	L(lo0)

L(b10):	mov	%rax, w2
	mov	%rdx, w3
	mov	(up,n,8), %rax
	xor	R32(w0), R32(w0)
	mul	v1
	add	$-2, n
	jmp	L(lo2)

L(bx1):	test	$2, R8(n)
	jz	L(b11)

L(b01):	mov	%rax, w3
	mov	%rdx, w0
	mov	(up,n,8), %rax
	mul	v1
	xor	R32(w1), R32(w1)
	inc	n
	jmp	L(lo1)

L(b11):	mov	%rax, w1
	mov	%rdx, w2
	mov	(up,n,8), %rax
	xor	R32(w3), R32(w3)
	dec	n
	jmp	L(lo3)

	ALIGN(32)
L(top):	mov	-8(up,n,8), %rax
	mul	v1
	mov	w2, -16(rp,n,8)
L(lo1):	add	%rax, w0
	mov	w3, -8(rp,n,8)
	adc	%rdx, w1
	mov	(up,n,8), %rax
	mul	v0
	mov	$0, R32(w2)
	add	%rax, w0
	adc	%rdx, w1
	adc	$0, R32(w2)
	mov	(up,n,8), %rax
L(lo0):	mul	v1
	add	%rax, w1
	adc	%rdx, w2
	mov	8(up,n,8), %rax
	mul	v0
	add	%rax, w1
	mov	w0, (rp,n,8)
	mov	$0, R32(w3)
	mov	8(up,n,8), %rax
	adc	%rdx, w2
	adc	$0, R32(w3)
L(lo3):	mul	v1
	add	%rax, w2
	mov	16(up,n,8), %rax
	adc	%rdx, w3
	mul	v0
	add	%rax, w2
	mov	16(up,n,8), %rax
	mov	$0, R32(w0)
	adc	%rdx, w3
	adc	$0, R32(w0)
	mul	v1
	mov	w1, 8(rp,n,8)
L(lo2):	add	%rax, w3
	adc	%rdx, w0
	mov	24(up,n,8), %rax
	mul	v0
	add	%rax, w3
	adc	%rdx, w0
	mov	$0, R32(w1)
	adc	$0, R32(w1)
	add	$4, n
	jnc	L(top)

L(end):	mov	-8(up), %rax
	mul	v1
	mov	w2, -16(rp)
	add	%rax, w0
	mov	w3, -8(rp)
	adc	%rdx, w1
	mov	w0, (rp)
	mov	w1, %rax

	pop	%rbp
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
