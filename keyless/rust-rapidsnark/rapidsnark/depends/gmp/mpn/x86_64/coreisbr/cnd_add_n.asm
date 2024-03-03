dnl  AMD64 mpn_cnd_add_n.

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
C Intel NHM	 3.75
C Intel SBR	 1.93
C Intel IBR	 1.89
C Intel HWL	 1.78
C Intel BWL	 1.50
C Intel SKL	 1.50
C Intel atom
C Intel SLM	 4.0
C VIA nano

C NOTES
C  * It might seem natural to use the cmov insn here, but since this function
C    is supposed to have the exact same execution pattern for cnd true and
C    false, and since cmov's documentation is not clear about whether it
C    actually reads both source operands and writes the register for a false
C    condition, we cannot use it.

C INPUT PARAMETERS
define(`cnd_arg', `%rdi')	dnl rcx
define(`rp',	  `%rsi')	dnl rdx
define(`up',	  `%rdx')	dnl r8
define(`vp',	  `%rcx')	dnl r9
define(`n',	  `%r8')	dnl rsp+40

define(`cnd',     `%rbx')

define(ADDSUB,	add)
define(ADCSBB,	adc)
define(func,	mpn_cnd_add_n)

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_cnd_add_n)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), R32(%r8)')
	push	%rbx

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
	and	cnd, %r9
	and	cnd, %r10
	ADDSUB	(up), %rdi
	mov	%rdi, (rp)
	ADCSBB	8(up), %r9
	mov	%r9, 8(rp)
	ADCSBB	16(up), %r10
	mov	%r10, 16(rp)
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
	and	cnd, %rdi
	and	cnd, %r9
	ADDSUB	(up), %rdi
	mov	%rdi, (rp)
	ADCSBB	8(up), %r9
	mov	%r9, 8(rp)
	sbb	R32(%rax), R32(%rax)	C save carry
	lea	16(up), up
	lea	16(vp), vp
	lea	16(rp), rp
	sub	$2, n
	jnz	L(top)
	jmp	L(end)

L(b1):	mov	(vp), %rdi
	and	cnd, %rdi
	ADDSUB	(up), %rdi
	mov	%rdi, (rp)
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
	and	cnd, %r9
	and	cnd, %r10
	and	cnd, %r11
	add	R32(%rax), R32(%rax)	C restore carry
	ADCSBB	(up), %rdi
	mov	%rdi, (rp)
	ADCSBB	8(up), %r9
	mov	%r9, 8(rp)
	ADCSBB	16(up), %r10
	mov	%r10, 16(rp)
	ADCSBB	24(up), %r11
	lea	32(up), up
	mov	%r11, 24(rp)
	lea	32(rp), rp
	sbb	R32(%rax), R32(%rax)	C save carry
	sub	$4, n
	jnz	L(top)

L(end):	neg	R32(%rax)
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
