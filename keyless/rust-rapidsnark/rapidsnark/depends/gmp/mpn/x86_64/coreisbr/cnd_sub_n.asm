dnl  AMD64 mpn_cnd_add_n, mpn_cnd_sub_n

dnl  Copyright 2011-2013, 2017 Free Software Foundation, Inc.

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
C Intel PNR	 3.0
C Intel NHM	 2.75
C Intel SBR	 2.15
C Intel IBR	 1.96
C Intel HWL	 2.0
C Intel BWL	 1.65
C Intel SKL	 1.65
C Intel atom
C Intel SLM	 4.5
C VIA nano

C NOTES
C  * It might seem natural to use the cmov insn here, but since this function
C    is supposed to have the exact same execution pattern for cnd true and
C    false, and since cmov's documentation is not clear about whether it
C    actually reads both source operands and writes the register for a false
C    condition, we cannot use it.
C  * Given that we have a dedicated cnd_add_n, it might look strange that this
C    file provides cnd_add_n and not just cnd_sub_n.  But that's harmless, and
C    this file's generality might come in handy for some pipeline.

C INPUT PARAMETERS
define(`cnd_arg', `%rdi')	dnl rcx
define(`rp',	  `%rsi')	dnl rdx
define(`up',	  `%rdx')	dnl r8
define(`vp',	  `%rcx')	dnl r9
define(`n',	  `%r8')	dnl rsp+40

define(`cnd',     `%rbx')

ifdef(`OPERATION_cnd_add_n',`
	define(ADDSUB,	add)
	define(ADCSBB,	adc)
	define(func,	mpn_cnd_add_n)')
ifdef(`OPERATION_cnd_sub_n',`
	define(ADDSUB,	sub)
	define(ADCSBB,	sbb)
	define(func,	mpn_cnd_sub_n)')

MULFUNC_PROLOGUE(mpn_cnd_add_n mpn_cnd_sub_n)

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(func)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), R32(%r8)')
	push	%rbx
	push	%rbp
	push	%r12
	push	%r13

	neg	cnd_arg
	sbb	cnd, cnd		C make cnd mask

	test	$1, R8(n)
	jz	L(x0)
L(x1):	test	$2, R8(n)
	jz	L(b1)

L(b3):	mov	(vp), %rdi
	mov	8(vp), %r9
	mov	16(vp), %r10
	and	cnd, %rdi
	mov	(up), %r12
	and	cnd, %r9
	mov	8(up), %r13
	and	cnd, %r10
	mov	16(up), %rbp
	ADDSUB	%rdi, %r12
	mov	%r12, (rp)
	ADCSBB	%r9, %r13
	mov	%r13, 8(rp)
	ADCSBB	%r10, %rbp
	mov	%rbp, 16(rp)
	sbb	R32(%rax), R32(%rax)	C save carry
	lea	24(up), up
	lea	24(vp), vp
	lea	24(rp), rp
	sub	$3, n
	jnz	L(top)
	jmp	L(end)

L(x0):	xor	R32(%rax), R32(%rax)
	test	$2, R8(n)
	jz	L(top)

L(b2):	mov	(vp), %rdi
	mov	8(vp), %r9
	mov	(up), %r12
	and	cnd, %rdi
	mov	8(up), %r13
	and	cnd, %r9
	ADDSUB	%rdi, %r12
	mov	%r12, (rp)
	ADCSBB	%r9, %r13
	mov	%r13, 8(rp)
	sbb	R32(%rax), R32(%rax)	C save carry
	lea	16(up), up
	lea	16(vp), vp
	lea	16(rp), rp
	sub	$2, n
	jnz	L(top)
	jmp	L(end)

L(b1):	mov	(vp), %rdi
	mov	(up), %r12
	and	cnd, %rdi
	ADDSUB	%rdi, %r12
	mov	%r12, (rp)
	sbb	R32(%rax), R32(%rax)	C save carry
	lea	8(up), up
	lea	8(vp), vp
	lea	8(rp), rp
	dec	n
	jz	L(end)

	ALIGN(16)
L(top):	mov	(vp), %rdi
	mov	8(vp), %r9
	mov	16(vp), %r10
	mov	24(vp), %r11
	lea	32(vp), vp
	and	cnd, %rdi
	mov	(up), %r12
	and	cnd, %r9
	mov	8(up), %r13
	and	cnd, %r10
	mov	16(up), %rbp
	and	cnd, %r11
	add	R32(%rax), R32(%rax)	C restore carry
	mov	24(up), %rax
	lea	32(up), up
	ADCSBB	%rdi, %r12
	mov	%r12, (rp)
	ADCSBB	%r9, %r13
	mov	%r13, 8(rp)
	ADCSBB	%r10, %rbp
	mov	%rbp, 16(rp)
	ADCSBB	%r11, %rax
	mov	%rax, 24(rp)
	lea	32(rp), rp
	sbb	R32(%rax), R32(%rax)	C save carry
	sub	$4, n
	jnz	L(top)

L(end):	neg	R32(%rax)
	pop	%r13
	pop	%r12
	pop	%rbp
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
