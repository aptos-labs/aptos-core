dnl  AMD64 mpn_sqr_basecase optimised for Intel Broadwell.

dnl  Copyright 2015, 2017 Free Software Foundation, Inc.

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

C cycles/limb	mul_1		addmul_1
C AMD K8,K9	n/a		n/a
C AMD K10	n/a		n/a
C AMD bd1	n/a		n/a
C AMD bd2	n/a		n/a
C AMD bd3	n/a		n/a
C AMD bd4	 ?		 ?
C AMD zen	 ?		 ?
C AMD bt1	n/a		n/a
C AMD bt2	n/a		n/a
C Intel P4	n/a		n/a
C Intel PNR	n/a		n/a
C Intel NHM	n/a		n/a
C Intel SBR	n/a		n/a
C Intel IBR	n/a		n/a
C Intel HWL	 1.68		n/a
C Intel BWL	 1.51	      1.67-1.74
C Intel SKL	 1.52	      1.63-1.71
C Intel atom	n/a		n/a
C Intel SLM	n/a		n/a
C VIA nano	n/a		n/a

C The inner loops of this code are the result of running a code generation and
C optimisation tool suite written by David Harvey and Torbjorn Granlund.

C TODO
C  * We have 8 addmul_1 loops which fall into each other.  The idea is to save
C    on switching code, since a circularly updated computed goto target will
C    hardly allow correct branch prediction.  On 2nd thought, we now might make
C    each of the 8 loop branches be poorly predicted since they will be
C    executed fewer times for each time.  With just one addmul_1 loop, the loop
C    count will change only once each 8th time.
C  * Do overlapped software pipelining.
C  * Perhaps load in shrx/sarx, eliminating separate load insn.
C  * Schedule add+stored in small n code.
C  * Try swapping adox and adcx insn, making mulx have more time to run.

define(`rp',      `%rdi')
define(`up',      `%rsi')
define(`un_param',`%rdx')

define(`n',       `%rcx')
define(`un_save', `%rbx')
define(`u0',      `%rdx')

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

	cmp	$2, un_param
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

L(gt2):	cmp	$4, un_param
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

L(gt3):	push	%rbx

	lea	-3(un_param), R32(un_save)
	lea	5(un_param), R32(n)
	mov	R32(un_param), R32(%rax)
	and	$-8, R32(un_save)
	shr	$3, R32(n)		C count for mul_1 loop
	neg	un_save			C 8*count and offert for addmul_1 loops
	and	$7, R32(%rax)		C clear CF for adc as side-effect

	mov	(up), u0

	lea	L(mtab)(%rip), %r10
ifdef(`PIC',
`	movslq	(%r10,%rax,4), %r8
	lea	(%r8, %r10), %r10
	jmp	*%r10
',`
	jmp	*(%r10,%rax,8)
')

L(mf0):	mulx(	u0, w0, w1)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w2, w3)
	lea	64(up), up
	add	w1, w2
	jmp	L(mb0)

L(mf3):	mulx(	u0, w2, w3)		C up[0]^2
	add	u0, u0
	mov	w2, (rp)
	mulx(	8,(up), w0, w1)
	lea	24(up), up
	lea	24(rp), rp
	add	w3, w0
	jmp	L(mb3)

L(mf4):	mulx(	u0, w0, w1)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w2, w3)
	mov	w0, (rp)
	lea	32(up), up
	lea	32(rp), rp
	add	w1, w2
	jmp	L(mb4)

L(mf5):	mulx(	u0, w2, w3)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w0, w1)
	mov	w2, (rp)
	lea	40(up), up
	lea	40(rp), rp
	add	w3, w0
	jmp	L(mb5)

L(mf6):	mulx(	u0, w0, w1)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w2, w3)
	mov	w0, (rp)
	lea	48(up), up
	lea	48(rp), rp
	add	w1, w2
	jmp	L(mb6)

L(mf7):	mulx(	u0, w2, w3)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w0, w1)
	mov	w2, (rp)
	lea	56(up), up
	lea	56(rp), rp
	add	w3, w0
	jmp	L(mb7)

L(mf1):	mulx(	u0, w2, w3)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w0, w1)
	mov	w2, (rp)
	lea	8(up), up
	lea	8(rp), rp
	add	w3, w0
	jmp	L(mb1)

L(mf2):	mulx(	u0, w0, w1)		C up[0]^2
	add	u0, u0
	mulx(	8,(up), w2, w3)
	mov	w0, (rp)
	lea	16(up), up
	lea	16(rp), rp
	dec	R32(n)
	add	w1, w2
	mulx(	(up), w0, w1)

	ALIGN(16)
L(top):	mov	w2, -8(rp)
	adc	w3, w0
L(mb1):	mulx(	8,(up), w2, w3)
	adc	w1, w2
	lea	64(up), up
L(mb0):	mov	w0, (rp)
	mov	w2, 8(rp)
	mulx(	-48,(up), w0, w1)
	lea	64(rp), rp
	adc	w3, w0
L(mb7):	mulx(	-40,(up), w2, w3)
	mov	w0, -48(rp)
	adc	w1, w2
L(mb6):	mov	w2, -40(rp)
	mulx(	-32,(up), w0, w1)
	adc	w3, w0
L(mb5):	mulx(	-24,(up), w2, w3)
	mov	w0, -32(rp)
	adc	w1, w2
L(mb4):	mulx(	-16,(up), w0, w1)
	mov	w2, -24(rp)
	adc	w3, w0
L(mb3):	mulx(	-8,(up), w2, w3)
	adc	w1, w2
	mov	w0, -16(rp)
	dec	R32(n)
	mulx(	(up), w0, w1)
	jnz	L(top)

L(end):	mov	w2, -8(rp)
	adc	w3, w0
C	mov	w0, (rp)
C	adc	%rcx, w1
C	mov	w1, 8(rp)

	lea	L(atab)(%rip), %r10
ifdef(`PIC',
`	movslq	(%r10,%rax,4), %r11
	lea	(%r11, %r10), %r11
',`
	mov	(%r10,%rax,8), %r11
')
	mov	$63, R32(%rax)
	jmp	*%r11

L(ed0):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f7):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	-64(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	(up), w1		C up[-1]
	mov	8(up), u0		C up[0]
	shrx(	%rax, w1, w0)
	sarx(	%rax, w1, w1)
	and	u0, w1			C "ci" in C code
	mulx(	u0, w2, w3)		C up[0]^2
	lea	(w0,u0,2), u0		C "u0" arg in C code
	jmp	L(b7)

	ALIGN(16)
L(tp0):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed0)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
L(b0):	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp0)

L(ed1):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f0):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	-64(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	-8(up), w3		C up[-1]
	mov	(up), u0		C up[0]
	shrx(	%rax, w3, w2)
	sarx(	%rax, w3, w3)
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	lea	(w2,u0,2), u0		C "u0" arg in C code
	adcx(	w3, w0)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	jmp	L(b0)

	ALIGN(16)
L(tp1):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed1)
L(b1):	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp1)

L(ed2):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f1):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	8(un_save), un_save
	lea	-56(rp,un_save,8), rp
	mov	-16(up), w1		C up[-1]
	mov	-8(up), u0		C up[0]
	shrx(	%rax, w1, w0)
	sarx(	%rax, w1, w1)
	and	u0, w1			C "ci" in C code
	mulx(	u0, w2, w3)		C up[0]^2
	lea	(w0,u0,2), u0		C "u0" arg in C code
	adcx(	w1, w2)			C FIXME: crossjump?
	mulx(	(up), w0, w1)
	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jmp	L(b1)

	ALIGN(16)
L(tp2):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed2)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
L(b2):	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp2)

L(ed3):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f2):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	or	R32(un_save), R32(n)
	jz	L(cor3)
	lea	-56(rp,un_save,8), rp
	mov	-24(up), w3		C up[-1]
	mov	-16(up), u0		C up[0]
	shrx(	%rax, w3, w2)
	sarx(	%rax, w3, w3)
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	lea	(w2,u0,2), u0		C "u0" arg in C code
	adcx(	w3, w0)
	jmp	L(b2)

	ALIGN(16)
L(tp3):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed3)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
L(b3):	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp3)

L(ed4):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f3):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	-32(up), w1		C up[-1]
	mov	-24(up), u0		C up[0]
	shrx(	%rax, w1, w0)
	sarx(	%rax, w1, w1)
	and	u0, w1			C "ci" in C code
	mulx(	u0, w2, w3)		C up[0]^2
	lea	(w0,u0,2), u0		C "u0" arg in C code
	adcx(	w1, w2)
	jmp	L(b3)

	ALIGN(16)
L(tp4):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed4)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
L(b4):	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp4)

L(ed5):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f4):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	-40(up), w3		C up[-1]
	mov	-32(up), u0		C up[0]
	shrx(	%rax, w3, w2)
	sarx(	%rax, w3, w3)
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	lea	(w2,u0,2), u0		C "u0" arg in C code
	adcx(	w3, w0)
	jmp	L(b4)

	ALIGN(16)
L(tp5):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed5)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
L(b5):	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp5)

L(ed6):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f5):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	-48(up), w1		C up[-1]
	mov	-40(up), u0		C up[0]
	shrx(	%rax, w1, w0)
	sarx(	%rax, w1, w1)
	and	u0, w1			C "ci" in C code
	mulx(	u0, w2, w3)		C up[0]^2
	lea	(w0,u0,2), u0		C "u0" arg in C code
	adcx(	w1, w2)
	jmp	L(b5)

	ALIGN(16)
L(tp6):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed6)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
L(b6):	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp6)

L(ed7):	adox(	(rp), w0)
	adox(	%rcx, w1)		C relies on rcx = 0
L(f6):	mov	w0, (rp)
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 8(rp)
	lea	(up,un_save,8), up
	mov	R32(un_save), R32(n)
	lea	-56(rp,un_save,8), rp
	mov	-56(up), w3		C up[-1]
	mov	-48(up), u0		C up[0]
	shrx(	%rax, w3, w2)
	sarx(	%rax, w3, w3)
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	lea	(w2,u0,2), u0		C "u0" arg in C code
	adcx(	w3, w0)
	mulx(	-40,(up), w2, w3)
	jmp	L(b6)

	ALIGN(16)
L(tp7):	adox(	-8,(rp), w2)
	adcx(	w3, w0)
	mov	w2, -8(rp)
	jrcxz	L(ed7)
	mulx(	8,(up), w2, w3)
	adox(	(rp), w0)
	lea	8(n), R32(n)
	mov	w0, (rp)
L(b7):	adcx(	w1, w2)
	mulx(	16,(up), w0, w1)
	adcx(	w3, w0)
	adox(	8,(rp), w2)
	mov	w2, 8(rp)
	mulx(	24,(up), w2, w3)
	lea	64(up), up
	adcx(	w1, w2)
	adox(	16,(rp), w0)
	mov	w0, 16(rp)
	mulx(	-32,(up), w0, w1)
	adox(	24,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 24(rp)
	mulx(	-24,(up), w2, w3)
	adcx(	w1, w2)
	adox(	32,(rp), w0)
	mov	w0, 32(rp)
	mulx(	-16,(up), w0, w1)
	adox(	40,(rp), w2)
	adcx(	w3, w0)
	mov	w2, 40(rp)
	adox(	48,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 48(rp)
	lea	64(rp), rp
	adcx(	w1, w2)
	mulx(	(up), w0, w1)
	jmp	L(tp7)

L(cor3):lea	-64(rp), rp
	mov	-24(up), w3		C up[-1]
	mov	-16(up), u0		C up[0]
	shrx(	%rax, w3, w2)
	sarx(	%rax, w3, w3)
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	lea	(w2,u0,2), u0		C "u0" arg in C code
	adcx(	w3, w0)
	adox(	56,(rp), w0)
	mulx(	-8,(up), w2, w3)
	mov	w0, 56(rp)
	adcx(	w1, w2)
	mulx(	(up), %rbx, w1)
	adox(	64,(rp), w2)
	adcx(	w3, %rbx)
	mov	w2, 64(rp)
	adox(	72,(rp), %rbx)
	adox(	%rcx, w1)		C relies on rcx = 0
	adc	%rcx, w1		C relies on rcx = 0
	mov	w1, 80(rp)	C FIXME
C wd2
	mov	-16(up), w1		C up[-1]
	mov	-8(up), u0		C up[0]
	shrx(	%rax, w1, w0)
	sarx(	%rax, w1, w1)
	and	u0, w1			C "ci" in C code
	mulx(	u0, w2, w3)		C up[0]^2
	lea	(w0,u0,2), u0		C "u0" arg in C code
	adcx(	w1, w2)
	mulx(	(up), w0, %rax)
	adox(	%rbx, w2)
	adcx(	w3, w0)
	mov	w2, 72(rp)
	adox(	80,(rp), w0)
	adox(	%rcx, %rax)		C relies on rcx = 0
	mov	w0, 80(rp)
	adc	%rcx, %rax		C relies on rcx = 0
C wd1
	mov	-8(up), w3		C up[-1]
	mov	(up), u0		C up[0]
	sar	$63, w3
	and	u0, w3			C "ci" in C code
	mulx(	u0, w0, w1)		C up[0]^2
	adcx(	w3, w0)
	adox(	%rax, w0)
	mov	w0, 88(rp)
	adcx(	%rcx, w1)
	adox(	%rcx, w1)
	mov	w1, 96(rp)

	pop	%rbx
	FUNC_EXIT()
	ret

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
L(atab):JMPENT(	L(f6), L(atab))
	JMPENT(	L(f7), L(atab))
	JMPENT(	L(f0), L(atab))
	JMPENT(	L(f1), L(atab))
	JMPENT(	L(f2), L(atab))
	JMPENT(	L(f3), L(atab))
	JMPENT(	L(f4), L(atab))
	JMPENT(	L(f5), L(atab))
	TEXT
EPILOGUE()
