/*
Copyright 2018-2019 Free Software Foundation, Inc.

This file is part of the GNU MP Library test suite.

The GNU MP Library test suite is free software; you can redistribute it
and/or modify it under the terms of the GNU General Public License as
published by the Free Software Foundation; either version 3 of the License,
or (at your option) any later version.

The GNU MP Library test suite is distributed in the hope that it will be
useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General
Public License for more details.

You should have received a copy of the GNU General Public License along with
the GNU MP Library test suite.  If not, see https://www.gnu.org/licenses/.  */

/* Usage:

   ./primes [p|c] [n0] <nMax>

     Checks mpz_probab_prime_p(n, r) exhaustively, starting from n=n0
     up to nMax.
     If n0 * n0 > nMax, the intervall is sieved piecewise, else the
     full intervall [0..nMax] is sieved at once.
     With the parameter "p" (or nothing), tests all numbers. With "c"
     only composites are tested.

   ./primes n [n0] <nMax>

     Checks mpz_nextprime() exhaustively, starting from n=n0 up to
     nMax.

     WARNING: The full intervall [0..nMax] is sieved at once, even if
     only a piece is needed. This may require a lot of memory!

 */

#include <stdlib.h>
#include <stdio.h>
#include "gmp-impl.h"
#include "longlong.h"
#include "tests.h"
#define STOP(x) return (x)
/* #define STOP(x) x */
#define REPS 10
/* #define TRACE(x,n) if ((n)>1) {x;} */
#define TRACE(x,n)

/* The full primesieve.c is included, just for block_resieve, that
   is not exported ... */
#undef gmp_primesieve
#include "../../primesieve.c"

#ifndef BLOCK_SIZE
#define BLOCK_SIZE 2048
#endif

/*********************************************************/
/* Section sieve: sieving functions and tools for primes */
/*********************************************************/

static mp_size_t
primesieve_size (mp_limb_t n) { return n_to_bit(n) / GMP_LIMB_BITS + 1; }

/*************************************************************/
/* Section macros: common macros, for swing/fac/bin (&sieve) */
/*************************************************************/

#define LOOP_ON_SIEVE_CONTINUE(prime,end,sieve)			\
    __max_i = (end);						\
								\
    do {							\
      ++__i;							\
      if (((sieve)[__index] & __mask) == 0)			\
	{							\
          mp_limb_t prime;					\
	  prime = id_to_n(__i)

#define LOOP_ON_SIEVE_BEGIN(prime,start,end,off,sieve)		\
  do {								\
    mp_limb_t __mask, __index, __max_i, __i;			\
								\
    __i = (start)-(off);					\
    __index = __i / GMP_LIMB_BITS;				\
    __mask = CNST_LIMB(1) << (__i % GMP_LIMB_BITS);		\
    __i += (off);						\
								\
    LOOP_ON_SIEVE_CONTINUE(prime,end,sieve)

#define LOOP_ON_SIEVE_STOP					\
	}							\
      __mask = __mask << 1 | __mask >> (GMP_LIMB_BITS-1);	\
      __index += __mask & 1;					\
    }  while (__i <= __max_i)

#define LOOP_ON_SIEVE_END					\
    LOOP_ON_SIEVE_STOP;						\
  } while (0)

mpz_t g;

int something_wrong (mpz_t er, int exp)
{
  fprintf (stderr, "value = %lu , expected = %i\n", mpz_get_ui (er), exp);
  return -1;
}

int
check_pprime (unsigned long begin, unsigned long end, int composites)
{
  begin = (begin / 6U) * 6U;
  for (;(begin < 2) & (begin <= end); ++begin)
    {
      *(g->_mp_d) = begin;
      TRACE(printf ("-%li ", begin),1);
      if (mpz_probab_prime_p (g, REPS))
	STOP (something_wrong (g, 0));
    }
  for (;(begin < 4) & (begin <= end); ++begin)
    {
      *(g->_mp_d) = begin;
      TRACE(printf ("+%li ", begin),2);
      if (!composites && !mpz_probab_prime_p (g, REPS))
	STOP (something_wrong (g, 1));
    }
  if (end > 4) {
    if ((end > 10000) && (begin > end / begin))
      {
	mp_limb_t *sieve, *primes;
	mp_size_t size_s, size_p, off;
	unsigned long start;

	mpz_set_ui (g, end);
	mpz_sqrt (g, g);
	start = mpz_get_ui (g) + GMP_LIMB_BITS;
	size_p = primesieve_size (start);

	primes = __GMP_ALLOCATE_FUNC_LIMBS (size_p);
	gmp_primesieve (primes, start);

	size_s = BLOCK_SIZE * 2;
	sieve = __GMP_ALLOCATE_FUNC_LIMBS (size_s);
	off = n_to_bit(begin) + (begin % 3 == 0);

	do {
	  TRACE (printf ("off =%li\n", off),3);
	  block_resieve (sieve, BLOCK_SIZE, off, primes);
	  TRACE (printf ("LOOP =%li - %li\n", id_to_n (off+1), id_to_n (off + BLOCK_SIZE * GMP_LIMB_BITS)),3);
	  LOOP_ON_SIEVE_BEGIN (prime, off, off + BLOCK_SIZE * GMP_LIMB_BITS - 1,
			       off, sieve);

	  do {
	    *(g->_mp_d) = begin;
	    TRACE(printf ("-%li ", begin),1);
	    if (mpz_probab_prime_p (g, REPS))
	      STOP (something_wrong (g, 0));
	    if ((begin & 0xff) == 0)
	      {
		spinner();
		if ((begin & 0xfffffff) == 0)
		  printf ("%li (0x%lx)\n", begin, begin);
	      }
	  } while (++begin < prime);

	  *(g->_mp_d) = begin;
	  TRACE(printf ("+%li ", begin),2);
	  if (!composites && ! mpz_probab_prime_p (g, REPS))
	    STOP (something_wrong (g, 1));
	  ++begin;

	  LOOP_ON_SIEVE_END;
	  off += BLOCK_SIZE * GMP_LIMB_BITS;
	} while (begin < end);

	__GMP_FREE_FUNC_LIMBS (sieve, size_s);
	__GMP_FREE_FUNC_LIMBS (primes, size_p);
      }
    else
      {
	mp_limb_t *sieve;
	mp_size_t size;
	unsigned long start;

	size = primesieve_size (end);

	sieve = __GMP_ALLOCATE_FUNC_LIMBS (size);
	gmp_primesieve (sieve, end);
	start = MAX (begin, 5) | 1;
	LOOP_ON_SIEVE_BEGIN (prime, n_to_bit(start) + (start % 3 == 0),
			     n_to_bit (end), 0, sieve);

	do {
	  *(g->_mp_d) = begin;
	  TRACE(printf ("-%li ", begin),1);
	  if (mpz_probab_prime_p (g, REPS))
	    STOP (something_wrong (g, 0));
	  if ((begin & 0xff) == 0)
	    {
	      spinner();
	      if ((begin & 0xfffffff) == 0)
		printf ("%li (0x%lx)\n", begin, begin);
	    }
	} while (++begin < prime);

	*(g->_mp_d) = begin;
	TRACE(printf ("+%li ", begin),2);
	if (!composites && ! mpz_probab_prime_p (g, REPS))
	  STOP (something_wrong (g, 1));
	++begin;

	LOOP_ON_SIEVE_END;

	__GMP_FREE_FUNC_LIMBS (sieve, size);
      }
  }

  for (;begin < end; ++begin)
    {
      *(g->_mp_d) = begin;
      TRACE(printf ("-%li ", begin),1);
      if (mpz_probab_prime_p (g, REPS))
	STOP (something_wrong (g, 0));
    }

  gmp_printf ("%Zd\n", g);
  return 0;
}

int
check_nprime (unsigned long begin, unsigned long end)
{
  if (begin < 2)
    {
      *(g->_mp_d) = begin;
      TRACE(printf ("%li ", begin),1);
      mpz_nextprime (g, g);
      if (mpz_cmp_ui (g, 2) != 0)
	STOP (something_wrong (g, -1));
      begin = mpz_get_ui (g);
    }
  if (begin < 3)
    {
      *(g->_mp_d) = begin;
      TRACE(printf ("%li ", begin),1);
      mpz_nextprime (g, g);
      if (mpz_cmp_ui (g, 3) != 0)
	STOP (something_wrong (g, -1));
      begin = mpz_get_ui (g);
    }
  if (end > 4)
      {
	mp_limb_t *sieve;
	mp_size_t size;
	unsigned long start;

	size = primesieve_size (end);

	sieve = __GMP_ALLOCATE_FUNC_LIMBS (size);
	gmp_primesieve (sieve, end);
	start = MAX (begin, 5) | 1;
	*(g->_mp_d) = begin;
	LOOP_ON_SIEVE_BEGIN (prime, n_to_bit(start) + (start % 3 == 0),
			     n_to_bit (end), 0, sieve);

	mpz_nextprime (g, g);
	if (mpz_cmp_ui (g, prime) != 0)
	  STOP (something_wrong (g, -1));

	if (prime - start > 200)
	  {
	    start = prime;
	    spinner();
	    if (prime - begin > 0xfffffff)
	      {
		begin = prime;
		printf ("%li (0x%lx)\n", begin, begin);
	      }
	  }

	LOOP_ON_SIEVE_END;

	__GMP_FREE_FUNC_LIMBS (sieve, size);
      }

  if (mpz_cmp_ui (g, end) < 0)
    {
      mpz_nextprime (g, g);
      if (mpz_cmp_ui (g, end) <= 0)
	STOP (something_wrong (g, -1));
    }

  gmp_printf ("%Zd\n", g);
  return 0;
}

int
main (int argc, char **argv)
{
  int ret, mode = 0;
  unsigned long begin = 0, end = 0;

  for (;argc > 1;--argc,++argv)
    switch (*argv[1]) {
    case 'p':
      mode = 0;
      break;
    case 'c':
      mode = 2;
      break;
    case 'n':
      mode = 1;
      break;
    default:
      begin = end;
      end = atol (argv[1]);
    }

  if (begin >= end)
    {
      fprintf (stderr, "usage: primes [n|p|c] [n0] <nMax>\n");
      exit (1);
    }

  mpz_init_set_ui (g, ULONG_MAX);

  switch (mode) {
  case 1:
    ret = check_nprime (begin, end);
    break;
  default:
    ret = check_pprime (begin, end, mode);
  }

  mpz_clear (g);

  if (ret == 0)
    printf ("Prime tests checked in [%lu - %lu] [0x%lx - 0x%lx].\n", begin, end, begin, end);
  return ret;
}
