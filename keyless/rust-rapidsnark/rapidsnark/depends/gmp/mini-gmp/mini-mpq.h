/* mini-mpq, a minimalistic implementation of a GNU GMP subset.

Copyright 2018, 2019 Free Software Foundation, Inc.

This file is part of the GNU MP Library.

The GNU MP Library is free software; you can redistribute it and/or modify
it under the terms of either:

  * the GNU Lesser General Public License as published by the Free
    Software Foundation; either version 3 of the License, or (at your
    option) any later version.

or

  * the GNU General Public License as published by the Free Software
    Foundation; either version 2 of the License, or (at your option) any
    later version.

or both in parallel, as here.

The GNU MP Library is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

You should have received copies of the GNU General Public License and the
GNU Lesser General Public License along with the GNU MP Library.  If not,
see https://www.gnu.org/licenses/.  */

/* Header */

#ifndef __MINI_MPQ_H__
#define __MINI_MPQ_H__

#include "mini-gmp.h"

#if defined (__cplusplus)
extern "C" {
#endif

typedef struct
{
  __mpz_struct _mp_num;
  __mpz_struct _mp_den;
} __mpq_struct;

typedef __mpq_struct mpq_t[1];

typedef const __mpq_struct *mpq_srcptr;
typedef __mpq_struct *mpq_ptr;

#define mpq_numref(Q) (&((Q)->_mp_num))
#define mpq_denref(Q) (&((Q)->_mp_den))

void mpq_abs (mpq_t, const mpq_t);
void mpq_add (mpq_t, const mpq_t, const mpq_t);
void mpq_canonicalize (mpq_t);
void mpq_clear (mpq_t);
int mpq_cmp (const mpq_t, const mpq_t);
int mpq_cmp_si (const mpq_t, signed long, unsigned long);
int mpq_cmp_ui (const mpq_t, unsigned long, unsigned long);
int mpq_cmp_z (const mpq_t, const mpz_t);
void mpq_div (mpq_t, const mpq_t, const mpq_t);
void mpq_div_2exp (mpq_t, const mpq_t, mp_bitcnt_t);
int mpq_equal (const mpq_t, const mpq_t);
double mpq_get_d (const mpq_t);
void mpq_get_den (mpz_t, const mpq_t);
void mpq_get_num (mpz_t, const mpq_t);
char * mpq_get_str (char *, int, const mpq_t q);
void mpq_init (mpq_t);
void mpq_inv (mpq_t, const mpq_t);
void mpq_mul (mpq_t, const mpq_t, const mpq_t);
void mpq_mul_2exp (mpq_t, const mpq_t, mp_bitcnt_t);
void mpq_neg (mpq_t, const mpq_t);
void mpq_set (mpq_t, const mpq_t);
void mpq_set_d (mpq_t, double);
void mpq_set_den (mpq_t, const mpz_t);
void mpq_set_num (mpq_t, const mpz_t);
void mpq_set_si (mpq_t, signed long, unsigned long);
int mpq_set_str (mpq_t, const char *, int);
void mpq_set_ui (mpq_t, unsigned long, unsigned long);
void mpq_set_z (mpq_t, const mpz_t);
int mpq_sgn (const mpq_t);
void mpq_sub (mpq_t, const mpq_t, const mpq_t);
void mpq_swap (mpq_t, mpq_t);

/* This long list taken from gmp.h. */
/* For reference, "defined(EOF)" cannot be used here.  In g++ 2.95.4,
   <iostream> defines EOF but not FILE.  */
#if defined (FILE)                                              \
  || defined (H_STDIO)                                          \
  || defined (_H_STDIO)               /* AIX */                 \
  || defined (_STDIO_H)               /* glibc, Sun, SCO */     \
  || defined (_STDIO_H_)              /* BSD, OSF */            \
  || defined (__STDIO_H)              /* Borland */             \
  || defined (__STDIO_H__)            /* IRIX */                \
  || defined (_STDIO_INCLUDED)        /* HPUX */                \
  || defined (__dj_include_stdio_h_)  /* DJGPP */               \
  || defined (_FILE_DEFINED)          /* Microsoft */           \
  || defined (__STDIO__)              /* Apple MPW MrC */       \
  || defined (_MSL_STDIO_H)           /* Metrowerks */          \
  || defined (_STDIO_H_INCLUDED)      /* QNX4 */                \
  || defined (_ISO_STDIO_ISO_H)       /* Sun C++ */             \
  || defined (__STDIO_LOADED)         /* VMS */
size_t mpq_out_str (FILE *, int, const mpq_t);
#endif

void mpz_set_q (mpz_t, const mpq_t);

#if defined (__cplusplus)
}
#endif
#endif /* __MINI_MPQ_H__ */
