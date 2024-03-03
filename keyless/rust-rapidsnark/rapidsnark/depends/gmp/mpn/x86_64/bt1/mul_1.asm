dnl  AMD64 mpn_mul_1 optimised for AMD bt1/bt2.

dnl  Copyright 2003-2005, 2007, 2008, 2011, 2012, 2019 Free Software
dnl  Foundation, Inc.

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
C AMD K8,K9	 4.53		old measurement
C AMD K10	 4.53		old measurement
C AMD bd1	 4.56		old measurement
C AMD bd2	 4.47		old measurement
C AMD bd3	 ?
C AMD bd4	 ?
C AMD zen	 ?
C AMD bt1	 5.12
C AMD bt2	 5.17
C Intel P4	12.6		old measurement
C Intel PNR	 4.53		old measurement
C Intel NHM	 4.36		old measurement
C Intel SBR	 3.0		old measurement
C Intel IBR	 2.55		old measurement
C Intel HWL	 2.28		old measurement
C Intel BWL	 2.36		old measurement
C Intel SKL	 2.39		old measurement
C Intel atom	21.0		old measurement
C Intel SLM	 9		old measurement
C Intel GLM	 ?
C VIA nano	 ?

C The loop of this code is the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

C Standard parameters
define(`rp',              `%rdi')
define(`up',              `%rsi')
define(`n_param',         `%rdx')
define(`v0',              `%rcx')
define(`cy',              `%r8')
C Standard allocations
define(`n',               `%rbx')
define(`w0',              `%r8')
define(`w1',              `%r9')
define(`w2',              `%r10')
define(`w3',              `%r11')

C DOS64 parameters
IFDOS(` define(`rp',      `%rcx')    ') dnl
IFDOS(` define(`up',      `%rsi')    ') dnl
IFDOS(` define(`n_param', `%r8')     ') dnl
IFDOS(` define(`v0',      `%r9')     ') dnl
IFDOS(` define(`cy',      `56(%rsp)')') dnl
C DOS64 allocations
IFDOS(` define(`n',       `%rbx')    ') dnl
IFDOS(` define(`w0',      `%r8')     ') dnl
IFDOS(` define(`w1',      `%rdi')    ') dnl
IFDOS(` define(`w2',      `%r10')    ') dnl
IFDOS(` define(`w3',      `%r11')    ') dnl

	ALIGN(64)
PROLOGUE(mpn_mul_1)
IFDOS(`	push	%rsi		')
IFDOS(`	push	%rdi		')
IFDOS(`	mov	%rdx, %rsi	')

	push	%rbx
	mov	(up), %rax

	lea	(rp,n_param,8), rp
	lea	(up,n_param,8), up
	mov	n_param, n

	test	$1, R8(n_param)
	jne	L(bx1)

L(bx0):	mul	v0
	neg	n
	mov	%rax, w0
	mov	%rdx, w1
	test	$2, R8(n)
	jne	L(L2)

L(b00):	add	$2, n
	jmp	L(L0)

	ALIGN(16)
L(b11):	mov	%rax, w2
	mov	%rdx, w3
	neg	n
	inc	n
	jmp	L(L3)

	ALIGN(16)
L(bx1):	mul	v0
	test	$2, R8(n)
	jne	L(b11)

L(b01):	sub	$3, n
	jc	L(n1)
	mov	%rax, w2
	mov	%rdx, w3
	neg	n

	ALIGN(16)
L(top):	mov	-16(up,n,8), %rax
	mul	v0
	mov	%rax, w0
	mov	%rdx, w1
	mov	w2, -24(rp,n,8)
	add	w3, w0
	adc	$0, w1
L(L0):	mov	-8(up,n,8), %rax
	mul	v0
	mov	%rax, w2
	mov	%rdx, w3
	mov	w0, -16(rp,n,8)
	add	w1, w2
	adc	$0, w3
L(L3):	mov	(up,n,8), %rax
	mul	v0
	mov	%rax, w0
	mov	%rdx, w1
	mov	w2, -8(rp,n,8)
	add	w3, w0
	adc	$0, w1
L(L2):	mov	8(up,n,8), %rax
	mul	v0
	mov	%rax, w2
	mov	%rdx, w3
	mov	w0, (rp,n,8)
	add	w1, w2
	adc	$0, w3
	add	$4, n
	js	L(top)

L(end):	mov	w2, -8(rp)
	mov	w3, %rax
	pop	%rbx
IFDOS(`	pop	%rdi		')
IFDOS(`	pop	%rsi		')
	ret

	ALIGN(32)
L(n1):	mov	%rax, -8(rp)
	mov	%rdx, %rax
	pop	%rbx
IFDOS(`	pop	%rdi		')
IFDOS(`	pop	%rsi		')
	ret
EPILOGUE()

ASM_START()
	TEXT
	ALIGN(64)
PROLOGUE(mpn_mul_1c)
IFDOS(`	push	%rsi		')
IFDOS(`	push	%rdi		')
IFDOS(`	mov	%rdx, %rsi	')
	mov	cy, w2
	push	%rbx
	mov	(up), %rax

	lea	(rp,n_param,8), rp
	lea	(up,n_param,8), up
	mov	n_param, n

	test	$1, R8(n_param)
	jne	L(cx1)

L(cx0):	mul	v0
	neg	n
	mov	%rax, w0
	mov	%rdx, w1
	add	w2, w0
	adc	$0, w1
	test	$2, R8(n)
	jne	L(L2)

L(c00):	add	$2, n
	jmp	L(L0)

	ALIGN(16)
L(cx1):	mul	v0
	test	$2, R8(n)
	je	L(c01)

L(c11):	neg	n
	inc	n
	add	%rax, w2
	mov	%rdx, w3
	adc	$0, w3
	jmp	L(L3)

L(c01):	cmp	$1, n
	jz	L(m1)
	neg	n
	add	$3, n
	add	%rax, w2
	mov	%rdx, w3
	adc	$0, w3
	jmp	L(top)

	ALIGN(32)
L(m1):	add	%rax, w2
	mov	%rdx, %rax
	mov	w2, -8(rp)
	adc	$0, %rax
	pop	%rbx
IFDOS(`	pop	%rdi		')
IFDOS(`	pop	%rsi		')
	ret
EPILOGUE()
