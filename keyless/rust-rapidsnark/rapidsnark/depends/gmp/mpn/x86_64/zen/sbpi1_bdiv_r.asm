dnl  AMD64 mpn_sbpi1_bdiv_r optimised for AMD Zen

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


define(`up',       `%rdi')
define(`un_param', `%rsi')
define(`dp_param', `%rdx')
define(`dn_param', `%rcx')
define(`dinv',     `%r8')

define(`i',        `%rcx')
define(`dn',       `%r14')

define(`dp',       `%rsi')
define(`un',       `%r15')

C TODO
C  * The o1...o8  loops for special dn counts were naively hand-optimised by
C    folding the generic loops.  They can probably be tuned.  The speculative
C    quotient limb generation might not be in the optimal spot.
C  * Perhaps avoid late-in-loop jumps, e.g., lo0.
C  * Improve regalloc wrt dn_param/dn and un_param/un to save some moves.

C ABI_SUPPORT(DOS64)
C ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_sbpi1_bdiv_r)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), dinv	')
	push	%r15
	push	%r14
	push	%r13
	push	%r12
	push	%rbp
	push	%rbx

	sub	dn_param, un_param		C outer loop count
	mov	dn_param, dn		C FIXME: Suppress by reg re-alloc
	push	dinv				C keep dinv on stack
	mov	un_param, un		C FIXME: Suppress by reg re-alloc
	xor	R32(%rbp), R32(%rbp)

	lea	(dp_param,dn_param,8), dp

	mov	(up), %rdx
	imul	dinv, %rdx			C first quotient limb

	neg	dn
	lea	-32(up,dn_param,8), up

	test	$1, R8(dn_param)
	jnz	L(cx1)

L(cx0):	test	$2, R8(dn_param)
	jnz	L(b2)


C =============================================================================
L(b0):	cmp	$-4, dn
	jnz	L(gt4)

L(o4):	mulx(	-32,(dp), %r9, %r14)
	mulx(	-24,(dp), %r11, %r10)
	mulx(	-16,(dp), %r13, %r12)
	mulx(	-8,(dp), %rbx, %rax)
	add	%r14, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	(up), %r9
	adc	8(up), %r11
	mov	%r8, %rdx			C dinv
	mov	%r11, 8(up)
	mulx(	%r11, %rdx, %r12)		C next quotient
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o4)
	jmp	L(ret)

L(gt4):	cmp	$-8, dn
	jnz	L(out0)

L(o8):	mulx(	-64,(dp), %r9, %r14)
	mulx(	-56,(dp), %rcx, %r10)
	mulx(	-48,(dp), %r13, %r12)
	mulx(	-40,(dp), %rbx, %rax)
	add	%r14, %rcx
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	-32(up), %r9
	mulx(	-32,(dp), %r9, %r14)
	adc	-24(up), %rcx
	mov	%rcx, -24(up)
	mulx(	-24,(dp), %r11, %r10)
	adc	%r13, -16(up)
	mulx(	-16,(dp), %r13, %r12)
	adc	%rbx, -8(up)
	adc	%rax, %r9
	mulx(	-8,(dp), %rbx, %rax)
	adc	%r14, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	mov	%r8, %rdx			C dinv
	mulx(	%rcx, %rdx, %r12)		C next quotient
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o8)
	jmp	L(ret)

L(out0):mov	dn, i
	.byte	0xc4,0x22,0xb3,0xf6,0x04,0xf6		C mulx (dp,dn,8),%r9,%r8
	.byte	0xc4,0x22,0xa3,0xf6,0x54,0xf6,0x08	C mulx 8(dp,dn,8),%r11,%r10
	.byte	0xc4,0x22,0x93,0xf6,0x64,0xf6,0x10	C mulx 16(dp,dn,8),%r13,%r12
	clc
	jmp	L(lo0)

	ALIGN(16)
L(top0):add	%r9, (up,i,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (dp,i,8), %r9, %r8
	adc	%r11, 8(up,i,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(dp,i,8), %r11, %r10
	adc	%r13, 16(up,i,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(dp,i,8), %r13, %r12
	adc	%rbx, 24(up,i,8)
	adc	%rax, %r9
L(lo0):	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(dp,i,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, i
	js	L(top0)

	mov	(%rsp), %rdx			C dinv
	.byte	0xc4,0x22,0xeb,0xf6,0x64,0xf7,0x28	C mulx 40(%rdi,%r14,8),%rdx,%r12
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(out0)
	jmp	L(ret)

L(cx1):	test	$2, R8(dn_param)
	jnz	L(b3)

C =============================================================================
L(b1):	cmp	$-1, dn
	jnz	L(gt1)

	mov	24(up), %r9
L(o1):	mulx(	-8,(dp), %rbx, %rdx)
	add	%r9, %rbx
	adc	%rbp, %rdx
	add	32(up), %rdx
	setc	R8(%rbp)
	mov	%rdx, %r9
	mulx(	%r8, %rdx, %r12)		C next quotient
	lea	8(up), up
	dec	un
	jne	L(o1)
	mov	%r9, 24(up)
	jmp	L(ret)

L(gt1):	cmp	$-5, dn
	jnz	L(out1)

L(o5):	mulx(	-40,(dp), %rbx, %rax)
	mulx(	-32,(dp), %r9, %r14)
	mulx(	-24,(dp), %r11, %r10)
	mulx(	-16,(dp), %r13, %r12)
	add	-8(up), %rbx
	adc	%rax, %r9
	mulx(	-8,(dp), %rbx, %rax)
	adc	%r14, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	(up), %r9
	mov	%r9, (up)
	mov	%r8, %rdx			C dinv
	mulx(	%r9, %rdx, %r12)		C next quotient
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o5)
	jmp	L(ret)

L(out1):lea	1(dn), i
	.byte	0xc4,0xa2,0xe3,0xf6,0x04,0xf6		C mulx (dp,dn,8),%rbx,%rax
	.byte	0xc4,0x22,0xb3,0xf6,0x44,0xf6,0x08	C mulx 8(dp,dn,8),%r9,%r8
	.byte	0xc4,0x22,0xa3,0xf6,0x54,0xf6,0x10	C mulx 16(dp,dn,8),%r11,%r10
	clc
	jmp	L(lo1)

	ALIGN(16)
L(top1):add	%r9, (up,i,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (dp,i,8), %r9, %r8
	adc	%r11, 8(up,i,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(dp,i,8), %r11, %r10
	adc	%r13, 16(up,i,8)
L(lo1):	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(dp,i,8), %r13, %r12
	adc	%rbx, 24(up,i,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(dp,i,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, i
	js	L(top1)

	mov	(%rsp), %rdx			C dinv
	.byte	0xc4,0x22,0xeb,0xf6,0x64,0xf7,0x28	C mulx 40(up,dn,8), %rdx, %r12
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(out1)
	jmp	L(ret)

C =============================================================================
L(b2):	cmp	$-2, dn
	jnz	L(gt2)

	mov	16(up), %r10
	mov	24(up), %r9
L(o2):	mulx(	-16,(dp), %r13, %r12)
	mulx(	-8,(dp), %rbx, %rax)
	add	%r12, %rbx
	adc	$0, %rax
	add	%r10, %r13			C add just to produce carry
	mov	%r9, %r10
	adc	%rbx, %r10
	mov	%r8, %rdx
	mulx(	%r10, %rdx, %r12)		C next quotient
	adc	%rbp, %rax
	setc	R8(%rbp)
	mov	32(up), %r9
	add	%rax, %r9
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o2)
	mov	%r10, 16(up)
	mov	%r9, 24(up)
	jmp	L(ret)

L(gt2):	cmp	$-6, dn
	jnz	L(out2)

L(o6):	mulx(	-48,(dp), %r13, %r12)
	mulx(	-40,(dp), %rcx, %rax)
	add	%r12, %rcx
	adc	$0, %rax
	mulx(	-32,(dp), %r9, %r14)
	mulx(	-24,(dp), %r11, %r10)
	add	-16(up), %r13
	mulx(	-16,(dp), %r13, %r12)
	adc	-8(up), %rcx
	mov	%rcx, -8(up)
	adc	%rax, %r9
	mulx(	-8,(dp), %rbx, %rax)
	adc	%r14, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	mov	%r8, %rdx			C dinv
	mulx(	%rcx, %rdx, %r12)		C next quotient
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o6)
	jmp	L(ret)

L(out2):lea	2(dn), i
	.byte	0xc4,0x22,0x93,0xf6,0x24,0xf6		C mulx (dp,dn,8),%r13,%r12
	.byte	0xc4,0xa2,0xe3,0xf6,0x44,0xf6,0x08	C mulx 8(dp,dn,8),%rbx,%rax
	add	%r12, %rbx
	adc	$0, %rax
	.byte	0xc4,0x22,0xb3,0xf6,0x44,0xf6,0x10	C mulx 16(dp,dn,8),%r9,%r8
	jmp	L(lo2)

	ALIGN(16)
L(top2):add	%r9, (up,i,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (dp,i,8), %r9, %r8
	adc	%r11, 8(up,i,8)
L(lo2):	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(dp,i,8), %r11, %r10
	adc	%r13, 16(up,i,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(dp,i,8), %r13, %r12
	adc	%rbx, 24(up,i,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(dp,i,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, i
	js	L(top2)

	mov	(%rsp), %rdx			C dinv
	.byte	0xc4,0x22,0xeb,0xf6,0x64,0xf7,0x28	C mulx 40(up,dn,8), %rdx, %r12
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(out2)
	jmp	L(ret)

C =============================================================================
L(b3):	cmp	$-3, dn
	jnz	L(gt3)

	mov	8(up), %r14
	mov	16(up), %r9
	mov	24(up), %rcx
L(o3):	mulx(	-24,(dp), %r11, %r10)
	mulx(	-16,(dp), %r13, %r12)
	mulx(	-8,(dp), %rbx, %rax)
	add	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	%r14, %r11
	mov	%r9, %r14
	adc	%r13, %r14
	mov	%rcx, %r9
	mov	%r8, %rdx			C dinv
	mulx(	%r14, %rdx, %r12)		C next quotient
	adc	%rbx, %r9
	adc	%rbp, %rax
	setc	R8(%rbp)
	mov	32(up), %rcx
	add	%rax, %rcx
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o3)
	mov	%r14, 8(up)
	mov	%r9, 16(up)
	mov	%rcx, 24(up)
	jmp	L(ret)

L(gt3):	cmp	$-7, dn
	jnz	L(out3)

L(o7):	mulx(	-56,(dp), %r11, %r10)
	mulx(	-48,(dp), %rcx, %r12)
	mulx(	-40,(dp), %rbx, %rax)
	add	%r10, %rcx
	adc	%r12, %rbx
	adc	$0, %rax
	mulx(	-32,(dp), %r9, %r14)
	add	-24(up), %r11
	mulx(	-24,(dp), %r11, %r10)
	adc	-16(up), %rcx
	mov	%rcx, -16(up)
	mulx(	-16,(dp), %r13, %r12)
	adc	%rbx, -8(up)
	adc	%rax, %r9
	mulx(	-8,(dp), %rbx, %rax)
	adc	%r14, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	mov	%r8, %rdx			C dinv
	mulx(	%rcx, %rdx, %r12)		C next quotient
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(o7)
	jmp	L(ret)

L(out3):lea	3(dn), i
	.byte	0xc4,0x22,0xa3,0xf6,0x14,0xf6		C mulx (dp,dn,8),%r11,%r10
	.byte	0xc4,0x22,0x93,0xf6,0x64,0xf6,0x08	C mulx 8(dp,dn,8),%r13,%r12
	.byte	0xc4,0xa2,0xe3,0xf6,0x44,0xf6,0x10	C mulx 16(dp,dn,8),%rbx,%rax
	add	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	jmp	L(lo3)

	ALIGN(16)
L(top3):add	%r9, (up,i,8)
L(lo3):	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (dp,i,8), %r9, %r8
	adc	%r11, 8(up,i,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(dp,i,8), %r11, %r10
	adc	%r13, 16(up,i,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(dp,i,8), %r13, %r12
	adc	%rbx, 24(up,i,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(dp,i,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, i
	js	L(top3)

	mov	(%rsp), %rdx			C dinv
	.byte	0xc4,0x22,0xeb,0xf6,0x64,0xf7,0x28	C mulx 40(up,dn,8), %rdx, %r12
	add	%r9, (up)
	adc	%r11, 8(up)
	adc	%r13, 16(up)
	adc	%rbx, 24(up)
	adc	%rbp, %rax
	setc	R8(%rbp)
	add	%rax, 32(up)
	adc	$0, R32(%rbp)
	lea	8(up), up
	dec	un
	jne	L(out3)

L(ret):	mov	%rbp, %rax
	pop	%rsi			C dummy dealloc
	pop	%rbx
	pop	%rbp
	pop	%r12
	pop	%r13
	pop	%r14
	pop	%r15
	ret
EPILOGUE()
