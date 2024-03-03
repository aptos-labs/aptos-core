/*

Copyright 2012-2014, 2016, 2018, 2020 Free Software Foundation, Inc.

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

#include <assert.h>
#include <limits.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "testutils.h"
#include "../mini-mpq.h"

#define MAXBITS 400
#define COUNT 2000

#define GMP_LIMB_BITS (sizeof(mp_limb_t) * CHAR_BIT)
#define MAXLIMBS ((MAXBITS + GMP_LIMB_BITS - 1) / GMP_LIMB_BITS)

static void
test_small (void)
{
  struct {
    const char *input;
    const char *decimal;
  } data[] = {
    { "1832407/3", "1832407/3" },
    { " 2763959/6", "2763959/6 " },
    { "4 981 999 / 1 8", "4981999/18" },
    { "10\t73981/30 ", "1073981/30" },
    { "958 544 /1", "00958544/01" },
    { "-0", "0000" },
    { " -000  ", "0/ 1" },
    { "0704436/011", "231710/9" },
    /* Check the case of large number of leading zeros. */
    { "0000000000000000000000000/1", "0/0000000000000000000000001" },
    { "000000000000000704436/000011", "0000000000000000231710/00009" },
    { " 012/ 02503517", "10/689999" },
    { "0b 10/0 1312143", "2/365667" },
    { "-03 274062/0x1", "-882738/1" },
    { "012\t242", "005282" },
    { "9/0b11010111110010001111", "9/883855" },
    { "022/ 0b11001010010100001", "18/103585" },
    { "-0b101010110011101111/0x12", "-175343/18" },
    { "-05/0b 111 1111 0110 1110 0110", "-5/521958" },
    { "0b 011 111 110 111 001 000 011/0b00110", "1044035/6" },
    { " 0x53dfc", "343548" },
    { "-0x00012/0x000fA019", "-18/1024025" },
    { "0x 642d1", "410321" },
    { "0x5 8067/0Xa", "360551/10" },
    { "-0xd6Be6/3", "-879590/3" },
    { "\t0B1110000100000000011", "460803" },
    { "0B\t1111110010010100101", "517285" },
    { "-0x 00 2d/0B1\t010111101101110100", "-45/359284" },
    { "-0B101\t1001101111111001", "-367609" },
    { "0B10001001010111110000/0xf", "562672/15" },
    { "0Xe4B7e/1", "936830" },
    { "0X1E4bf/0X1", "124095" },
    { "-0Xfdb90/05", "-1039248/5" },
    { "0b010/0X7fc47", "2/523335" },
    { "15/0X8167c", "15/530044" },
    /* Some invalid inputs */
    { "", NULL },
    { "0x", NULL },
    { "0b", NULL },
    { "0z", NULL },
    { "-", NULL },
    { "/0x ", NULL },
    { "0|1", NULL },
    { "/", NULL },
    { "0ab", NULL },
    { "10x0", NULL },
    { "1/0xxab", NULL },
    { "0/ab", NULL },
    { "0/#", NULL },
    { "$foo/1", NULL },
    { NULL, NULL }
  };
  unsigned i;
  mpq_t a, b;
  mpq_init (a);
  mpq_init (b);

  for (i = 0; data[i].input; i++)
    {
      int res = mpq_set_str (a, data[i].input, 0);
      if (data[i].decimal)
	{
	  if (res != 0)
	    {
	      fprintf (stderr, "mpq_set_str returned -1, input: %s\n",
		       data[i].input);
	      abort ();
	    }
	  if (mpq_set_str (b, data[i].decimal, 10) != 0)
	    {
	      fprintf (stderr, "mpq_set_str returned -1, decimal input: %s\n",
		       data[i].input);
	      abort ();
	    }
	  if (!mpq_equal (a, b))
	    {
	      fprintf (stderr, "mpq_set_str failed for input: %s\n",
		       data[i].input);

	      dump ("got_num", mpq_numref (a));
	      dump ("got_den", mpq_denref (a));
	      dump ("ref_num", mpq_numref (b));
	      dump ("ref_den", mpq_denref (b));
	      abort ();
	    }
	}
      else if (res != -1)
	{
	  fprintf (stderr, "mpq_set_str returned %d, invalid input: %s\n",
		   res, data[i].input);
	  abort ();
	}
    }

  mpq_clear (a);
  mpq_clear (b);
}

void
testmain (int argc, char **argv)
{
  unsigned i;
  char *ap;
  char *bp;
  char *rp;
  size_t rn, arn;

  mpq_t a, b;

  FILE *tmp;

  test_small ();

  mpq_init (a);
  mpq_init (b);

  tmp = tmpfile ();
  if (!tmp)
    fprintf (stderr,
	     "Failed to create temporary file. Skipping mpq_out_str tests.\n");

  if (mpq_out_str (tmp, 63, a) != 0)
    {
      printf ("mpq_out_str did not return 0 (error) with base > 62\n");
      abort ();
    }

  if (mpq_out_str (tmp, -37, a) != 0)
    {
      printf ("mpq_out_str did not return 0 (error) with base < -37\n");
      abort ();
    }

  for (i = 0; i < COUNT/60; i++)
    {
      int base;
      for (base = 2; base <= 62; ++base)
	{
	  hex_mpq_random_str_op (MAXBITS, (i&1 || base > 36) ? base: -base, &ap, &rp);
	  if (mpq_set_str (a, ap, 16) != 0)
	    {
	      fprintf (stderr, "mpq_set_str failed on input %s\n", ap);
	      abort ();
	    }

	  rn = strlen (rp);
	  arn = rn - (rp[0] == '-');

	  bp = mpq_get_str (NULL, (i&1 || base > 36) ? base: -base, a);
	  if (strcmp (bp, rp))
	    {
	      fprintf (stderr, "mpz_get_str failed:\n");
	      dump ("a_num", mpq_numref (a));
	      dump ("a_den", mpq_denref (a));
	      fprintf (stderr, "b = %s\n", bp);
	      fprintf (stderr, "  base = %d\n", base);
	      fprintf (stderr, "r = %s\n", rp);
	      abort ();
	    }

	  /* Just a few tests with file i/o. */
	  if (tmp && i < 20)
	    {
	      size_t tn;
	      rewind (tmp);
	      tn = mpq_out_str (tmp, (i&1 || base > 36) ? base: -base, a);
	      if (tn != rn)
		{
		  fprintf (stderr, "mpq_out_str, bad return value:\n");
		  dump ("a_num", mpq_numref (a));
		  dump ("a_den", mpq_denref (a));
		  fprintf (stderr, "r = %s\n", rp);
		  fprintf (stderr, "  base %d, correct size %u, got %u\n",
			   base, (unsigned) rn, (unsigned)tn);
		  abort ();
		}
	      rewind (tmp);
	      memset (bp, 0, rn);
	      tn = fread (bp, 1, rn, tmp);
	      if (tn != rn)
		{
		  fprintf (stderr,
			   "fread failed, expected %lu bytes, got only %lu.\n",
			   (unsigned long) rn, (unsigned long) tn);
		  abort ();
		}

	      if (memcmp (bp, rp, rn) != 0)
		{
		  fprintf (stderr, "mpq_out_str failed:\n");
		  dump ("a_num", mpq_numref (a));
		  dump ("a_den", mpq_denref (a));
		  fprintf (stderr, "b = %s\n", bp);
		  fprintf (stderr, "  base = %d\n", base);
		  fprintf (stderr, "r = %s\n", rp);
		  abort ();
		}
	    }

	  mpq_set_str (b, rp, base);

	  if (!mpq_equal (a, b))
	    {
	      fprintf (stderr, "mpq_set_str failed:\n");
	      fprintf (stderr, "r = %s\n", rp);
	      fprintf (stderr, "  base = %d\n", base);
	      fprintf (stderr, "r = %s\n", ap);
	      fprintf (stderr, "  base = 16\n");
	      dump ("b_num", mpq_numref (b));
	      dump ("b_den", mpq_denref (b));
	      dump ("r_num", mpq_numref (a));
	      dump ("r_den", mpq_denref (a));
	      abort ();
	    }

	  free (ap);
	  free (rp);
	  testfree (bp);
	}
    }
  mpq_clear (a);
  mpq_clear (b);
}
