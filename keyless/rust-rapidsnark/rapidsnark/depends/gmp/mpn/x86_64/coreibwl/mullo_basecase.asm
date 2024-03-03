dnl  X64-64 mpn_mullo_basecase optimised for Intel Broadwell.

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
define(`jmpreg',`%rbx')
define(`nn',    `%rbp')

C TODO
C  * Suppress more rp[] rewrites in corner.
C  * Rearrange feed-in jumps for short branch forms.
C  * Perhaps roll out the heavy artillery and 8-way unroll outer loop.  Since
C    feed-in code implodes, the blow-up will not be more than perhaps 4x.
C  * Micro-optimise critical lead-in code block around L(ent).
C  * Write n < 4 code specifically for Broadwell (current code is for Haswell).

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
L(big):	push	%r14
	push	%r12
	push	%rbx
	push	%rbp
	mov	-8(vp_param,n,8), %r14	C FIXME Put at absolute end
	imul	(up), %r14		C FIXME Put at absolute end
	lea	-3(n), R32(nn)
	lea	8(vp_param), vp
	mov	(vp_param), %rdx

	mov	R32(n), R32(%rax)
	shr	$3, R32(n)
	and	$7, R32(%rax)		C clear OF, CF as side-effect
	lea	L(mtab)(%rip), %r10
ifdef(`PIC',
`	movslq	(%r10,%rax,4), %rax
	lea	(%rax, %r10), %r10
	jmp	*%r10
',`
	jmp	*(%r10,%rax,8)
')

L(mf0):	mulx(	(up), %r10, %r8)
	lea	56(up), up
	lea	-8(rp), rp
	lea	L(f7)(%rip), jmpreg
	jmp	L(mb0)

L(mf3):	mulx(	(up), %r9, %rax)
	lea	16(up), up
	lea	16(rp), rp
	jrcxz	L(mc)
	inc	R32(n)
	lea	L(f2)(%rip), jmpreg
	jmp	L(mb3)

L(mc):	mulx(	-8,(up), %r10, %r8)
	add	%rax, %r10
	mov	%r9, -16(rp)
	mulx(	(up), %r9, %rax)
	mov	%r10, -8(rp)
	adc	%r8, %r9
	mov	%r9, (rp)
	jmp	L(c2)

L(mf4):	mulx(	(up), %r10, %r8)
	lea	24(up), up
	lea	24(rp), rp
	inc	R32(n)
	lea	L(f3)(%rip), jmpreg
	jmp	L(mb4)

L(mf5):	mulx(	(up), %r9, %rax)
	lea	32(up), up
	lea	32(rp), rp
	inc	R32(n)
	lea	L(f4)(%rip), jmpreg
	jmp	L(mb5)

L(mf6):	mulx(	(up), %r10, %r8)
	lea	40(up), up
	lea	40(rp), rp
	inc	R32(n)
	lea	L(f5)(%rip), jmpreg
	jmp	L(mb6)

L(mf7):	mulx(	(up), %r9, %rax)
	lea	48(up), up
	lea	48(rp), rp
	lea	L(f6)(%rip), jmpreg
	jmp	L(mb7)

L(mf1):	mulx(	(up), %r9, %rax)
	lea	L(f0)(%rip), jmpreg
	jmp	L(mb1)

L(mf2):	mulx(	(up), %r10, %r8)
	lea	8(up), up
	lea	8(rp), rp
	lea	L(f1)(%rip), jmpreg
	mulx(	(up), %r9, %rax)

C FIXME ugly fallthrough FIXME
	ALIGN(32)
L(mtop):mov	%r10, -8(rp)
	adc	%r8, %r9
L(mb1):	mulx(	8,(up), %r10, %r8)
	adc	%rax, %r10
	lea	64(up), up
	mov	%r9, (rp)
L(mb0):	mov	%r10, 8(rp)
	mulx(	-48,(up), %r9, %rax)
	lea	64(rp), rp
	adc	%r8, %r9
L(mb7):	mulx(	-40,(up), %r10, %r8)
	mov	%r9, -48(rp)
	adc	%rax, %r10
L(mb6):	mov	%r10, -40(rp)
	mulx(	-32,(up), %r9, %rax)
	adc	%r8, %r9
L(mb5):	mulx(	-24,(up), %r10, %r8)
	mov	%r9, -32(rp)
	adc	%rax, %r10
L(mb4):	mulx(	-16,(up), %r9, %rax)
	mov	%r10, -24(rp)
	adc	%r8, %r9
L(mb3):	mulx(	-8,(up), %r10, %r8)
	adc	%rax, %r10
	mov	%r9, -16(rp)
	dec	R32(n)
	mulx(	(up), %r9, %rax)
	jnz	L(mtop)

L(mend):mov	%r10, -8(rp)
	adc	%r8, %r9
	mov	%r9, (rp)
	adc	%rcx, %rax

	lea	8(,nn,8), %r12
	neg	%r12
	shr	$3, R32(nn)
	jmp	L(ent)

L(f0):	mulx(	(up), %r10, %r8)
	lea	-8(up), up
	lea	-8(rp), rp
	lea	L(f7)(%rip), jmpreg
	jmp	L(b0)

L(f1):	mulx(	(up), %r9, %rax)
	lea	-1(nn), R32(nn)
	lea	L(f0)(%rip), jmpreg
	jmp	L(b1)

L(end):	adox(	(rp), %r9)
	mov	%r9, (rp)
	adox(	%rcx, %rax)		C relies on rcx = 0
	adc	%rcx, %rax		C FIXME suppress, use adc below; reqs ent path edits
	lea	8(%r12), %r12
L(ent):	mulx(	8,(up), %r10, %r8)	C r8 unused (use imul?)
	add	%rax, %r14
	add	%r10, %r14		C h
	lea	(up,%r12), up		C reset up
	lea	8(rp,%r12), rp		C reset rp
	mov	(vp), %rdx
	lea	8(vp), vp
	or	R32(nn), R32(n)		C copy count, clear CF,OF (n = 0 prior)
	jmp	*jmpreg

L(f7):	mulx(	(up), %r9, %rax)
	lea	-16(up), up
	lea	-16(rp), rp
	lea	L(f6)(%rip), jmpreg
	jmp	L(b7)

L(f2):	mulx(	(up), %r10, %r8)
	lea	8(up), up
	lea	8(rp), rp
	mulx(	(up), %r9, %rax)
	lea	L(f1)(%rip), jmpreg

C FIXME ugly fallthrough FIXME
	ALIGN(32)
L(top):	adox(	-8,(rp), %r10)
	adcx(	%r8, %r9)
	mov	%r10, -8(rp)
	jrcxz	L(end)
L(b1):	mulx(	8,(up), %r10, %r8)
	adox(	(rp), %r9)
	lea	-1(n), R32(n)
	mov	%r9, (rp)
	adcx(	%rax, %r10)
L(b0):	mulx(	16,(up), %r9, %rax)
	adcx(	%r8, %r9)
	adox(	8,(rp), %r10)
	mov	%r10, 8(rp)
L(b7):	mulx(	24,(up), %r10, %r8)
	lea	64(up), up
	adcx(	%rax, %r10)
	adox(	16,(rp), %r9)
	mov	%r9, 16(rp)
L(b6):	mulx(	-32,(up), %r9, %rax)
	adox(	24,(rp), %r10)
	adcx(	%r8, %r9)
	mov	%r10, 24(rp)
L(b5):	mulx(	-24,(up), %r10, %r8)
	adcx(	%rax, %r10)
	adox(	32,(rp), %r9)
	mov	%r9, 32(rp)
L(b4):	mulx(	-16,(up), %r9, %rax)
	adox(	40,(rp), %r10)
	adcx(	%r8, %r9)
	mov	%r10, 40(rp)
L(b3):	adox(	48,(rp), %r9)
	mulx(	-8,(up), %r10, %r8)
	mov	%r9, 48(rp)
	lea	64(rp), rp
	adcx(	%rax, %r10)
	mulx(	(up), %r9, %rax)
	jmp	L(top)

L(f6):	mulx(	(up), %r10, %r8)
	lea	40(up), up
	lea	-24(rp), rp
	lea	L(f5)(%rip), jmpreg
	jmp	L(b6)

L(f5):	mulx(	(up), %r9, %rax)
	lea	32(up), up
	lea	-32(rp), rp
	lea	L(f4)(%rip), jmpreg
	jmp	L(b5)

L(f4):	mulx(	(up), %r10, %r8)
	lea	24(up), up
	lea	-40(rp), rp
	lea	L(f3)(%rip), jmpreg
	jmp	L(b4)

L(f3):	mulx(	(up), %r9, %rax)
	lea	16(up), up
	lea	-48(rp), rp
	jrcxz	L(cor)
	lea	L(f2)(%rip), jmpreg
	jmp	L(b3)

L(cor):	adox(	48,(rp), %r9)
	mulx(	-8,(up), %r10, %r8)
	mov	%r9, 48(rp)
	lea	64(rp), rp
	adcx(	%rax, %r10)
	mulx(	(up), %r9, %rax)
	adox(	-8,(rp), %r10)
	adcx(	%r8, %r9)
	mov	%r10, -8(rp)		C FIXME suppress
	adox(	(rp), %r9)
	mov	%r9, (rp)		C FIXME suppress
	adox(	%rcx, %rax)
L(c2):
	mulx(	8,(up), %r10, %r8)
	adc	%rax, %r14
	add	%r10, %r14
	mov	(vp), %rdx
	test	R32(%rcx), R32(%rcx)
	mulx(	-16,(up), %r10, %r8)
	mulx(	-8,(up), %r9, %rax)
	adox(	-8,(rp), %r10)
	adcx(	%r8, %r9)
	mov	%r10, -8(rp)
	adox(	(rp), %r9)
	adox(	%rcx, %rax)
	adc	%rcx, %rax
	mulx(	(up), %r10, %r8)
	add	%rax, %r14
	add	%r10, %r14
	mov	8(vp), %rdx
	mulx(	-16,(up), %rcx, %rax)
	add	%r9, %rcx
	mov	%rcx, (rp)
	adc	$0, %rax
	mulx(	-8,(up), %r10, %r8)
	add	%rax, %r14
	add	%r10, %r14
	mov	%r14, 8(rp)
	pop	%rbp
	pop	%rbx
	pop	%r12
	pop	%r14
	FUNC_EXIT()
	ret
EPILOGUE()
	JUMPTABSECT
	ALIGN(8)
L(mtab):JMPENT(	L(mf7), L(mtab))
	JMPENT(	L(mf0), L(mtab))
	JMPENT(	L(mf1), L(mtab))
	JMPENT(	L(mf2), L(mtab))
	JMPENT(	L(mf3), L(mtab))
	JMPENT(	L(mf4), L(mtab))
	JMPENT(	L(mf5), L(mtab))
	JMPENT(	L(mf6), L(mtab))
