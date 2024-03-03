dnl  AMD64 SSSE3/XOP mpn_hamdist -- hamming distance.

dnl  Copyright 2010-2017 Free Software Foundation, Inc.

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

C		    cycles/limb	  good for cpu?
C AMD K8,K9		n/a
C AMD K10		n/a
C AMD bd1	     1.51-2.0		y
C AMD bd2	     1.50-1.9		y
C AMD bd3		 ?
C AMD bd4		 ?
C AMD zen		n/a
C AMD bobcat		n/a
C AMD jaguar		n/a
C Intel P4		n/a
C Intel PNR		n/a
C Intel NHM		n/a
C Intel SBR		n/a
C Intel IBR		n/a
C Intel HWL		n/a
C Intel BWL		n/a
C Intel SKL		n/a
C Intel atom		n/a
C Intel SLM		n/a
C VIA nano		n/a

C TODO
C  * We need to use .byte for vpshlb, vpperm, vphaddubq, and all popcnt if we
C    intend to support old systems.

C We use vpshlb and vpperm below, which are XOP extensions to AVX.  Some
C systems, e.g., NetBSD, set OSXSAVE but nevertheless trigger SIGILL for AVX.
C We fall back to the core2 code.
ifdef(`GMP_AVX_NOT_REALLY_AVAILABLE',`
MULFUNC_PROLOGUE(mpn_hamdist)
include_mpn(`x86_64/core2/hamdist.asm')
',`

define(`up',		`%rdi')
define(`vp',		`%rsi')
define(`n',		`%rdx')

ABI_SUPPORT(DOS64)
ABI_SUPPORT(STD64)

ASM_START()
	TEXT
	ALIGN(32)
PROLOGUE(mpn_hamdist)
	FUNC_ENTRY(3)
	cmp	$5, n
	jl	L(sma)

	lea	L(cnsts)(%rip), %r9

	xor	R32(%r10), R32(%r10)
	test	$8, R8(vp)
	jz	L(ali)
	mov	(up), %r8
	xor	(vp), %r8
	add	$8, up
	add	$8, vp
	dec	n
	popcnt	%r8, %r10
L(ali):

ifdef(`PIC', `define(`OFF1',16) define(`OFF2',32) define(`OFF3',48)',
	     `define(`OFF1',32) define(`OFF2',48) define(`OFF3',64)')
	movdqa	OFF1`'(%r9), %xmm7	C nibble counts table
	movdqa	OFF2`'(%r9), %xmm6	C splat shift counts
	movdqa	OFF3`'(%r9), %xmm5	C masks
	pxor	%xmm4, %xmm4
	pxor	%xmm8, %xmm8		C grand total count

	mov	R32(n), R32(%rax)
	and	$6, R32(%rax)
	lea	-64(up,%rax,8), up
	lea	-64(vp,%rax,8), vp
ifdef(`PIC',`
	movslq	(%r9,%rax,2), %r11
	add	%r9, %r11
	jmp	*%r11
',`
	jmp	*(%r9,%rax,4)
')

L(0):	add	$64, up
	add	$64, vp
	sub	$2, n

	ALIGN(32)
L(top):	lddqu	(up), %xmm0
	pxor	(vp), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm5, %xmm0
	pand	%xmm5, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0,%xmm7,%xmm7,%xmm2
	.byte	0x8f,0xe8,0x40,0xa3,0xdf,0x10	C vpperm %xmm1,%xmm7,%xmm7,%xmm3
	paddb	%xmm2, %xmm3
	paddb	%xmm3, %xmm4
L(6):	lddqu	16(up), %xmm0
	pxor	16(vp), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm5, %xmm0
	pand	%xmm5, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0,%xmm7,%xmm7,%xmm2
	.byte	0x8f,0xe8,0x40,0xa3,0xdf,0x10	C vpperm %xmm1,%xmm7,%xmm7,%xmm3
	paddb	%xmm2, %xmm3
	paddb	%xmm3, %xmm4
L(4):	lddqu	32(up), %xmm0
	pxor	32(vp), %xmm0
	.byte	0x8f,0xe9,0x48,0x94,0xc8	C vpshlb %xmm6, %xmm0, %xmm1
	pand	%xmm5, %xmm0
	pand	%xmm5, %xmm1
	.byte	0x8f,0xe8,0x40,0xa3,0xd7,0x00	C vpperm %xmm0,%xmm7,%xmm7,%xmm2
	.byte	0x8f,0xe9,0x78,0xd3,0xc4	C vphaddubq %xmm4, %xmm0
	.byte	0x8f,0xe8,0x40,0xa3,0xe7,0x10	C vpperm %xmm1,%xmm7,%xmm7,%xmm4
	paddb	%xmm2, %xmm3
	paddb	%xmm2, %xmm4
	paddq	%xmm0, %xmm8		C sum to 2 x 64-bit counts
L(2):	mov	48(up), %r8
	mov	56(up), %r9
	add	$64, up
	xor	48(vp), %r8
	xor	56(vp), %r9
	add	$64, vp
	popcnt	%r8, %r8
	popcnt	%r9, %r9
	add	%r8, %r10
	add	%r9, %r10
	sub	$8, n
	jg	L(top)

	test	$1, R8(n)
	jz	L(x)
	mov	(up), %r8
	xor	(vp), %r8
	popcnt	%r8, %r8
	add	%r8, %r10
L(x):	.byte	0x8f,0xe9,0x78,0xd3,0xc4	C vphaddubq %xmm4, %xmm0
	paddq	%xmm0, %xmm8
	pshufd	$14, %xmm8, %xmm0
	paddq	%xmm8, %xmm0
	movq	%xmm0, %rax
	add	%r10, %rax
	FUNC_EXIT()
	ret

L(sma):	mov	(up), %r8
	xor	(vp), %r8
	popcnt	%r8, %rax
	dec	n
	jz	L(ed)
L(tp):	mov	8(up), %r8
	add	$8, up
	xor	8(vp), %r8
	add	$8, vp
	popcnt	%r8, %r8
	add	%r8, %rax
	dec	n
	jnz	L(tp)
L(ed):	FUNC_EXIT()
	ret
EPILOGUE()
DEF_OBJECT(L(cnsts),16,`JUMPTABSECT')
	JMPENT(	L(0), L(cnsts))
	JMPENT(	L(2), L(cnsts))
	JMPENT(	L(4), L(cnsts))
	JMPENT(	L(6), L(cnsts))
	.byte	0x00,0x01,0x01,0x02,0x01,0x02,0x02,0x03
	.byte	0x01,0x02,0x02,0x03,0x02,0x03,0x03,0x04
	.byte	-4,-4,-4,-4,-4,-4,-4,-4
	.byte	-4,-4,-4,-4,-4,-4,-4,-4
	.byte	0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f
	.byte	0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f,0x0f
END_OBJECT(L(cnsts))
')
