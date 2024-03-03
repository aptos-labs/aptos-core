/*
Copyright 2020 Free Software Foundation, Inc.

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

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

typedef unsigned long mp_limb_t; /* neat */

void
one (const char *op, size_t ind, mp_limb_t m0, mp_limb_t s0)
{
  printf ("static void f%zu(mp_limb_t*r1p,mp_limb_t*r0p){", ind);
  printf ("mp_limb_t r1,r0;");
  printf ("%s(r1,r0,0,%ld,0,%ld);", op, (long) m0, (long) s0);
  printf ("*r1p=r1;*r0p=r0;");
  printf ("}\n");
}

mp_limb_t ops[1000];

enum what_t {ADD, SUB};

int
main (int argc, char **argv)
{
  size_t n_operands = 0;
  size_t n_functions = 0;
  const char *op;
  enum what_t what;

  if (argc == 2 && strcmp (argv[1], "add") == 0)
    {
      op = "add_ssaaaa";
      what = ADD;
    }
  else if (argc == 2 && strcmp (argv[1], "sub") == 0)
    {
      op = "sub_ddmmss";
      what = SUB;
    }
  else
    {
      fprintf (stderr, "what do yuo want me to do?\n");
      exit (1);
    }

  for (int i = 0; i < 16; i++)
    {
      ops[n_operands++] = 1 << i;
      ops[n_operands++] = -(1 << i);
      ops[n_operands++] = (1 << i) - 1;
      ops[n_operands++] = -(1 << i) - 1;
    }

  printf ("#include <stdlib.h>\n");
  printf ("#include <stdio.h>\n");
  printf ("#include \"gmp-impl.h\"\n");
  printf ("#include \"longlong.h\"\n");

  /* Print out ops[] definition.  */
  printf ("static const int ops[%zu] = {\n", n_operands);
  for (int i = 0; i < n_operands; i++)
    {
      printf ("%ld,", (long) ops[i]);
      if ((i + 1) % 4 == 0)
	puts ("");
    }
  printf ("};\n");

  /* Generate functions and print them.  */
  for (int i = 0; i < n_operands; i++)
    {
      for (int j = 0; j < n_operands; j++)
	{
	  one (op, n_functions++, ops[i], ops[j]);
	}
    }

  /* Print out function pointer table.  */
  printf ("typedef void (*func_t) (mp_limb_t*, mp_limb_t*);\n");
  printf ("static const func_t funcs[%zu] = {\n", n_functions);
  for (size_t i = 0; i < n_functions; i++)
    {
      printf ("f%zu,", i);
      if ((i + 1) % 16 == 0)
	puts ("");
    }
  printf ("};\n");

  /* Print out table of reference results.  */
  printf ("static const int ref[%zu][2] = {\n", n_functions);
  for (int i = 0; i < n_operands; i++)
    {
      for (int j = 0; j < n_operands; j++)
	{
	  if (what == ADD)
	    printf ("{%6ld,%2ld},", (long) ( ops[i] + ops[j]), (long) ((mp_limb_t) ((ops[i] + ops[j]) < ops[i])));
	  else     /* SUB */
	    printf ("{%6ld,%2ld},", (long) ( ops[i] - ops[j]), (long) (-(mp_limb_t) (ops[i] < ops[j])));
	  if ((i * n_operands + j) % 8 == 0)
	    puts ("");
	}
    }
  printf ("};\n");

  printf ("int main ()\n{\n");
  printf ("  mp_limb_t r1, r0;\n");
  printf ("  int err = 0;\n");
  printf ("  size_t ind = 0;\n");
  printf ("  for (size_t i = 0; i < %zu; i++)\n", n_functions);
  printf ("    {\n");
  printf ("      int ii = i / %zu, jj = i %% %zu;\n", n_operands, n_operands);
  printf ("      funcs[i](&r1, &r0);\n");
  printf ("      if (r0 != (mp_limb_signed_t) ref[ind][0] || r1 != (mp_limb_signed_t) ref[ind][1]) {\n");
  printf ("         printf (\"error for f%%zu(%%d,%%d): want (%%d,%%d) got (%%d,%%d)\\n\", i, (int) ops[ii], (int) ops[jj], ref[ind][1], ref[ind][0], (int) r1, (int) r0);\n");
  printf ("         err++;\n");
  printf ("       }\n");
  printf ("      ind++;\n");
  printf ("    }\n");

  printf ("  return err != 0;\n");
  printf ("}\n");
  return 0;
}
