dnl  X64-64 mpn_mullo_basecase optimised for AMD Zen.

dnl  Contributed to the GNU project by Torbjorn Granlund.

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

C The inner loops of this code are the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

define(`rp',	   `%rdi')
define(`up',	   `%rsi')
define(`vp_param', `%rdx')
define(`n',	   `%rcx')

define(`vp',	`%r11')
define(`nn',    `%rbp')

C TODO
C  * Rearrange feed-in jumps for short branch forms.
C  * Roll out the heavy artillery and 4-way unroll outer loop.  Since feed-in
C    code implodes, the blow-up will not be more than perhaps 2.5x.
C  * Micro-optimise critical lead-in code blocks.
C  * Clean up register use, e.g. r15 vs vp, disuse of nn, etc.
C  * Write n < 4 code specifically for Zen (current code is for Haswell).

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_mullo_basecase)
	FUNC_ENTRY(4)
	cmp	$4, R32(n)
	jae	L(big)

	mov	vp_param, vp
	mov	(up), %rdx

	cmp	$2, R32(n)
	jae	L(gt1)
L(n1):	imul	(vp), %rdx
	mov	%rdx, (rp)
	FUNC_EXIT()
	ret
L(gt1):	ja	L(gt2)
L(n2):	mov	(vp), %r9
	mulx(	%r9, %rax, %rdx)
	mov	%rax, (rp)
	mov	8(up), %rax
	imul	%r9, %rax
	add	%rax, %rdx
	mov	8(vp), %r9
	mov	(up), %rcx
	imul	%r9, %rcx
	add	%rcx, %rdx
	mov	%rdx, 8(rp)
	FUNC_EXIT()
	ret
L(gt2):
L(n3):	mov	(vp), %r9
	mulx(	%r9, %rax, %r10)	C u0 x v0
	mov	%rax, (rp)
	mov	8(up), %rdx
	mulx(	%r9, %rax, %rdx)	C u1 x v0
	imul	16(up), %r9		C u2 x v0
	add	%rax, %r10
	adc	%rdx, %r9
	mov	8(vp), %r8
	mov	(up), %rdx
	mulx(	%r8, %rax, %rdx)	C u0 x v1
	add	%rax, %r10
	adc	%rdx, %r9
	imul	8(up), %r8		C u1 x v1
	add	%r8, %r9
	mov	%r10, 8(rp)
	mov	16(vp), %r10
	mov	(up), %rax
	imul	%rax, %r10		C u0 x v2
	add	%r10, %r9
	mov	%r9, 16(rp)
	FUNC_EXIT()
	ret

	ALIGN(16)
L(big):	push	%r15
	push	%r14
	push	%r13
	push	%r12
	push	%rbp
	push	%rbx

	mov	(up), %r9
	lea	-8(up,n,8), up
	lea	-40(rp,n,8), rp

	mov	$4, R32(%r14)
	sub	n, %r14
	mov	-8(vp_param,n,8), %rbp
	imul	%r9, %rbp
	lea	8(vp_param), %r15
	mov	(vp_param), %rdx

	test	$1, R8(%r14)
	jnz	L(mx0)
L(mx1):	test	$2, R8(%r14)
	jz	L(mb3)

L(mb1):	mulx(	%r9, %rbx, %rax)
	lea	-2(%r14), n
	.byte	0xc4,0x22,0xb3,0xf6,0x44,0xf6,0xf0	C mulx -0x10(%rsi,%r14,8),%r9,%r8
	.byte	0xc4,0x22,0xa3,0xf6,0x54,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%r11,%r10
	jmp	L(mlo1)

L(mb3):	mulx(	%r9, %r11, %r10)
	.byte	0xc4,0x22,0x93,0xf6,0x64,0xf6,0xf0	C mulx -0x10(%rsi,%r14,8),%r13,%r12
	.byte	0xc4,0xa2,0xe3,0xf6,0x44,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%rbx,%rax
	lea	(%r14), n
	jrcxz	L(x)
	jmp	L(mlo3)
L(x):	jmp	L(mcor)

L(mb2):	mulx(	%r9, %r13, %r12)
	.byte	0xc4,0xa2,0xe3,0xf6,0x44,0xf6,0xf0	C mulx -0x10(%rsi,%r14,8),%rbx,%rax
	lea	-1(%r14), n
	.byte	0xc4,0x22,0xb3,0xf6,0x44,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%r9,%r8
	jmp	L(mlo2)

L(mx0):	test	$2, R8(%r14)
	jz	L(mb2)

L(mb0):	mulx(	%r9, %r9, %r8)
	.byte	0xc4,0x22,0xa3,0xf6,0x54,0xf6,0xf0	C mulx -0x10(%rsi,%r14,8),%r11,%r10
	.byte	0xc4,0x22,0x93,0xf6,0x64,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%r13,%r12
	lea	-3(%r14), n
	jmp	L(mlo0)

	ALIGN(16)
L(mtop):jrcxz	L(mend)
	adc	%r8, %r11
	mov	%r9, (rp,n,8)
L(mlo3):.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r10, %r13
	mov	%r11, 8(rp,n,8)
L(mlo2):.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r12, %rbx
	mov	%r13, 16(rp,n,8)
L(mlo1):.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rax, %r9
	mov	%rbx, 24(rp,n,8)
L(mlo0):.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	lea	4(n), n
	jmp	L(mtop)

L(mend):mov	%r9, (rp)
	adc	%r8, %r11
	mov	%r11, 8(rp)
	adc	%r10, %r13
	mov	%r13, 16(rp)
	adc	%r12, %rbx
	mov	%rbx, 24(rp)

L(outer):
	mulx(	(up), %r10, %r8)	C FIXME r8 unused (use imul?)
	adc	%rax, %rbp
	add	%r10, %rbp
	mov	(%r15), %rdx
	add	$8, %r15
	mov	-24(up,%r14,8), %r8
	lea	-8(up), up

	test	$1, R8(%r14)
	jz	L(x0)
L(x1):	test	$2, R8(%r14)
	jnz	L(b3)

L(b1):	mulx(	%r8, %rbx, %rax)
	lea	-1(%r14), n
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (%rsi,%rcx,8),%r9,%r8
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 0x8(%rsi,%rcx,8),%r11,%r10
	jmp	L(lo1)

L(x0):	test	$2, R8(%r14)
	jz	L(b2)

L(b0):	mulx(	%r8, %r9, %r8)
	lea	-2(%r14), n
	.byte	0xc4,0x22,0xa3,0xf6,0x54,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%r11,%r10
	.byte	0xc4,0x22,0x93,0xf6,0x24,0xf6		C mulx (%rsi,%r14,8),%r13,%r12
	jmp	L(lo0)

L(b3):	mulx(	%r8, %r11, %r10)
	lea	1(%r14), n
	.byte	0xc4,0x22,0x93,0xf6,0x64,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%r13,%r12
	.byte	0xc4,0xa2,0xe3,0xf6,0x04,0xf6		C mulx (%rsi,%r14,8),%rbx,%rax
	add	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	jrcxz	L(cor)
	jmp	L(lo3)

L(cor):	add	8(rp), %r11
	mov	16(rp), %r10
	mov	24(rp), %r12
L(mcor):mov	%r11, 8(rp)
	adc	%r10, %r13
	adc	%r12, %rbx
	mulx(	(up), %r10, %r8)	C FIXME r8 unused (use imul?)
	adc	%rax, %rbp
	add	%r10, %rbp
	mov	(%r15), %rdx
	mov	-24(up), %r8
	mulx(	%r8, %r9, %r12)
	mulx(	-16,(up), %r14, %rax)
	add	%r12, %r14
	adc	$0, %rax
	adc	%r9, %r13
	mov	%r13, 16(rp)
	adc	%r14, %rbx
	mulx(	-8,(up), %r10, %r8)	C FIXME r8 unused (use imul?)
	adc	%rax, %rbp
	add	%r10, %rbp
	mov	8(%r15), %rdx
	mulx(	-24,(up), %r14, %rax)
	add	%r14, %rbx
	mov	%rbx, 24(rp)
	mulx(	-16,(up), %r10, %r8)	C FIXME r8 unused (use imul?)
	adc	%rax, %rbp
	add	%r10, %rbp
	mov	%rbp, 32(rp)
	pop	%rbx
	pop	%rbp
	pop	%r12
	pop	%r13
	pop	%r14
	pop	%r15
	FUNC_EXIT()
	ret

L(b2):	mulx(	%r8, %r13, %r12)
	lea	(%r14), n
	.byte	0xc4,0xa2,0xe3,0xf6,0x44,0xf6,0xf8	C mulx -0x8(%rsi,%r14,8),%rbx,%rax
	add	%r12, %rbx
	adc	$0, %rax
	.byte	0xc4,0x22,0xb3,0xf6,0x04,0xf6		C mulx (%rsi,%r14,8),%r9,%r8
	jmp	L(lo2)

	ALIGN(16)
L(top):	add	%r9, (rp,n,8)
L(lo3):	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r11, 8(rp,n,8)
L(lo2):	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r13, 16(rp,n,8)
L(lo1):	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rbx, 24(rp,n,8)
	adc	%rax, %r9
L(lo0):	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, n
	js	L(top)

	add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	%r13, 16(rp)
	adc	%rbx, 24(rp)
	inc	%r14
	jmp	L(outer)
EPILOGUE()
