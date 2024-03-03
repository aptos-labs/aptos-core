dnl  AMD64 mpn_bdiv_q_1, mpn_pi1_bdiv_q_1 -- Hensel division by 1-limb divisor.

dnl  Copyright 2001, 2002, 2004-2006, 2010-2012, 2017 Free Software Foundation,
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

C	    cycles/limb    cycles/limb
C	       norm	       unorm
C AMD K8,K9	11		11
C AMD K10	11		11
C AMD bull	13.5		14
C AMD pile	14		15
C AMD steam
C AMD excavator
C AMD bobcat	14		14
C AMD jaguar	14.5		15
C Intel P4	33		33
C Intel core2	13.5		13.25
C Intel NHM	14		14
C Intel SBR	8		8.25
C Intel IBR	7.75		7.85
C Intel HWL	8		8
C Intel BWL	8		8
C Intel SKL	8		8
C Intel atom	34		36
C Intel SLM	13.7		13.5
C VIA nano	19.25		19.25	needs re-measuring

C INPUT PARAMETERS
define(`rp',		`%rdi')
define(`up',		`%rsi')
define(`n',		`%rdx')
define(`d',		`%rcx')
define(`di',		`%r8')		C	just mpn_pi1_bdiv_q_1
define(`ncnt',		`%r9')		C	just mpn_pi1_bdiv_q_1

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(16)
PROLOGUE(mpn_bdiv_q_1)
	FUNC_ENTRY(4)
	push	%rbx

	mov	%rcx, %rax
	xor	R32(%rcx), R32(%rcx)	C ncnt count
	mov	%rdx, %r10

	bt	$0, R32(%rax)
	jnc	L(evn)			C skip bsf unless divisor is even

L(odd):	mov	%rax, %rbx
	shr	R32(%rax)
	and	$127, R32(%rax)		C d/2, 7 bits

	LEA(	binvert_limb_table, %rdx)

	movzbl	(%rdx,%rax), R32(%rax)	C inv 8 bits

	mov	%rbx, %r11		C d without twos

	lea	(%rax,%rax), R32(%rdx)	C 2*inv
	imul	R32(%rax), R32(%rax)	C inv*inv
	imul	R32(%rbx), R32(%rax)	C inv*inv*d
	sub	R32(%rax), R32(%rdx)	C inv = 2*inv - inv*inv*d, 16 bits

	lea	(%rdx,%rdx), R32(%rax)	C 2*inv
	imul	R32(%rdx), R32(%rdx)	C inv*inv
	imul	R32(%rbx), R32(%rdx)	C inv*inv*d
	sub	R32(%rdx), R32(%rax)	C inv = 2*inv - inv*inv*d, 32 bits

	lea	(%rax,%rax), %r8	C 2*inv
	imul	%rax, %rax		C inv*inv
	imul	%rbx, %rax		C inv*inv*d
	sub	%rax, %r8		C inv = 2*inv - inv*inv*d, 64 bits

	jmp	L(pi1)

L(evn):	bsf	%rax, %rcx
	shr	R8(%rcx), %rax
	jmp	L(odd)
EPILOGUE()

PROLOGUE(mpn_pi1_bdiv_q_1)
	FUNC_ENTRY(4)
IFDOS(`	mov	56(%rsp), %r8	')
IFDOS(`	mov	64(%rsp), %r9	')
	push	%rbx

	mov	%rcx, %r11		C d
	mov	%rdx, %r10		C n
	mov	%r9, %rcx		C ncnt

L(pi1):	mov	(up), %rax		C up[0]

	dec	%r10
	jz	L(one)

	lea	8(up,%r10,8), up	C up end
	lea	(rp,%r10,8), rp		C rp end
	neg	%r10			C -n

	test	R32(%rcx), R32(%rcx)
	jnz	L(unorm)		C branch if count != 0
	xor	R32(%rbx), R32(%rbx)
	jmp	L(nent)

	ALIGN(8)
L(ntop):mul	%r11			C carry limb in rdx	0 10
	mov	-8(up,%r10,8), %rax	C
	sub	%rbx, %rax		C apply carry bit
	setc	R8(%rbx)		C
	sub	%rdx, %rax		C apply carry limb	5
	adc	$0, R32(%rbx)		C			6
L(nent):imul	%r8, %rax		C			6
	mov	%rax, (rp,%r10,8)	C
	inc	%r10			C
	jnz	L(ntop)

	mov	-8(up), %r9		C up high limb
	jmp	L(com)

L(unorm):
	mov	(up,%r10,8), %r9	C up[1]
	shr	R8(%rcx), %rax		C
	neg	R32(%rcx)
	shl	R8(%rcx), %r9		C
	neg	R32(%rcx)
	or	%r9, %rax
	xor	R32(%rbx), R32(%rbx)
	jmp	L(uent)

	ALIGN(8)
L(utop):mul	%r11			C carry limb in rdx	0 10
	mov	(up,%r10,8), %rax	C
	shl	R8(%rcx), %rax		C
	neg	R32(%rcx)
	or	%r9, %rax
	sub	%rbx, %rax		C apply carry bit
	setc	R8(%rbx)		C
	sub	%rdx, %rax		C apply carry limb	5
	adc	$0, R32(%rbx)		C			6
L(uent):imul	%r8, %rax		C			6
	mov	(up,%r10,8), %r9	C
	shr	R8(%rcx), %r9		C
	neg	R32(%rcx)
	mov	%rax, (rp,%r10,8)	C
	inc	%r10			C
	jnz	L(utop)

L(com):	mul	%r11			C carry limb in rdx
	sub	%rbx, %r9		C apply carry bit
	sub	%rdx, %r9		C apply carry limb
	imul	%r8, %r9
	mov	%r9, (rp)
	pop	%rbx
	FUNC_EXIT()
	ret

L(one):	shr	R8(%rcx), %rax
	imul	%r8, %rax
	mov	%rax, (rp)
	pop	%rbx
	FUNC_EXIT()
	ret
EPILOGUE()
