dnl  AMD64 mpn_sqr_basecase optimised for AMD Zen.

dnl  Copyright 2012, 2013, 2017 Free Software Foundation, Inc.

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

C TODO
C  * Do overlapped software pipelining.  This should close the remaining gap to
C    mul_basecase.
C
C  * Update un just once in the outer loop.
C
C  * Perhaps keep un and n pre-multiplied by 8, thus suppressing ",8" from
C    loads and stores.  At least in some cases, the non-scaled form is faster.
C
C  * Optimise xit3 code, e.g., using shrx and sarx like in the main loop.
C
C  * The mul_1 feed-in code has gotten little attention and could probably be
C    improved.  Perhaps even expand it to 4 separate loops to allow straight
C    fall-through into the 4 addmul_1 loops.
C
C  * Clean up ad-hoc scratch register usage in the addmul_1 feed-in code blocks.

define(`rp',      `%rdi')
define(`up',      `%rsi')
define(`un_param',`%rdx')

define(`un',      `%rbp')
define(`n',       `%rcx')

C these are used just for the small op code
define(`w0',	`%r8')
define(`w1',	`%r9')
define(`w2',	`%r10')
define(`w3',	`%r11')


ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_sqr_basecase)
	FUNC_ENTRY(3)

	cmp	$2, R32(un_param)
	jae	L(gt1)

	mov	(up), %rdx
	mulx(	%rdx, %rax, %rdx)
	mov	%rax, (rp)
	mov	%rdx, 8(rp)
	FUNC_EXIT()
	ret

L(gt1):	jne	L(gt2)

	mov	(up), %rdx
	mov	8(up), %rcx
	mulx(	%rcx, %r9, %r10)	C v0 * v1	W 1 2
	mulx(	%rdx, %rax, %r8)	C v0 * v0	W 0 1
	mov	%rcx, %rdx
	mulx(	%rdx, %r11, %rdx)	C v1 * v1	W 2 3
	add	%r9, %r9		C		W 1
	adc	%r10, %r10		C		W 2
	adc	$0, %rdx		C		W 3
	add	%r9, %r8		C W 1
	adc	%r11, %r10		C W 2
	adc	$0, %rdx		C W 3
	mov	%rax, (rp)
	mov	%r8, 8(rp)
	mov	%r10, 16(rp)
	mov	%rdx, 24(rp)
	FUNC_EXIT()
	ret

L(gt2):	cmp	$4, R32(un_param)
	jae	L(gt3)

	push	%rbx
	mov	(up), %rdx
	mulx(	8,(up), w2, w3)
	mulx(	16,(up), w0, w1)
	add	w3, w0
	mov	8(up), %rdx
	mulx(	16,(up), %rax, w3)
	adc	%rax, w1
	adc	$0, w3
	test	R32(%rbx), R32(%rbx)
	mov	(up), %rdx
	mulx(	%rdx, %rbx, %rcx)
	mov	%rbx, (rp)
	mov	8(up), %rdx
	mulx(	%rdx, %rax, %rbx)
	mov	16(up), %rdx
	mulx(	%rdx, %rsi, %rdx)
	adcx(	w2, w2)
	adcx(	w0, w0)
	adcx(	w1, w1)
	adcx(	w3, w3)
	adox(	w2, %rcx)
	adox(	w0, %rax)
	adox(	w1, %rbx)
	adox(	w3, %rsi)
	mov	$0, R32(%r8)
	adox(	%r8, %rdx)
	adcx(	%r8, %rdx)
	mov	%rcx, 8(rp)
	mov	%rax, 16(rp)
	mov	%rbx, 24(rp)
	mov	%rsi, 32(rp)
	mov	%rdx, 40(rp)
	pop	%rbx
	FUNC_EXIT()
	ret

L(gt3):	push	%r15
C	push	%r14
	push	%r13
	push	%r12
	push	%rbp
	push	%rbx
	mov	R32(un_param), R32(un)

	mov	(up), %rdx		C up[0]
	mov	8(up), %r9		C up[1]

	mulx(	%rdx, %rax, %r15)	C up[0]^2
	mov	%rax, (rp)
	shl	%rdx

	lea	(up,un,8), up
	lea	-32(rp,un,8), rp

	neg	un
	lea	4(un), n
	and	$-4, n

	test	$1, R8(un)
	jnz	L(mx0)
L(mx1):	test	$2, R8(un)
	jz	L(mb3)

L(mb1):	mulx(	%r9, %rbx, %rax)
	.byte	0xc4,0x62,0xb3,0xf6,0x44,0xee,0x10	C mulx 16(up,un,8), %r9, %r8
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xee,0x18	C mulx 24(up,un,8), %r11, %r10
	add	%r15, %rbx
	jmp	L(mlo1)

L(mb3):	mulx(	%r9, %r11, %r10)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xee,0x10	C mulx 16(up,un,8), %r13, %r12
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xee,0x18	C mulx 24(up,un,8), %rbx, %rax
	add	%r15, %r11
	jrcxz	L(n4)
	jmp	L(mlo3)
L(n4):	mov	%r11, 8(rp)
	adc	%r10, %r13
	adc	%r12, %rbx
	jmp	L(m)

L(mx0):	test	$2, R8(un)
	jnz	L(mb0)

L(mb2):	mulx(	%r9, %r13, %r12)
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xee,0x10	C mulx 16(up,un,8), %rbx, %rax
	.byte	0xc4,0x62,0xb3,0xf6,0x44,0xee,0x18	C mulx 24(up,un,8), %r9, %r8
	add	%r15, %r13
	jmp	L(mlo2)

L(mb0):	mulx(	%r9, %r9, %r8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xee,0x10	C mulx 16(up,un,8), %r11, %r10
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xee,0x18	C mulx 24(up,un,8), %r13, %r12
	add	%r15, %r9
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
	adc	$0, %rax
	mov	%rbx, 24(rp)
	mov	%rax, 32(rp)

	lea	2(un), un

	mov	$63, R32(%r15)			C keep at 63 for shrx/sarx.
	test	$1, R8(un)
	jz	L(x0)
L(x1):	test	$2, R8(un)
	jz	L(f3)
	jmp	L(f1)
L(x0):	test	$2, R8(un)
	jz	L(f0)
C	jmp	L(f2)

L(f2):	mov	-8(up,un,8), %rdx		C up[0]
	lea	2(un), n
	lea	8(rp), rp
	.byte	0xc4,0x62,0x82,0xf7,0x5c,0xee,0xf0	C sarx %r15, -16(up,un,8), %r11
	.byte	0xc4,0x62,0x83,0xf7,0x6c,0xee,0xf0	C shrx %r15, -16(up,un,8), %r13
	and	%rdx, %r11			C "ci" in C code
	mulx(	%rdx, %rax, %r10)		C up[0]^2
	lea	(%r13,%rdx,2), %rdx		C "u0" arg in C code
	add	%rax, %r11

	.byte	0xc4,0x62,0x93,0xf6,0x24,0xee		C mulx (up,un,8), %r13, %r12
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xee,0x08	C mulx 8(up,un,8), %rbx, %rax
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	jmp	L(b2)

	ALIGN(16)
L(top2):add	%r9, (rp,n,8)
L(b2):	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r11, 8(rp,n,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r13, 16(rp,n,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rbx, 24(rp,n,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, n
	jnz	L(top2)

	inc	un
	add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	%r13, 16(rp)
	adc	%rbx, 24(rp)
	adc	$0, %rax
	mov	%rax, 32(rp)

L(f1):	mov	-8(up,un,8), %rdx		C up[0]
	lea	1(un), n
	lea	8(rp), rp
	.byte	0xc4,0x62,0x82,0xf7,0x6c,0xee,0xf0	C sarx	%r15, -16(up,un,8), %r13
	.byte	0xc4,0xe2,0x83,0xf7,0x5c,0xee,0xf0	C shrx	%r15, -16(up,un,8), %rbx
	and	%rdx, %r13			C "ci" in C code
	mulx(	%rdx, %rax, %r12)		C up[0]^2
	lea	(%rbx,%rdx,2), %rdx		C "u0" arg in C code
	add	%rax, %r13

	.byte	0xc4,0xe2,0xe3,0xf6,0x04,0xee		C mulx (up,un,8), %rbx, %rax
	adc	%r12, %rbx
	adc	$0, %rax
	.byte	0xc4,0x62,0xb3,0xf6,0x44,0xee,0x08	C mulx 8(up,un,8), %r9, %r8
	jmp	L(b1)

	ALIGN(16)
L(top1):add	%r9, (rp,n,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r11, 8(rp,n,8)
L(b1):	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r13, 16(rp,n,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rbx, 24(rp,n,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, n
	jnz	L(top1)

	inc	un
	add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	%r13, 16(rp)
	adc	%rbx, 24(rp)
	adc	$0, %rax
	mov	%rax, 32(rp)

L(f0):	mov	-8(up,un,8), %rdx		C up[0]
	lea	(un), n
	lea	8(rp), rp
	.byte	0xc4,0xe2,0x82,0xf7,0x5c,0xee,0xf0	C sarx	%r15, -16(up,un,8), %rbx
	.byte	0xc4,0x62,0x83,0xf7,0x4c,0xee,0xf0	C shrx	%r15, -16(up,un,8), %r9
	and	%rdx, %rbx			C "ci" in C code
	mulx(	%rdx, %r10, %rax)		C up[0]^2
	lea	(%r9,%rdx,2), %rdx		C "u0" arg in C code
	add	%r10, %rbx
	adc	$0, %rax			C "cin" in C code

	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,un,8), %r9, %r8
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xee,0x08	C mulx 8(up,un,8), %r11, %r10
	jmp	L(b0)

	ALIGN(16)
L(top0):add	%r9, (rp,n,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r11, 8(rp,n,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r13, 16(rp,n,8)
L(b0):	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rbx, 24(rp,n,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, n
	jnz	L(top0)

	inc	un
	add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	%r13, 16(rp)
	adc	%rbx, 24(rp)
	adc	$0, %rax
	mov	%rax, 32(rp)

L(f3):	mov	-8(up,un,8), %rdx		C up[0]
	lea	3(un), n
	lea	8(rp), rp
	.byte	0xc4,0x62,0x82,0xf7,0x4c,0xee,0xf0	C sarx %r15, -16(up,un,8), %r9
	.byte	0xc4,0x62,0x83,0xf7,0x5c,0xee,0xf0	C shrx %r15, -16(up,un,8), %r11
	and	%rdx, %r9			C "ci" in C code
	mulx(	%rdx, %rax, %r8)		C up[0]^2
	lea	(%r11,%rdx,2), %rdx		C "u0" arg in C code
	add	%rax, %r9

	.byte	0xc4,0x62,0xa3,0xf6,0x14,0xee		C mulx (%rsi,%rbp,8),%r11,%r10
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xee,0x08	C mulx 0x8(%rsi,%rbp,8),%r13,%r12
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xee,0x10	C mulx 0x10(%rsi,%rbp,8),%rbx,%rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	jrcxz	L(xit3)
	jmp	L(top3)			C FIXME perhaps fall through

	ALIGN(16)
L(top3):add	%r9, (rp,n,8)
	.byte	0xc4,0x62,0xb3,0xf6,0x04,0xce		C mulx (up,n,8), %r9, %r8
	adc	%r11, 8(rp,n,8)
	.byte	0xc4,0x62,0xa3,0xf6,0x54,0xce,0x08	C mulx 8(up,n,8), %r11, %r10
	adc	%r13, 16(rp,n,8)
	.byte	0xc4,0x62,0x93,0xf6,0x64,0xce,0x10	C mulx 16(up,n,8), %r13, %r12
	adc	%rbx, 24(rp,n,8)
	adc	%rax, %r9
	.byte	0xc4,0xe2,0xe3,0xf6,0x44,0xce,0x18	C mulx 24(up,n,8), %rbx, %rax
	adc	%r8, %r11
	adc	%r10, %r13
	adc	%r12, %rbx
	adc	$0, %rax
	add	$4, n
	jnz	L(top3)

	inc	un
	add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	%r13, 16(rp)
	adc	%rbx, 24(rp)
	adc	$0, %rax
	mov	%rax, 32(rp)
	jmp	L(f2)


L(xit3):add	%r9, (rp)
	adc	%r11, 8(rp)
	adc	16(rp), %r13
	adc	24(rp), %rbx
L(m):	adc	$0, %rax
	mov	%rax, 32(rp)
	mov	-24(up), %rdx		C FIXME: CSE
	mov	-32(up), %r9		C FIXME: CSE
	sar	$63, %r9
	and	%rdx, %r9
	add	%r13, %r9
	mulx(	%rdx, %rax, %r10)
	mov	-16(up), %r8		C FIXME: CSE
	adc	$0, %r10
	add	%rax, %r9
	adc	$0, %r10
	mov	%r9, 16(rp)
	mov	-32(up), %rax
	shl	%rax
	adc	%rdx, %rdx
	mulx(	%r8, %r13, %r12)
	mulx(	-8,(up), %r11, %rax)	C FIXME: CSE
	add	%r10, %r13
	adc	%r12, %r11
	adc	$0, %rax
	add	%rbx, %r13
	mov	%r13, 24(rp)
	adc	32(rp), %r11
	adc	$0, %rax
	mov	-16(up), %rdx		C FIXME: CSE
	mov	-8(up), %r8		C FIXME: CSE
	mov	-24(up), %r9
	sar	$63, %r9
	and	%rdx, %r9
	add	%r11, %r9
	mulx(	%rdx, %rbp, %r10)
	adc	$0, %r10
	add	%rbp, %r9
	adc	$0, %r10
	mov	%r9, 32(rp)
	mov	-24(up), %rbp
	shl	%rbp
	adc	%rdx, %rdx
	mulx(	%r8, %rbx, %rbp)
	add	%r10, %rbx
	adc	$0, %rbp
	adc	%rbx, %rax
	mov	%rax, 40(rp)
	adc	$0, %rbp
	mov	-8(up), %rdx		C FIXME: CSE
	mov	-16(up), %r9		C FIXME: CSE
	sar	$63, %r9
	and	%rdx, %r9
	add	%rbp, %r9
	mulx(	%rdx, %rbp, %r10)
	adc	$0, %r10
	add	%rbp, %r9
	adc	$0, %r10
	mov	%r9, 48(rp)
	mov	%r10, 56(rp)

	pop	%rbx
	pop	%rbp
	pop	%r12
	pop	%r13
C	pop	%r14
	pop	%r15

	FUNC_EXIT()
	ret
EPILOGUE()
