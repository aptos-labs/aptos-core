dnl  AMD64 mpn_addmul_2 optimised for AMD Bulldozer.

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
C AMD K8,K9
C AMD K10
C AMD bd1	 4.2
C AMD bd2	 4.4
C AMD bd3
C AMD bd4
C AMD zen
C AMD bt1
C AMD bt2
C Intel P4
C Intel PNR
C Intel NHM
C Intel SBR
C Intel IBR
C Intel HWL
C Intel BWL
C Intel SKL
C Intel atom
C Intel SLM
C VIA nano

C The loop of this code is the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

define(`rp',      `%rdi')   C rcx
define(`up',      `%rsi')   C rdx
define(`n_param', `%rdx')   C r8
define(`vp',      `%rcx')   C r9

define(`n',       `%rcx')
define(`v0',      `%rbx')
define(`v1',      `%rbp')
define(`X0',      `%r12')
define(`X1',      `%r13')

define(`w0',    `%r8')
define(`w1',    `%r9')
define(`w2',    `%r10')
define(`w3',    `%r11')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_addmul_2)
	FUNC_ENTRY(4)
	push	%rbx
	push	%rbp
	push	%r12
	push	%r13

	mov	(vp), v0
	mov	8(vp), v1

	mov	(up), %rax
	mov	$0, R32(w2)		C abuse w2

	lea	(up,n_param,8), up
	lea	(rp,n_param,8), rp
	sub	n_param, w2
	mul	v0

	test	$1, R8(w2)
	jnz	L(bx1)

L(bx0):	mov	%rdx, X0
	mov	%rax, X1
	test	$2, R8(w2)
	jnz	L(b10)

L(b00):	lea	(w2), n			C un = 4, 8, 12, ...
	mov	(up,w2,8), %rax
	mov	(rp,w2,8), w3
	mul	v1
	mov	%rax, w0
	mov	8(up,w2,8), %rax
	mov	%rdx, w1
	jmp	L(lo0)

L(b10):	lea	2(w2), n		C un = 2, 6, 10, ...
	mov	(up,w2,8), %rax
	mov	(rp,w2,8), w1
	mul	v1
	mov	%rdx, w3
	mov	%rax, w2
	mov	-8(up,n,8), %rax
	test	n, n
	jz	L(end)
	jmp	L(top)

L(bx1):	mov	%rax, X0
	mov	%rdx, X1
	test	$2, R8(w2)
	jz	L(b11)

L(b01):	lea	1(w2), n		C un = 1, 5, 9, ...
	mov	(up,w2,8), %rax
	mul	v1
	mov	(rp,w2,8), w2
	mov	%rdx, w0
	mov	%rax, w3
	jmp	L(lo1)

L(b11):	lea	-1(w2), n		C un = 3, 7, 11, ...
	mov	(up,w2,8), %rax
	mul	v1
	mov	(rp,w2,8), w0
	mov	%rax, w1
	mov	8(up,w2,8), %rax
	mov	%rdx, w2
	jmp	L(lo3)

	ALIGN(32)
L(top):
L(lo2):	mul	v0
	add	w1, X1
	mov	X1, -16(rp,n,8)
	mov	%rdx, X1
	adc	%rax, X0
	adc	$0, X1
	mov	-8(up,n,8), %rax
	mul	v1
	mov	-8(rp,n,8), w1
	mov	%rdx, w0
	add	w1, w2
	adc	%rax, w3
	adc	$0, w0
L(lo1):	mov	(up,n,8), %rax
	mul	v0
	add	w2, X0
	mov	X0, -8(rp,n,8)
	mov	%rdx, X0
	adc	%rax, X1
	mov	(up,n,8), %rax
	adc	$0, X0
	mov	(rp,n,8), w2
	mul	v1
	add	w2, w3
	adc	%rax, w0
	mov	8(up,n,8), %rax
	mov	%rdx, w1
	adc	$0, w1
L(lo0):	mul	v0
	add	w3, X1
	mov	X1, (rp,n,8)
	adc	%rax, X0
	mov	8(up,n,8), %rax
	mov	%rdx, X1
	adc	$0, X1
	mov	8(rp,n,8), w3
	mul	v1
	add	w3, w0
	adc	%rax, w1
	mov	16(up,n,8), %rax
	mov	%rdx, w2
	adc	$0, w2
L(lo3):	mul	v0
	add	w0, X0
	mov	X0, 8(rp,n,8)
	mov	%rdx, X0
	adc	%rax, X1
	adc	$0, X0
	mov	16(up,n,8), %rax
	mov	16(rp,n,8), w0
	mul	v1
	mov	%rdx, w3
	add	w0, w1
	adc	%rax, w2
	adc	$0, w3
	mov	24(up,n,8), %rax
	add	$4, n
	jnc	L(top)

L(end):	mul	v0
	add	w1, X1
	mov	X1, -16(rp)
	mov	%rdx, X1
	adc	%rax, X0
	adc	$0, X1
	mov	-8(up), %rax
	mul	v1
	mov	-8(rp), w1
	add	w1, w2
	adc	%rax, w3
	adc	$0, %rdx
	add	w2, X0
	adc	$0, X1
	mov	X0, -8(rp)
	add	w3, X1
	mov	X1, (rp)
	adc	$0, %rdx
	mov	%rdx, %rax

	pop	%r13
	pop	%r12
	pop	%rbp
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
