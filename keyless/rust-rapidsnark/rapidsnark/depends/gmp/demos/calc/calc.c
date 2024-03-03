/* A Bison parser, made by GNU Bison 3.6.4.  */

/* Bison implementation for Yacc-like parsers in C

   Copyright (C) 1984, 1989-1990, 2000-2015, 2018-2020 Free Software Foundation,
   Inc.

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.  */

/* As a special exception, you may create a larger work that contains
   part or all of the Bison parser skeleton and distribute that work
   under terms of your choice, so long as that work isn't itself a
   parser generator using the skeleton or a modified version thereof
   as a parser skeleton.  Alternatively, if you modify or redistribute
   the parser skeleton itself, you may (at your option) remove this
   special exception, which will cause the skeleton and the resulting
   Bison output files to be licensed under the GNU General Public
   License without this special exception.

   This special exception was added by the Free Software Foundation in
   version 2.2 of Bison.  */

/* C LALR(1) parser skeleton written by Richard Stallman, by
   simplifying the original so-called "semantic" parser.  */

/* DO NOT RELY ON FEATURES THAT ARE NOT DOCUMENTED in the manual,
   especially those whose name start with YY_ or yy_.  They are
   private implementation details that can be changed or removed.  */

/* All symbols defined below should begin with yy or YY, to avoid
   infringing on user name space.  This should be done even for local
   variables, as they might otherwise be expanded by user macros.
   There are some unavoidable exceptions within include files to
   define necessary library symbols; they are noted "INFRINGES ON
   USER NAME SPACE" below.  */

/* Identify Bison output.  */
#define YYBISON 1

/* Bison version.  */
#define YYBISON_VERSION "3.6.4"

/* Skeleton name.  */
#define YYSKELETON_NAME "yacc.c"

/* Pure parsers.  */
#define YYPURE 0

/* Push parsers.  */
#define YYPUSH 0

/* Pull parsers.  */
#define YYPULL 1




/* First part of user prologue.  */
#line 1 "../../../gmp/demos/calc/calc.y"

/* A simple integer desk calculator using yacc and gmp.

Copyright 2000-2002 Free Software Foundation, Inc.

This file is part of the GNU MP Library.

This program is free software; you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation; either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE.  See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program.  If not, see https://www.gnu.org/licenses/.  */


/* This is a simple program, meant only to show one way to use GMP for this
   sort of thing.  There's few features, and error checking is minimal.
   Standard input is read, calc_help() below shows the inputs accepted.

   Expressions are evaluated as they're read.  If user defined functions
   were wanted it'd be necessary to build a parse tree like pexpr.c does, or
   a list of operations for a stack based evaluator.  That would also make
   it possible to detect and optimize evaluations "mod m" like pexpr.c does.

   A stack is used for intermediate values in the expression evaluation,
   separate from the yacc parser stack.  This is simple, makes error
   recovery easy, minimizes the junk around mpz calls in the rules, and
   saves initializing or clearing "mpz_t"s during a calculation.  A
   disadvantage though is that variables must be copied to the stack to be
   worked on.  A more sophisticated calculator or language system might be
   able to avoid that when executing a compiled or semi-compiled form.

   Avoiding repeated initializing and clearing of "mpz_t"s is important.  In
   this program the time spent parsing is obviously much greater than any
   possible saving from this, but a proper calculator or language should
   take some trouble over it.  Don't be surprised if an init/clear takes 3
   or more times as long as a 10 limb addition, depending on the system (see
   the mpz_init_realloc_clear example in tune/README).  */


#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "gmp.h"
#define NO_CALC_H /* because it conflicts with normal calc.c stuff */
#include "calc-common.h"


#define numberof(x)  (sizeof (x) / sizeof ((x)[0]))


void
calc_help (void)
{
  printf ("Examples:\n");
  printf ("    2+3*4        expressions are evaluated\n");
  printf ("    x=5^6        variables a to z can be set and used\n");
  printf ("Operators:\n");
  printf ("    + - *        arithmetic\n");
  printf ("    / %%          division and remainder (rounding towards negative infinity)\n");
  printf ("    ^            exponentiation\n");
  printf ("    !            factorial\n");
  printf ("    << >>        left and right shifts\n");
  printf ("    <= >= >      \\ comparisons, giving 1 if true, 0 if false\n");
  printf ("    == != <      /\n");
  printf ("    && ||        logical and/or, giving 1 if true, 0 if false\n");
  printf ("Functions:\n");
  printf ("    abs(n)       absolute value\n");
  printf ("    bin(n,m)     binomial coefficient\n");
  printf ("    fib(n)       fibonacci number\n");
  printf ("    gcd(a,b,..)  greatest common divisor\n");
  printf ("    kron(a,b)    kronecker symbol\n");
  printf ("    lcm(a,b,..)  least common multiple\n");
  printf ("    lucnum(n)    lucas number\n");
  printf ("    nextprime(n) next prime after n\n");
  printf ("    powm(b,e,m)  modulo powering, b^e%%m\n");
  printf ("    root(n,r)    r-th root\n");
  printf ("    sqrt(n)      square root\n");
  printf ("Other:\n");
  printf ("    hex          \\ set hex or decimal for input and output\n");
  printf ("    decimal      /   (\"0x\" can be used for hex too)\n");
  printf ("    quit         exit program (EOF works too)\n");
  printf ("    ;            statements are separated with a ; or newline\n");
  printf ("    \\            continue expressions with \\ before newline\n");
  printf ("    # xxx        comments are # though to newline\n");
  printf ("Hex numbers must be entered in upper case, to distinguish them from the\n");
  printf ("variables a to f (like in bc).\n");
}


int  ibase = 0;
int  obase = 10;


/* The stack is a fixed size, which means there's a limit on the nesting
   allowed in expressions.  A more sophisticated program could let it grow
   dynamically.  */

mpz_t    stack[100];
mpz_ptr  sp = stack[0];

#define CHECK_OVERFLOW()                                                  \
  if (sp >= stack[numberof(stack)])	/* FIXME */			\
    {                                                                     \
      fprintf (stderr,                                                    \
               "Value stack overflow, too much nesting in expression\n"); \
      YYERROR;                                                            \
    }

#define CHECK_EMPTY()                                                   \
  if (sp != stack[0])                                                   \
    {                                                                   \
      fprintf (stderr, "Oops, expected the value stack to be empty\n"); \
      sp = stack[0];                                                    \
    }


mpz_t  variable[26];

#define CHECK_VARIABLE(var)                                             \
  if ((var) < 0 || (var) >= numberof (variable))                        \
    {                                                                   \
      fprintf (stderr, "Oops, bad variable somehow: %d\n", var);        \
      YYERROR;                                                          \
    }


#define CHECK_UI(name,z)                        \
  if (! mpz_fits_ulong_p (z))                   \
    {                                           \
      fprintf (stderr, "%s too big\n", name);   \
      YYERROR;                                  \
    }


#line 212 "calc.c"

# ifndef YY_CAST
#  ifdef __cplusplus
#   define YY_CAST(Type, Val) static_cast<Type> (Val)
#   define YY_REINTERPRET_CAST(Type, Val) reinterpret_cast<Type> (Val)
#  else
#   define YY_CAST(Type, Val) ((Type) (Val))
#   define YY_REINTERPRET_CAST(Type, Val) ((Type) (Val))
#  endif
# endif
# ifndef YY_NULLPTR
#  if defined __cplusplus
#   if 201103L <= __cplusplus
#    define YY_NULLPTR nullptr
#   else
#    define YY_NULLPTR 0
#   endif
#  else
#   define YY_NULLPTR ((void*)0)
#  endif
# endif

/* Use api.header.include to #include this header
   instead of duplicating it here.  */
#ifndef YY_YY_CALC_H_INCLUDED
# define YY_YY_CALC_H_INCLUDED
/* Debug traces.  */
#ifndef YYDEBUG
# define YYDEBUG 0
#endif
#if YYDEBUG
extern int yydebug;
#endif

/* Token kinds.  */
#ifndef YYTOKENTYPE
# define YYTOKENTYPE
  enum yytokentype
  {
    YYEMPTY = -2,
    YYEOF = 0,                     /* "end of file"  */
    YYerror = 256,                 /* error  */
    YYUNDEF = 257,                 /* "invalid token"  */
    EOS = 258,                     /* EOS  */
    BAD = 259,                     /* BAD  */
    HELP = 260,                    /* HELP  */
    HEX = 261,                     /* HEX  */
    DECIMAL = 262,                 /* DECIMAL  */
    QUIT = 263,                    /* QUIT  */
    ABS = 264,                     /* ABS  */
    BIN = 265,                     /* BIN  */
    FIB = 266,                     /* FIB  */
    GCD = 267,                     /* GCD  */
    KRON = 268,                    /* KRON  */
    LCM = 269,                     /* LCM  */
    LUCNUM = 270,                  /* LUCNUM  */
    NEXTPRIME = 271,               /* NEXTPRIME  */
    POWM = 272,                    /* POWM  */
    ROOT = 273,                    /* ROOT  */
    SQRT = 274,                    /* SQRT  */
    NUMBER = 275,                  /* NUMBER  */
    VARIABLE = 276,                /* VARIABLE  */
    LOR = 277,                     /* LOR  */
    LAND = 278,                    /* LAND  */
    EQ = 279,                      /* EQ  */
    NE = 280,                      /* NE  */
    LE = 281,                      /* LE  */
    GE = 282,                      /* GE  */
    LSHIFT = 283,                  /* LSHIFT  */
    RSHIFT = 284,                  /* RSHIFT  */
    UMINUS = 285                   /* UMINUS  */
  };
  typedef enum yytokentype yytoken_kind_t;
#endif
/* Token kinds.  */
#define YYEOF 0
#define YYerror 256
#define YYUNDEF 257
#define EOS 258
#define BAD 259
#define HELP 260
#define HEX 261
#define DECIMAL 262
#define QUIT 263
#define ABS 264
#define BIN 265
#define FIB 266
#define GCD 267
#define KRON 268
#define LCM 269
#define LUCNUM 270
#define NEXTPRIME 271
#define POWM 272
#define ROOT 273
#define SQRT 274
#define NUMBER 275
#define VARIABLE 276
#define LOR 277
#define LAND 278
#define EQ 279
#define NE 280
#define LE 281
#define GE 282
#define LSHIFT 283
#define RSHIFT 284
#define UMINUS 285

/* Value type.  */
#if ! defined YYSTYPE && ! defined YYSTYPE_IS_DECLARED
union YYSTYPE
{
#line 142 "../../../gmp/demos/calc/calc.y"

  char  *str;
  int   var;

#line 329 "calc.c"

};
typedef union YYSTYPE YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif


extern YYSTYPE yylval;

int yyparse (void);

#endif /* !YY_YY_CALC_H_INCLUDED  */
/* Symbol kind.  */
enum yysymbol_kind_t
{
  YYSYMBOL_YYEMPTY = -2,
  YYSYMBOL_YYEOF = 0,                      /* "end of file"  */
  YYSYMBOL_YYerror = 1,                    /* error  */
  YYSYMBOL_YYUNDEF = 2,                    /* "invalid token"  */
  YYSYMBOL_EOS = 3,                        /* EOS  */
  YYSYMBOL_BAD = 4,                        /* BAD  */
  YYSYMBOL_HELP = 5,                       /* HELP  */
  YYSYMBOL_HEX = 6,                        /* HEX  */
  YYSYMBOL_DECIMAL = 7,                    /* DECIMAL  */
  YYSYMBOL_QUIT = 8,                       /* QUIT  */
  YYSYMBOL_ABS = 9,                        /* ABS  */
  YYSYMBOL_BIN = 10,                       /* BIN  */
  YYSYMBOL_FIB = 11,                       /* FIB  */
  YYSYMBOL_GCD = 12,                       /* GCD  */
  YYSYMBOL_KRON = 13,                      /* KRON  */
  YYSYMBOL_LCM = 14,                       /* LCM  */
  YYSYMBOL_LUCNUM = 15,                    /* LUCNUM  */
  YYSYMBOL_NEXTPRIME = 16,                 /* NEXTPRIME  */
  YYSYMBOL_POWM = 17,                      /* POWM  */
  YYSYMBOL_ROOT = 18,                      /* ROOT  */
  YYSYMBOL_SQRT = 19,                      /* SQRT  */
  YYSYMBOL_NUMBER = 20,                    /* NUMBER  */
  YYSYMBOL_VARIABLE = 21,                  /* VARIABLE  */
  YYSYMBOL_LOR = 22,                       /* LOR  */
  YYSYMBOL_LAND = 23,                      /* LAND  */
  YYSYMBOL_24_ = 24,                       /* '<'  */
  YYSYMBOL_25_ = 25,                       /* '>'  */
  YYSYMBOL_EQ = 26,                        /* EQ  */
  YYSYMBOL_NE = 27,                        /* NE  */
  YYSYMBOL_LE = 28,                        /* LE  */
  YYSYMBOL_GE = 29,                        /* GE  */
  YYSYMBOL_LSHIFT = 30,                    /* LSHIFT  */
  YYSYMBOL_RSHIFT = 31,                    /* RSHIFT  */
  YYSYMBOL_32_ = 32,                       /* '+'  */
  YYSYMBOL_33_ = 33,                       /* '-'  */
  YYSYMBOL_34_ = 34,                       /* '*'  */
  YYSYMBOL_35_ = 35,                       /* '/'  */
  YYSYMBOL_36_ = 36,                       /* '%'  */
  YYSYMBOL_UMINUS = 37,                    /* UMINUS  */
  YYSYMBOL_38_ = 38,                       /* '^'  */
  YYSYMBOL_39_ = 39,                       /* '!'  */
  YYSYMBOL_40_ = 40,                       /* '='  */
  YYSYMBOL_41_ = 41,                       /* '('  */
  YYSYMBOL_42_ = 42,                       /* ')'  */
  YYSYMBOL_43_ = 43,                       /* ','  */
  YYSYMBOL_YYACCEPT = 44,                  /* $accept  */
  YYSYMBOL_top = 45,                       /* top  */
  YYSYMBOL_statements = 46,                /* statements  */
  YYSYMBOL_statement = 47,                 /* statement  */
  YYSYMBOL_e = 48,                         /* e  */
  YYSYMBOL_gcdlist = 49,                   /* gcdlist  */
  YYSYMBOL_lcmlist = 50                    /* lcmlist  */
};
typedef enum yysymbol_kind_t yysymbol_kind_t;




#ifdef short
# undef short
#endif

/* On compilers that do not define __PTRDIFF_MAX__ etc., make sure
   <limits.h> and (if available) <stdint.h> are included
   so that the code can choose integer types of a good width.  */

#ifndef __PTRDIFF_MAX__
# include <limits.h> /* INFRINGES ON USER NAME SPACE */
# if defined __STDC_VERSION__ && 199901 <= __STDC_VERSION__
#  include <stdint.h> /* INFRINGES ON USER NAME SPACE */
#  define YY_STDINT_H
# endif
#endif

/* Narrow types that promote to a signed type and that can represent a
   signed or unsigned integer of at least N bits.  In tables they can
   save space and decrease cache pressure.  Promoting to a signed type
   helps avoid bugs in integer arithmetic.  */

#ifdef __INT_LEAST8_MAX__
typedef __INT_LEAST8_TYPE__ yytype_int8;
#elif defined YY_STDINT_H
typedef int_least8_t yytype_int8;
#else
typedef signed char yytype_int8;
#endif

#ifdef __INT_LEAST16_MAX__
typedef __INT_LEAST16_TYPE__ yytype_int16;
#elif defined YY_STDINT_H
typedef int_least16_t yytype_int16;
#else
typedef short yytype_int16;
#endif

#if defined __UINT_LEAST8_MAX__ && __UINT_LEAST8_MAX__ <= __INT_MAX__
typedef __UINT_LEAST8_TYPE__ yytype_uint8;
#elif (!defined __UINT_LEAST8_MAX__ && defined YY_STDINT_H \
       && UINT_LEAST8_MAX <= INT_MAX)
typedef uint_least8_t yytype_uint8;
#elif !defined __UINT_LEAST8_MAX__ && UCHAR_MAX <= INT_MAX
typedef unsigned char yytype_uint8;
#else
typedef short yytype_uint8;
#endif

#if defined __UINT_LEAST16_MAX__ && __UINT_LEAST16_MAX__ <= __INT_MAX__
typedef __UINT_LEAST16_TYPE__ yytype_uint16;
#elif (!defined __UINT_LEAST16_MAX__ && defined YY_STDINT_H \
       && UINT_LEAST16_MAX <= INT_MAX)
typedef uint_least16_t yytype_uint16;
#elif !defined __UINT_LEAST16_MAX__ && USHRT_MAX <= INT_MAX
typedef unsigned short yytype_uint16;
#else
typedef int yytype_uint16;
#endif

#ifndef YYPTRDIFF_T
# if defined __PTRDIFF_TYPE__ && defined __PTRDIFF_MAX__
#  define YYPTRDIFF_T __PTRDIFF_TYPE__
#  define YYPTRDIFF_MAXIMUM __PTRDIFF_MAX__
# elif defined PTRDIFF_MAX
#  ifndef ptrdiff_t
#   include <stddef.h> /* INFRINGES ON USER NAME SPACE */
#  endif
#  define YYPTRDIFF_T ptrdiff_t
#  define YYPTRDIFF_MAXIMUM PTRDIFF_MAX
# else
#  define YYPTRDIFF_T long
#  define YYPTRDIFF_MAXIMUM LONG_MAX
# endif
#endif

#ifndef YYSIZE_T
# ifdef __SIZE_TYPE__
#  define YYSIZE_T __SIZE_TYPE__
# elif defined size_t
#  define YYSIZE_T size_t
# elif defined __STDC_VERSION__ && 199901 <= __STDC_VERSION__
#  include <stddef.h> /* INFRINGES ON USER NAME SPACE */
#  define YYSIZE_T size_t
# else
#  define YYSIZE_T unsigned
# endif
#endif

#define YYSIZE_MAXIMUM                                  \
  YY_CAST (YYPTRDIFF_T,                                 \
           (YYPTRDIFF_MAXIMUM < YY_CAST (YYSIZE_T, -1)  \
            ? YYPTRDIFF_MAXIMUM                         \
            : YY_CAST (YYSIZE_T, -1)))

#define YYSIZEOF(X) YY_CAST (YYPTRDIFF_T, sizeof (X))


/* Stored state numbers (used for stacks). */
typedef yytype_int8 yy_state_t;

/* State numbers in computations.  */
typedef int yy_state_fast_t;

#ifndef YY_
# if defined YYENABLE_NLS && YYENABLE_NLS
#  if ENABLE_NLS
#   include <libintl.h> /* INFRINGES ON USER NAME SPACE */
#   define YY_(Msgid) dgettext ("bison-runtime", Msgid)
#  endif
# endif
# ifndef YY_
#  define YY_(Msgid) Msgid
# endif
#endif


#ifndef YY_ATTRIBUTE_PURE
# if defined __GNUC__ && 2 < __GNUC__ + (96 <= __GNUC_MINOR__)
#  define YY_ATTRIBUTE_PURE __attribute__ ((__pure__))
# else
#  define YY_ATTRIBUTE_PURE
# endif
#endif

#ifndef YY_ATTRIBUTE_UNUSED
# if defined __GNUC__ && 2 < __GNUC__ + (7 <= __GNUC_MINOR__)
#  define YY_ATTRIBUTE_UNUSED __attribute__ ((__unused__))
# else
#  define YY_ATTRIBUTE_UNUSED
# endif
#endif

/* Suppress unused-variable warnings by "using" E.  */
#if ! defined lint || defined __GNUC__
# define YYUSE(E) ((void) (E))
#else
# define YYUSE(E) /* empty */
#endif

#if defined __GNUC__ && ! defined __ICC && 407 <= __GNUC__ * 100 + __GNUC_MINOR__
/* Suppress an incorrect diagnostic about yylval being uninitialized.  */
# define YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN                            \
    _Pragma ("GCC diagnostic push")                                     \
    _Pragma ("GCC diagnostic ignored \"-Wuninitialized\"")              \
    _Pragma ("GCC diagnostic ignored \"-Wmaybe-uninitialized\"")
# define YY_IGNORE_MAYBE_UNINITIALIZED_END      \
    _Pragma ("GCC diagnostic pop")
#else
# define YY_INITIAL_VALUE(Value) Value
#endif
#ifndef YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
# define YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
# define YY_IGNORE_MAYBE_UNINITIALIZED_END
#endif
#ifndef YY_INITIAL_VALUE
# define YY_INITIAL_VALUE(Value) /* Nothing. */
#endif

#if defined __cplusplus && defined __GNUC__ && ! defined __ICC && 6 <= __GNUC__
# define YY_IGNORE_USELESS_CAST_BEGIN                          \
    _Pragma ("GCC diagnostic push")                            \
    _Pragma ("GCC diagnostic ignored \"-Wuseless-cast\"")
# define YY_IGNORE_USELESS_CAST_END            \
    _Pragma ("GCC diagnostic pop")
#endif
#ifndef YY_IGNORE_USELESS_CAST_BEGIN
# define YY_IGNORE_USELESS_CAST_BEGIN
# define YY_IGNORE_USELESS_CAST_END
#endif


#define YY_ASSERT(E) ((void) (0 && (E)))

#if !defined yyoverflow

/* The parser invokes alloca or malloc; define the necessary symbols.  */

# ifdef YYSTACK_USE_ALLOCA
#  if YYSTACK_USE_ALLOCA
#   ifdef __GNUC__
#    define YYSTACK_ALLOC __builtin_alloca
#   elif defined __BUILTIN_VA_ARG_INCR
#    include <alloca.h> /* INFRINGES ON USER NAME SPACE */
#   elif defined _AIX
#    define YYSTACK_ALLOC __alloca
#   elif defined _MSC_VER
#    include <malloc.h> /* INFRINGES ON USER NAME SPACE */
#    define alloca _alloca
#   else
#    define YYSTACK_ALLOC alloca
#    if ! defined _ALLOCA_H && ! defined EXIT_SUCCESS
#     include <stdlib.h> /* INFRINGES ON USER NAME SPACE */
      /* Use EXIT_SUCCESS as a witness for stdlib.h.  */
#     ifndef EXIT_SUCCESS
#      define EXIT_SUCCESS 0
#     endif
#    endif
#   endif
#  endif
# endif

# ifdef YYSTACK_ALLOC
   /* Pacify GCC's 'empty if-body' warning.  */
#  define YYSTACK_FREE(Ptr) do { /* empty */; } while (0)
#  ifndef YYSTACK_ALLOC_MAXIMUM
    /* The OS might guarantee only one guard page at the bottom of the stack,
       and a page size can be as small as 4096 bytes.  So we cannot safely
       invoke alloca (N) if N exceeds 4096.  Use a slightly smaller number
       to allow for a few compiler-allocated temporary stack slots.  */
#   define YYSTACK_ALLOC_MAXIMUM 4032 /* reasonable circa 2006 */
#  endif
# else
#  define YYSTACK_ALLOC YYMALLOC
#  define YYSTACK_FREE YYFREE
#  ifndef YYSTACK_ALLOC_MAXIMUM
#   define YYSTACK_ALLOC_MAXIMUM YYSIZE_MAXIMUM
#  endif
#  if (defined __cplusplus && ! defined EXIT_SUCCESS \
       && ! ((defined YYMALLOC || defined malloc) \
             && (defined YYFREE || defined free)))
#   include <stdlib.h> /* INFRINGES ON USER NAME SPACE */
#   ifndef EXIT_SUCCESS
#    define EXIT_SUCCESS 0
#   endif
#  endif
#  ifndef YYMALLOC
#   define YYMALLOC malloc
#   if ! defined malloc && ! defined EXIT_SUCCESS
void *malloc (YYSIZE_T); /* INFRINGES ON USER NAME SPACE */
#   endif
#  endif
#  ifndef YYFREE
#   define YYFREE free
#   if ! defined free && ! defined EXIT_SUCCESS
void free (void *); /* INFRINGES ON USER NAME SPACE */
#   endif
#  endif
# endif
#endif /* !defined yyoverflow */

#if (! defined yyoverflow \
     && (! defined __cplusplus \
         || (defined YYSTYPE_IS_TRIVIAL && YYSTYPE_IS_TRIVIAL)))

/* A type that is properly aligned for any stack member.  */
union yyalloc
{
  yy_state_t yyss_alloc;
  YYSTYPE yyvs_alloc;
};

/* The size of the maximum gap between one aligned stack and the next.  */
# define YYSTACK_GAP_MAXIMUM (YYSIZEOF (union yyalloc) - 1)

/* The size of an array large to enough to hold all stacks, each with
   N elements.  */
# define YYSTACK_BYTES(N) \
     ((N) * (YYSIZEOF (yy_state_t) + YYSIZEOF (YYSTYPE)) \
      + YYSTACK_GAP_MAXIMUM)

# define YYCOPY_NEEDED 1

/* Relocate STACK from its old location to the new one.  The
   local variables YYSIZE and YYSTACKSIZE give the old and new number of
   elements in the stack, and YYPTR gives the new location of the
   stack.  Advance YYPTR to a properly aligned location for the next
   stack.  */
# define YYSTACK_RELOCATE(Stack_alloc, Stack)                           \
    do                                                                  \
      {                                                                 \
        YYPTRDIFF_T yynewbytes;                                         \
        YYCOPY (&yyptr->Stack_alloc, Stack, yysize);                    \
        Stack = &yyptr->Stack_alloc;                                    \
        yynewbytes = yystacksize * YYSIZEOF (*Stack) + YYSTACK_GAP_MAXIMUM; \
        yyptr += yynewbytes / YYSIZEOF (*yyptr);                        \
      }                                                                 \
    while (0)

#endif

#if defined YYCOPY_NEEDED && YYCOPY_NEEDED
/* Copy COUNT objects from SRC to DST.  The source and destination do
   not overlap.  */
# ifndef YYCOPY
#  if defined __GNUC__ && 1 < __GNUC__
#   define YYCOPY(Dst, Src, Count) \
      __builtin_memcpy (Dst, Src, YY_CAST (YYSIZE_T, (Count)) * sizeof (*(Src)))
#  else
#   define YYCOPY(Dst, Src, Count)              \
      do                                        \
        {                                       \
          YYPTRDIFF_T yyi;                      \
          for (yyi = 0; yyi < (Count); yyi++)   \
            (Dst)[yyi] = (Src)[yyi];            \
        }                                       \
      while (0)
#  endif
# endif
#endif /* !YYCOPY_NEEDED */

/* YYFINAL -- State number of the termination state.  */
#define YYFINAL  41
/* YYLAST -- Last index in YYTABLE.  */
#define YYLAST   552

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  44
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  7
/* YYNRULES -- Number of rules.  */
#define YYNRULES  49
/* YYNSTATES -- Number of states.  */
#define YYNSTATES  118

#define YYMAXUTOK   285


/* YYTRANSLATE(TOKEN-NUM) -- Symbol number corresponding to TOKEN-NUM
   as returned by yylex, with out-of-bounds checking.  */
#define YYTRANSLATE(YYX)                                \
  (0 <= (YYX) && (YYX) <= YYMAXUTOK                     \
   ? YY_CAST (yysymbol_kind_t, yytranslate[YYX])        \
   : YYSYMBOL_YYUNDEF)

/* YYTRANSLATE[TOKEN-NUM] -- Symbol number corresponding to TOKEN-NUM
   as returned by yylex.  */
static const yytype_int8 yytranslate[] =
{
       0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,    39,     2,     2,     2,    36,     2,     2,
      41,    42,    34,    32,    43,    33,     2,    35,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
      24,    40,    25,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    38,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     1,     2,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,    26,
      27,    28,    29,    30,    31,    37
};

#if YYDEBUG
  /* YYRLINE[YYN] -- Source line where rule number YYN was defined.  */
static const yytype_int16 yyrline[] =
{
       0,   167,   167,   168,   171,   172,   173,   175,   177,   182,
     188,   189,   190,   191,   197,   198,   199,   200,   201,   202,
     203,   205,   207,   209,   211,   213,   214,   215,   216,   217,
     218,   220,   221,   223,   224,   226,   228,   229,   231,   232,
     234,   235,   236,   238,   240,   246,   257,   258,   261,   262
};
#endif

/** Accessing symbol of state STATE.  */
#define YY_ACCESSING_SYMBOL(State) YY_CAST (yysymbol_kind_t, yystos[State])

#if YYDEBUG || 0
/* The user-facing name of the symbol whose (internal) number is
   YYSYMBOL.  No bounds checking.  */
static const char *yysymbol_name (yysymbol_kind_t yysymbol) YY_ATTRIBUTE_UNUSED;

/* YYTNAME[SYMBOL-NUM] -- String name of the symbol SYMBOL-NUM.
   First, the terminals, then, starting at YYNTOKENS, nonterminals.  */
static const char *const yytname[] =
{
  "\"end of file\"", "error", "\"invalid token\"", "EOS", "BAD", "HELP",
  "HEX", "DECIMAL", "QUIT", "ABS", "BIN", "FIB", "GCD", "KRON", "LCM",
  "LUCNUM", "NEXTPRIME", "POWM", "ROOT", "SQRT", "NUMBER", "VARIABLE",
  "LOR", "LAND", "'<'", "'>'", "EQ", "NE", "LE", "GE", "LSHIFT", "RSHIFT",
  "'+'", "'-'", "'*'", "'/'", "'%'", "UMINUS", "'^'", "'!'", "'='", "'('",
  "')'", "','", "$accept", "top", "statements", "statement", "e",
  "gcdlist", "lcmlist", YY_NULLPTR
};

static const char *
yysymbol_name (yysymbol_kind_t yysymbol)
{
  return yytname[yysymbol];
}
#endif

#ifdef YYPRINT
/* YYTOKNUM[NUM] -- (External) token number corresponding to the
   (internal) symbol number NUM (which must be that of a token).  */
static const yytype_int16 yytoknum[] =
{
       0,   256,   257,   258,   259,   260,   261,   262,   263,   264,
     265,   266,   267,   268,   269,   270,   271,   272,   273,   274,
     275,   276,   277,   278,    60,    62,   279,   280,   281,   282,
     283,   284,    43,    45,    42,    47,    37,   285,    94,    33,
      61,    40,    41,    44
};
#endif

#define YYPACT_NINF (-39)

#define yypact_value_is_default(Yyn) \
  ((Yyn) == YYPACT_NINF)

#define YYTABLE_NINF (-8)

#define yytable_value_is_error(Yyn) \
  ((Yyn) == YYTABLE_NINF)

  /* YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
     STATE-NUM.  */
static const yytype_int16 yypact[] =
{
      41,     3,   -39,   -39,   -39,   -39,     2,     4,    27,    32,
      35,    36,    39,    42,    45,    46,    47,   -39,   -18,   124,
     124,    89,    91,    87,   464,   -39,   124,   124,   124,   124,
     124,   124,   124,   124,   124,   124,   124,   124,   -39,   -36,
     254,   -39,    88,   -39,   124,   124,   124,   124,   124,   124,
     124,   124,   124,   124,   124,   124,   124,   124,   124,   124,
     -39,   275,   144,   296,   464,   -38,   166,   464,    29,   317,
     338,   188,   210,   359,   464,   -39,   -39,   481,   497,   513,
     513,   513,   513,   513,   513,    31,    31,   -15,   -15,   -36,
     -36,   -36,   -36,   -39,   124,   -39,   -39,   124,   124,   -39,
     124,   -39,   -39,   124,   124,   -39,   380,   464,   401,   464,
     232,   422,   -39,   -39,   124,   -39,   443,   -39
};

  /* YYDEFACT[STATE-NUM] -- Default reduction number in state STATE-NUM.
     Performed when YYTABLE does not specify something else to do.  Zero
     means the default is an error.  */
static const yytype_int8 yydefact[] =
{
       0,     0,    10,    11,    12,    13,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,    45,    44,     0,
       0,     0,     7,     2,     8,     6,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    44,    24,
       0,     1,     3,     4,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      23,     0,     0,     0,    46,     0,     0,    48,     0,     0,
       0,     0,     0,     0,     9,    14,     5,    32,    31,    25,
      30,    27,    28,    26,    29,    21,    22,    15,    16,    17,
      18,    19,    20,    33,     0,    35,    36,     0,     0,    38,
       0,    39,    40,     0,     0,    43,     0,    47,     0,    49,
       0,     0,    34,    37,     0,    42,     0,    41
};

  /* YYPGOTO[NTERM-NUM].  */
static const yytype_int8 yypgoto[] =
{
     -39,   -39,   -39,    70,   -19,   -39,   -39
};

  /* YYDEFGOTO[NTERM-NUM].  */
static const yytype_int8 yydefgoto[] =
{
      -1,    21,    22,    23,    24,    65,    68
};

  /* YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
     positive, shift that token.  If negative, reduce the rule whose
     number is the opposite.  If YYTABLE_NINF, syntax error.  */
static const yytype_int8 yytable[] =
{
      39,    40,    59,    60,    96,    97,    25,    61,    62,    63,
      64,    66,    67,    69,    70,    71,    72,    73,    74,    56,
      57,    58,    37,    59,    60,    77,    78,    79,    80,    81,
      82,    83,    84,    85,    86,    87,    88,    89,    90,    91,
      92,    -7,     1,    26,    -7,    27,     2,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    54,    55,    56,    57,    58,    28,    59,
      60,    99,   100,    29,    19,   106,    30,    31,   107,   108,
      32,   109,    20,    33,   110,   111,    34,    35,    36,    41,
      43,    76,    42,     0,     0,   116,     2,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,    19,     0,     0,     0,     0,     0,
       0,     0,    20,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    38,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,    19,     0,     0,
       0,     0,     0,     0,     0,    20,    44,    45,    46,    47,
      48,    49,    50,    51,    52,    53,    54,    55,    56,    57,
      58,     0,    59,    60,     0,     0,     0,    94,    44,    45,
      46,    47,    48,    49,    50,    51,    52,    53,    54,    55,
      56,    57,    58,     0,    59,    60,     0,     0,     0,    98,
      44,    45,    46,    47,    48,    49,    50,    51,    52,    53,
      54,    55,    56,    57,    58,     0,    59,    60,     0,     0,
       0,   103,    44,    45,    46,    47,    48,    49,    50,    51,
      52,    53,    54,    55,    56,    57,    58,     0,    59,    60,
       0,     0,     0,   104,    44,    45,    46,    47,    48,    49,
      50,    51,    52,    53,    54,    55,    56,    57,    58,     0,
      59,    60,     0,     0,     0,   114,    44,    45,    46,    47,
      48,    49,    50,    51,    52,    53,    54,    55,    56,    57,
      58,     0,    59,    60,     0,     0,    75,    44,    45,    46,
      47,    48,    49,    50,    51,    52,    53,    54,    55,    56,
      57,    58,     0,    59,    60,     0,     0,    93,    44,    45,
      46,    47,    48,    49,    50,    51,    52,    53,    54,    55,
      56,    57,    58,     0,    59,    60,     0,     0,    95,    44,
      45,    46,    47,    48,    49,    50,    51,    52,    53,    54,
      55,    56,    57,    58,     0,    59,    60,     0,     0,   101,
      44,    45,    46,    47,    48,    49,    50,    51,    52,    53,
      54,    55,    56,    57,    58,     0,    59,    60,     0,     0,
     102,    44,    45,    46,    47,    48,    49,    50,    51,    52,
      53,    54,    55,    56,    57,    58,     0,    59,    60,     0,
       0,   105,    44,    45,    46,    47,    48,    49,    50,    51,
      52,    53,    54,    55,    56,    57,    58,     0,    59,    60,
       0,     0,   112,    44,    45,    46,    47,    48,    49,    50,
      51,    52,    53,    54,    55,    56,    57,    58,     0,    59,
      60,     0,     0,   113,    44,    45,    46,    47,    48,    49,
      50,    51,    52,    53,    54,    55,    56,    57,    58,     0,
      59,    60,     0,     0,   115,    44,    45,    46,    47,    48,
      49,    50,    51,    52,    53,    54,    55,    56,    57,    58,
       0,    59,    60,     0,     0,   117,    44,    45,    46,    47,
      48,    49,    50,    51,    52,    53,    54,    55,    56,    57,
      58,     0,    59,    60,    45,    46,    47,    48,    49,    50,
      51,    52,    53,    54,    55,    56,    57,    58,     0,    59,
      60,    46,    47,    48,    49,    50,    51,    52,    53,    54,
      55,    56,    57,    58,     0,    59,    60,    -8,    -8,    -8,
      -8,    -8,    -8,    52,    53,    54,    55,    56,    57,    58,
       0,    59,    60
};

static const yytype_int8 yycheck[] =
{
      19,    20,    38,    39,    42,    43,     3,    26,    27,    28,
      29,    30,    31,    32,    33,    34,    35,    36,    37,    34,
      35,    36,    40,    38,    39,    44,    45,    46,    47,    48,
      49,    50,    51,    52,    53,    54,    55,    56,    57,    58,
      59,     0,     1,    41,     3,    41,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    32,    33,    34,    35,    36,    41,    38,
      39,    42,    43,    41,    33,    94,    41,    41,    97,    98,
      41,   100,    41,    41,   103,   104,    41,    41,    41,     0,
       3,     3,    22,    -1,    -1,   114,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    33,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    41,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    33,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    41,    22,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    -1,    38,    39,    -1,    -1,    -1,    43,    22,    23,
      24,    25,    26,    27,    28,    29,    30,    31,    32,    33,
      34,    35,    36,    -1,    38,    39,    -1,    -1,    -1,    43,
      22,    23,    24,    25,    26,    27,    28,    29,    30,    31,
      32,    33,    34,    35,    36,    -1,    38,    39,    -1,    -1,
      -1,    43,    22,    23,    24,    25,    26,    27,    28,    29,
      30,    31,    32,    33,    34,    35,    36,    -1,    38,    39,
      -1,    -1,    -1,    43,    22,    23,    24,    25,    26,    27,
      28,    29,    30,    31,    32,    33,    34,    35,    36,    -1,
      38,    39,    -1,    -1,    -1,    43,    22,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    -1,    38,    39,    -1,    -1,    42,    22,    23,    24,
      25,    26,    27,    28,    29,    30,    31,    32,    33,    34,
      35,    36,    -1,    38,    39,    -1,    -1,    42,    22,    23,
      24,    25,    26,    27,    28,    29,    30,    31,    32,    33,
      34,    35,    36,    -1,    38,    39,    -1,    -1,    42,    22,
      23,    24,    25,    26,    27,    28,    29,    30,    31,    32,
      33,    34,    35,    36,    -1,    38,    39,    -1,    -1,    42,
      22,    23,    24,    25,    26,    27,    28,    29,    30,    31,
      32,    33,    34,    35,    36,    -1,    38,    39,    -1,    -1,
      42,    22,    23,    24,    25,    26,    27,    28,    29,    30,
      31,    32,    33,    34,    35,    36,    -1,    38,    39,    -1,
      -1,    42,    22,    23,    24,    25,    26,    27,    28,    29,
      30,    31,    32,    33,    34,    35,    36,    -1,    38,    39,
      -1,    -1,    42,    22,    23,    24,    25,    26,    27,    28,
      29,    30,    31,    32,    33,    34,    35,    36,    -1,    38,
      39,    -1,    -1,    42,    22,    23,    24,    25,    26,    27,
      28,    29,    30,    31,    32,    33,    34,    35,    36,    -1,
      38,    39,    -1,    -1,    42,    22,    23,    24,    25,    26,
      27,    28,    29,    30,    31,    32,    33,    34,    35,    36,
      -1,    38,    39,    -1,    -1,    42,    22,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    -1,    38,    39,    23,    24,    25,    26,    27,    28,
      29,    30,    31,    32,    33,    34,    35,    36,    -1,    38,
      39,    24,    25,    26,    27,    28,    29,    30,    31,    32,
      33,    34,    35,    36,    -1,    38,    39,    24,    25,    26,
      27,    28,    29,    30,    31,    32,    33,    34,    35,    36,
      -1,    38,    39
};

  /* YYSTOS[STATE-NUM] -- The (internal number of the) accessing
     symbol of state STATE-NUM.  */
static const yytype_int8 yystos[] =
{
       0,     1,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    33,
      41,    45,    46,    47,    48,     3,    41,    41,    41,    41,
      41,    41,    41,    41,    41,    41,    41,    40,    21,    48,
      48,     0,    47,     3,    22,    23,    24,    25,    26,    27,
      28,    29,    30,    31,    32,    33,    34,    35,    36,    38,
      39,    48,    48,    48,    48,    49,    48,    48,    50,    48,
      48,    48,    48,    48,    48,    42,     3,    48,    48,    48,
      48,    48,    48,    48,    48,    48,    48,    48,    48,    48,
      48,    48,    48,    42,    43,    42,    42,    43,    43,    42,
      43,    42,    42,    43,    43,    42,    48,    48,    48,    48,
      48,    48,    42,    42,    43,    42,    48,    42
};

  /* YYR1[YYN] -- Symbol number of symbol that rule YYN derives.  */
static const yytype_int8 yyr1[] =
{
       0,    44,    45,    45,    46,    46,    46,    47,    47,    47,
      47,    47,    47,    47,    48,    48,    48,    48,    48,    48,
      48,    48,    48,    48,    48,    48,    48,    48,    48,    48,
      48,    48,    48,    48,    48,    48,    48,    48,    48,    48,
      48,    48,    48,    48,    48,    48,    49,    49,    50,    50
};

  /* YYR2[YYN] -- Number of symbols on the right hand side of rule YYN.  */
static const yytype_int8 yyr2[] =
{
       0,     2,     1,     2,     2,     3,     2,     0,     1,     3,
       1,     1,     1,     1,     3,     3,     3,     3,     3,     3,
       3,     3,     3,     2,     2,     3,     3,     3,     3,     3,
       3,     3,     3,     4,     6,     4,     4,     6,     4,     4,
       4,     8,     6,     4,     1,     1,     1,     3,     1,     3
};


enum { YYENOMEM = -2 };

#define yyerrok         (yyerrstatus = 0)
#define yyclearin       (yychar = YYEMPTY)

#define YYACCEPT        goto yyacceptlab
#define YYABORT         goto yyabortlab
#define YYERROR         goto yyerrorlab


#define YYRECOVERING()  (!!yyerrstatus)

#define YYBACKUP(Token, Value)                                    \
  do                                                              \
    if (yychar == YYEMPTY)                                        \
      {                                                           \
        yychar = (Token);                                         \
        yylval = (Value);                                         \
        YYPOPSTACK (yylen);                                       \
        yystate = *yyssp;                                         \
        goto yybackup;                                            \
      }                                                           \
    else                                                          \
      {                                                           \
        yyerror (YY_("syntax error: cannot back up")); \
        YYERROR;                                                  \
      }                                                           \
  while (0)

/* Backward compatibility with an undocumented macro.
   Use YYerror or YYUNDEF. */
#define YYERRCODE YYUNDEF


/* Enable debugging if requested.  */
#if YYDEBUG

# ifndef YYFPRINTF
#  include <stdio.h> /* INFRINGES ON USER NAME SPACE */
#  define YYFPRINTF fprintf
# endif

# define YYDPRINTF(Args)                        \
do {                                            \
  if (yydebug)                                  \
    YYFPRINTF Args;                             \
} while (0)

/* This macro is provided for backward compatibility. */
# ifndef YY_LOCATION_PRINT
#  define YY_LOCATION_PRINT(File, Loc) ((void) 0)
# endif


# define YY_SYMBOL_PRINT(Title, Kind, Value, Location)                    \
do {                                                                      \
  if (yydebug)                                                            \
    {                                                                     \
      YYFPRINTF (stderr, "%s ", Title);                                   \
      yy_symbol_print (stderr,                                            \
                  Kind, Value); \
      YYFPRINTF (stderr, "\n");                                           \
    }                                                                     \
} while (0)


/*-----------------------------------.
| Print this symbol's value on YYO.  |
`-----------------------------------*/

static void
yy_symbol_value_print (FILE *yyo,
                       yysymbol_kind_t yykind, YYSTYPE const * const yyvaluep)
{
  FILE *yyoutput = yyo;
  YYUSE (yyoutput);
  if (!yyvaluep)
    return;
# ifdef YYPRINT
  if (yykind < YYNTOKENS)
    YYPRINT (yyo, yytoknum[yykind], *yyvaluep);
# endif
  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  YYUSE (yykind);
  YY_IGNORE_MAYBE_UNINITIALIZED_END
}


/*---------------------------.
| Print this symbol on YYO.  |
`---------------------------*/

static void
yy_symbol_print (FILE *yyo,
                 yysymbol_kind_t yykind, YYSTYPE const * const yyvaluep)
{
  YYFPRINTF (yyo, "%s %s (",
             yykind < YYNTOKENS ? "token" : "nterm", yysymbol_name (yykind));

  yy_symbol_value_print (yyo, yykind, yyvaluep);
  YYFPRINTF (yyo, ")");
}

/*------------------------------------------------------------------.
| yy_stack_print -- Print the state stack from its BOTTOM up to its |
| TOP (included).                                                   |
`------------------------------------------------------------------*/

static void
yy_stack_print (yy_state_t *yybottom, yy_state_t *yytop)
{
  YYFPRINTF (stderr, "Stack now");
  for (; yybottom <= yytop; yybottom++)
    {
      int yybot = *yybottom;
      YYFPRINTF (stderr, " %d", yybot);
    }
  YYFPRINTF (stderr, "\n");
}

# define YY_STACK_PRINT(Bottom, Top)                            \
do {                                                            \
  if (yydebug)                                                  \
    yy_stack_print ((Bottom), (Top));                           \
} while (0)


/*------------------------------------------------.
| Report that the YYRULE is going to be reduced.  |
`------------------------------------------------*/

static void
yy_reduce_print (yy_state_t *yyssp, YYSTYPE *yyvsp,
                 int yyrule)
{
  int yylno = yyrline[yyrule];
  int yynrhs = yyr2[yyrule];
  int yyi;
  YYFPRINTF (stderr, "Reducing stack by rule %d (line %d):\n",
             yyrule - 1, yylno);
  /* The symbols being reduced.  */
  for (yyi = 0; yyi < yynrhs; yyi++)
    {
      YYFPRINTF (stderr, "   $%d = ", yyi + 1);
      yy_symbol_print (stderr,
                       YY_ACCESSING_SYMBOL (+yyssp[yyi + 1 - yynrhs]),
                       &yyvsp[(yyi + 1) - (yynrhs)]);
      YYFPRINTF (stderr, "\n");
    }
}

# define YY_REDUCE_PRINT(Rule)          \
do {                                    \
  if (yydebug)                          \
    yy_reduce_print (yyssp, yyvsp, Rule); \
} while (0)

/* Nonzero means print parse trace.  It is left uninitialized so that
   multiple parsers can coexist.  */
int yydebug;
#else /* !YYDEBUG */
# define YYDPRINTF(Args) ((void) 0)
# define YY_SYMBOL_PRINT(Title, Kind, Value, Location)
# define YY_STACK_PRINT(Bottom, Top)
# define YY_REDUCE_PRINT(Rule)
#endif /* !YYDEBUG */


/* YYINITDEPTH -- initial size of the parser's stacks.  */
#ifndef YYINITDEPTH
# define YYINITDEPTH 200
#endif

/* YYMAXDEPTH -- maximum size the stacks can grow to (effective only
   if the built-in stack extension method is used).

   Do not make this value too large; the results are undefined if
   YYSTACK_ALLOC_MAXIMUM < YYSTACK_BYTES (YYMAXDEPTH)
   evaluated with infinite-precision integer arithmetic.  */

#ifndef YYMAXDEPTH
# define YYMAXDEPTH 10000
#endif






/*-----------------------------------------------.
| Release the memory associated to this symbol.  |
`-----------------------------------------------*/

static void
yydestruct (const char *yymsg,
            yysymbol_kind_t yykind, YYSTYPE *yyvaluep)
{
  YYUSE (yyvaluep);
  if (!yymsg)
    yymsg = "Deleting";
  YY_SYMBOL_PRINT (yymsg, yykind, yyvaluep, yylocationp);

  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  YYUSE (yykind);
  YY_IGNORE_MAYBE_UNINITIALIZED_END
}


/* The lookahead symbol.  */
int yychar;

/* The semantic value of the lookahead symbol.  */
YYSTYPE yylval;
/* Number of syntax errors so far.  */
int yynerrs;




/*----------.
| yyparse.  |
`----------*/

int
yyparse (void)
{
    yy_state_fast_t yystate;
    /* Number of tokens to shift before error messages enabled.  */
    int yyerrstatus;

    /* The stacks and their tools:
       'yyss': related to states.
       'yyvs': related to semantic values.

       Refer to the stacks through separate pointers, to allow yyoverflow
       to reallocate them elsewhere.  */

    /* Their size.  */
    YYPTRDIFF_T yystacksize;

    /* The state stack.  */
    yy_state_t yyssa[YYINITDEPTH];
    yy_state_t *yyss;
    yy_state_t *yyssp;

    /* The semantic value stack.  */
    YYSTYPE yyvsa[YYINITDEPTH];
    YYSTYPE *yyvs;
    YYSTYPE *yyvsp;

  int yyn;
  /* The return value of yyparse.  */
  int yyresult;
  /* Lookahead token as an internal (translated) token number.  */
  yysymbol_kind_t yytoken = YYSYMBOL_YYEMPTY;
  /* The variables used to return semantic value and location from the
     action routines.  */
  YYSTYPE yyval;



#define YYPOPSTACK(N)   (yyvsp -= (N), yyssp -= (N))

  /* The number of symbols on the RHS of the reduced rule.
     Keep to zero when no symbol should be popped.  */
  int yylen = 0;

  yynerrs = 0;
  yystate = 0;
  yyerrstatus = 0;

  yystacksize = YYINITDEPTH;
  yyssp = yyss = yyssa;
  yyvsp = yyvs = yyvsa;


  YYDPRINTF ((stderr, "Starting parse\n"));

  yychar = YYEMPTY; /* Cause a token to be read.  */
  goto yysetstate;


/*------------------------------------------------------------.
| yynewstate -- push a new state, which is found in yystate.  |
`------------------------------------------------------------*/
yynewstate:
  /* In all cases, when you get here, the value and location stacks
     have just been pushed.  So pushing a state here evens the stacks.  */
  yyssp++;


/*--------------------------------------------------------------------.
| yysetstate -- set current state (the top of the stack) to yystate.  |
`--------------------------------------------------------------------*/
yysetstate:
  YYDPRINTF ((stderr, "Entering state %d\n", yystate));
  YY_ASSERT (0 <= yystate && yystate < YYNSTATES);
  YY_IGNORE_USELESS_CAST_BEGIN
  *yyssp = YY_CAST (yy_state_t, yystate);
  YY_IGNORE_USELESS_CAST_END
  YY_STACK_PRINT (yyss, yyssp);

  if (yyss + yystacksize - 1 <= yyssp)
#if !defined yyoverflow && !defined YYSTACK_RELOCATE
    goto yyexhaustedlab;
#else
    {
      /* Get the current used size of the three stacks, in elements.  */
      YYPTRDIFF_T yysize = yyssp - yyss + 1;

# if defined yyoverflow
      {
        /* Give user a chance to reallocate the stack.  Use copies of
           these so that the &'s don't force the real ones into
           memory.  */
        yy_state_t *yyss1 = yyss;
        YYSTYPE *yyvs1 = yyvs;

        /* Each stack pointer address is followed by the size of the
           data in use in that stack, in bytes.  This used to be a
           conditional around just the two extra args, but that might
           be undefined if yyoverflow is a macro.  */
        yyoverflow (YY_("memory exhausted"),
                    &yyss1, yysize * YYSIZEOF (*yyssp),
                    &yyvs1, yysize * YYSIZEOF (*yyvsp),
                    &yystacksize);
        yyss = yyss1;
        yyvs = yyvs1;
      }
# else /* defined YYSTACK_RELOCATE */
      /* Extend the stack our own way.  */
      if (YYMAXDEPTH <= yystacksize)
        goto yyexhaustedlab;
      yystacksize *= 2;
      if (YYMAXDEPTH < yystacksize)
        yystacksize = YYMAXDEPTH;

      {
        yy_state_t *yyss1 = yyss;
        union yyalloc *yyptr =
          YY_CAST (union yyalloc *,
                   YYSTACK_ALLOC (YY_CAST (YYSIZE_T, YYSTACK_BYTES (yystacksize))));
        if (! yyptr)
          goto yyexhaustedlab;
        YYSTACK_RELOCATE (yyss_alloc, yyss);
        YYSTACK_RELOCATE (yyvs_alloc, yyvs);
#  undef YYSTACK_RELOCATE
        if (yyss1 != yyssa)
          YYSTACK_FREE (yyss1);
      }
# endif

      yyssp = yyss + yysize - 1;
      yyvsp = yyvs + yysize - 1;

      YY_IGNORE_USELESS_CAST_BEGIN
      YYDPRINTF ((stderr, "Stack size increased to %ld\n",
                  YY_CAST (long, yystacksize)));
      YY_IGNORE_USELESS_CAST_END

      if (yyss + yystacksize - 1 <= yyssp)
        YYABORT;
    }
#endif /* !defined yyoverflow && !defined YYSTACK_RELOCATE */

  if (yystate == YYFINAL)
    YYACCEPT;

  goto yybackup;


/*-----------.
| yybackup.  |
`-----------*/
yybackup:
  /* Do appropriate processing given the current state.  Read a
     lookahead token if we need one and don't already have one.  */

  /* First try to decide what to do without reference to lookahead token.  */
  yyn = yypact[yystate];
  if (yypact_value_is_default (yyn))
    goto yydefault;

  /* Not known => get a lookahead token if don't already have one.  */

  /* YYCHAR is either empty, or end-of-input, or a valid lookahead.  */
  if (yychar == YYEMPTY)
    {
      YYDPRINTF ((stderr, "Reading a token\n"));
      yychar = yylex ();
    }

  if (yychar <= YYEOF)
    {
      yychar = YYEOF;
      yytoken = YYSYMBOL_YYEOF;
      YYDPRINTF ((stderr, "Now at end of input.\n"));
    }
  else if (yychar == YYerror)
    {
      /* The scanner already issued an error message, process directly
         to error recovery.  But do not keep the error token as
         lookahead, it is too special and may lead us to an endless
         loop in error recovery. */
      yychar = YYUNDEF;
      yytoken = YYSYMBOL_YYerror;
      goto yyerrlab1;
    }
  else
    {
      yytoken = YYTRANSLATE (yychar);
      YY_SYMBOL_PRINT ("Next token is", yytoken, &yylval, &yylloc);
    }

  /* If the proper action on seeing token YYTOKEN is to reduce or to
     detect an error, take that action.  */
  yyn += yytoken;
  if (yyn < 0 || YYLAST < yyn || yycheck[yyn] != yytoken)
    goto yydefault;
  yyn = yytable[yyn];
  if (yyn <= 0)
    {
      if (yytable_value_is_error (yyn))
        goto yyerrlab;
      yyn = -yyn;
      goto yyreduce;
    }

  /* Count tokens shifted since error; after three, turn off error
     status.  */
  if (yyerrstatus)
    yyerrstatus--;

  /* Shift the lookahead token.  */
  YY_SYMBOL_PRINT ("Shifting", yytoken, &yylval, &yylloc);
  yystate = yyn;
  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  *++yyvsp = yylval;
  YY_IGNORE_MAYBE_UNINITIALIZED_END

  /* Discard the shifted token.  */
  yychar = YYEMPTY;
  goto yynewstate;


/*-----------------------------------------------------------.
| yydefault -- do the default action for the current state.  |
`-----------------------------------------------------------*/
yydefault:
  yyn = yydefact[yystate];
  if (yyn == 0)
    goto yyerrlab;
  goto yyreduce;


/*-----------------------------.
| yyreduce -- do a reduction.  |
`-----------------------------*/
yyreduce:
  /* yyn is the number of a rule to reduce with.  */
  yylen = yyr2[yyn];

  /* If YYLEN is nonzero, implement the default value of the action:
     '$$ = $1'.

     Otherwise, the following line sets YYVAL to garbage.
     This behavior is undocumented and Bison
     users should not rely upon it.  Assigning to YYVAL
     unconditionally makes the parser a bit smaller, and it avoids a
     GCC warning that YYVAL may be used uninitialized.  */
  yyval = yyvsp[1-yylen];


  YY_REDUCE_PRINT (yyn);
  switch (yyn)
    {
  case 6:
#line 173 "../../../gmp/demos/calc/calc.y"
              { sp = stack[0]; yyerrok; }
#line 1516 "calc.c"
    break;

  case 8:
#line 177 "../../../gmp/demos/calc/calc.y"
      {
      mpz_out_str (stdout, obase, sp); putchar ('\n');
      sp--;
      CHECK_EMPTY ();
    }
#line 1526 "calc.c"
    break;

  case 9:
#line 182 "../../../gmp/demos/calc/calc.y"
                   {
      CHECK_VARIABLE ((yyvsp[-2].var));
      mpz_swap (variable[(yyvsp[-2].var)], sp);
      sp--;
      CHECK_EMPTY ();
    }
#line 1537 "calc.c"
    break;

  case 10:
#line 188 "../../../gmp/demos/calc/calc.y"
            { calc_help (); }
#line 1543 "calc.c"
    break;

  case 11:
#line 189 "../../../gmp/demos/calc/calc.y"
            { ibase = 16; obase = -16; }
#line 1549 "calc.c"
    break;

  case 12:
#line 190 "../../../gmp/demos/calc/calc.y"
            { ibase = 0;  obase = 10; }
#line 1555 "calc.c"
    break;

  case 13:
#line 191 "../../../gmp/demos/calc/calc.y"
            { exit (0); }
#line 1561 "calc.c"
    break;

  case 15:
#line 198 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_add    (sp, sp, sp+1); }
#line 1567 "calc.c"
    break;

  case 16:
#line 199 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_sub    (sp, sp, sp+1); }
#line 1573 "calc.c"
    break;

  case 17:
#line 200 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_mul    (sp, sp, sp+1); }
#line 1579 "calc.c"
    break;

  case 18:
#line 201 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_fdiv_q (sp, sp, sp+1); }
#line 1585 "calc.c"
    break;

  case 19:
#line 202 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_fdiv_r (sp, sp, sp+1); }
#line 1591 "calc.c"
    break;

  case 20:
#line 203 "../../../gmp/demos/calc/calc.y"
                  { CHECK_UI ("Exponent", sp);
                    sp--; mpz_pow_ui (sp, sp, mpz_get_ui (sp+1)); }
#line 1598 "calc.c"
    break;

  case 21:
#line 205 "../../../gmp/demos/calc/calc.y"
                  { CHECK_UI ("Shift count", sp);
                    sp--; mpz_mul_2exp (sp, sp, mpz_get_ui (sp+1)); }
#line 1605 "calc.c"
    break;

  case 22:
#line 207 "../../../gmp/demos/calc/calc.y"
                  { CHECK_UI ("Shift count", sp);
                    sp--; mpz_fdiv_q_2exp (sp, sp, mpz_get_ui (sp+1)); }
#line 1612 "calc.c"
    break;

  case 23:
#line 209 "../../../gmp/demos/calc/calc.y"
                  { CHECK_UI ("Factorial", sp);
                    mpz_fac_ui (sp, mpz_get_ui (sp)); }
#line 1619 "calc.c"
    break;

  case 24:
#line 211 "../../../gmp/demos/calc/calc.y"
                           { mpz_neg (sp, sp); }
#line 1625 "calc.c"
    break;

  case 25:
#line 213 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) <  0); }
#line 1631 "calc.c"
    break;

  case 26:
#line 214 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) <= 0); }
#line 1637 "calc.c"
    break;

  case 27:
#line 215 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) == 0); }
#line 1643 "calc.c"
    break;

  case 28:
#line 216 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) != 0); }
#line 1649 "calc.c"
    break;

  case 29:
#line 217 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) >= 0); }
#line 1655 "calc.c"
    break;

  case 30:
#line 218 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_cmp (sp, sp+1) >  0); }
#line 1661 "calc.c"
    break;

  case 31:
#line 220 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_sgn (sp) && mpz_sgn (sp+1)); }
#line 1667 "calc.c"
    break;

  case 32:
#line 221 "../../../gmp/demos/calc/calc.y"
                  { sp--; mpz_set_ui (sp, mpz_sgn (sp) || mpz_sgn (sp+1)); }
#line 1673 "calc.c"
    break;

  case 33:
#line 223 "../../../gmp/demos/calc/calc.y"
                                 { mpz_abs (sp, sp); }
#line 1679 "calc.c"
    break;

  case 34:
#line 224 "../../../gmp/demos/calc/calc.y"
                                 { sp--; CHECK_UI ("Binomial base", sp+1);
                                   mpz_bin_ui (sp, sp, mpz_get_ui (sp+1)); }
#line 1686 "calc.c"
    break;

  case 35:
#line 226 "../../../gmp/demos/calc/calc.y"
                                 { CHECK_UI ("Fibonacci", sp);
                                   mpz_fib_ui (sp, mpz_get_ui (sp)); }
#line 1693 "calc.c"
    break;

  case 37:
#line 229 "../../../gmp/demos/calc/calc.y"
                                 { sp--; mpz_set_si (sp,
                                         mpz_kronecker (sp, sp+1)); }
#line 1700 "calc.c"
    break;

  case 39:
#line 232 "../../../gmp/demos/calc/calc.y"
                                 { CHECK_UI ("Lucas number", sp);
                                   mpz_lucnum_ui (sp, mpz_get_ui (sp)); }
#line 1707 "calc.c"
    break;

  case 40:
#line 234 "../../../gmp/demos/calc/calc.y"
                                 { mpz_nextprime (sp, sp); }
#line 1713 "calc.c"
    break;

  case 41:
#line 235 "../../../gmp/demos/calc/calc.y"
                                 { sp -= 2; mpz_powm (sp, sp, sp+1, sp+2); }
#line 1719 "calc.c"
    break;

  case 42:
#line 236 "../../../gmp/demos/calc/calc.y"
                                 { sp--; CHECK_UI ("Nth-root", sp+1);
                                   mpz_root (sp, sp, mpz_get_ui (sp+1)); }
#line 1726 "calc.c"
    break;

  case 43:
#line 238 "../../../gmp/demos/calc/calc.y"
                                 { mpz_sqrt (sp, sp); }
#line 1732 "calc.c"
    break;

  case 44:
#line 240 "../../../gmp/demos/calc/calc.y"
               {
        sp++;
        CHECK_OVERFLOW ();
        CHECK_VARIABLE ((yyvsp[0].var));
        mpz_set (sp, variable[(yyvsp[0].var)]);
      }
#line 1743 "calc.c"
    break;

  case 45:
#line 246 "../../../gmp/demos/calc/calc.y"
             {
        sp++;
        CHECK_OVERFLOW ();
        if (mpz_set_str (sp, (yyvsp[0].str), ibase) != 0)
          {
            fprintf (stderr, "Invalid number: %s\n", (yyvsp[0].str));
            YYERROR;
          }
      }
#line 1757 "calc.c"
    break;

  case 47:
#line 258 "../../../gmp/demos/calc/calc.y"
                     { sp--; mpz_gcd (sp, sp, sp+1); }
#line 1763 "calc.c"
    break;

  case 49:
#line 262 "../../../gmp/demos/calc/calc.y"
                     { sp--; mpz_lcm (sp, sp, sp+1); }
#line 1769 "calc.c"
    break;


#line 1773 "calc.c"

      default: break;
    }
  /* User semantic actions sometimes alter yychar, and that requires
     that yytoken be updated with the new translation.  We take the
     approach of translating immediately before every use of yytoken.
     One alternative is translating here after every semantic action,
     but that translation would be missed if the semantic action invokes
     YYABORT, YYACCEPT, or YYERROR immediately after altering yychar or
     if it invokes YYBACKUP.  In the case of YYABORT or YYACCEPT, an
     incorrect destructor might then be invoked immediately.  In the
     case of YYERROR or YYBACKUP, subsequent parser actions might lead
     to an incorrect destructor call or verbose syntax error message
     before the lookahead is translated.  */
  YY_SYMBOL_PRINT ("-> $$ =", YY_CAST (yysymbol_kind_t, yyr1[yyn]), &yyval, &yyloc);

  YYPOPSTACK (yylen);
  yylen = 0;

  *++yyvsp = yyval;

  /* Now 'shift' the result of the reduction.  Determine what state
     that goes to, based on the state we popped back to and the rule
     number reduced by.  */
  {
    const int yylhs = yyr1[yyn] - YYNTOKENS;
    const int yyi = yypgoto[yylhs] + *yyssp;
    yystate = (0 <= yyi && yyi <= YYLAST && yycheck[yyi] == *yyssp
               ? yytable[yyi]
               : yydefgoto[yylhs]);
  }

  goto yynewstate;


/*--------------------------------------.
| yyerrlab -- here on detecting error.  |
`--------------------------------------*/
yyerrlab:
  /* Make sure we have latest lookahead translation.  See comments at
     user semantic actions for why this is necessary.  */
  yytoken = yychar == YYEMPTY ? YYSYMBOL_YYEMPTY : YYTRANSLATE (yychar);
  /* If not already recovering from an error, report this error.  */
  if (!yyerrstatus)
    {
      ++yynerrs;
      yyerror (YY_("syntax error"));
    }

  if (yyerrstatus == 3)
    {
      /* If just tried and failed to reuse lookahead token after an
         error, discard it.  */

      if (yychar <= YYEOF)
        {
          /* Return failure if at end of input.  */
          if (yychar == YYEOF)
            YYABORT;
        }
      else
        {
          yydestruct ("Error: discarding",
                      yytoken, &yylval);
          yychar = YYEMPTY;
        }
    }

  /* Else will try to reuse lookahead token after shifting the error
     token.  */
  goto yyerrlab1;


/*---------------------------------------------------.
| yyerrorlab -- error raised explicitly by YYERROR.  |
`---------------------------------------------------*/
yyerrorlab:
  /* Pacify compilers when the user code never invokes YYERROR and the
     label yyerrorlab therefore never appears in user code.  */
  if (0)
    YYERROR;

  /* Do not reclaim the symbols of the rule whose action triggered
     this YYERROR.  */
  YYPOPSTACK (yylen);
  yylen = 0;
  YY_STACK_PRINT (yyss, yyssp);
  yystate = *yyssp;
  goto yyerrlab1;


/*-------------------------------------------------------------.
| yyerrlab1 -- common code for both syntax error and YYERROR.  |
`-------------------------------------------------------------*/
yyerrlab1:
  yyerrstatus = 3;      /* Each real token shifted decrements this.  */

  /* Pop stack until we find a state that shifts the error token.  */
  for (;;)
    {
      yyn = yypact[yystate];
      if (!yypact_value_is_default (yyn))
        {
          yyn += YYSYMBOL_YYerror;
          if (0 <= yyn && yyn <= YYLAST && yycheck[yyn] == YYSYMBOL_YYerror)
            {
              yyn = yytable[yyn];
              if (0 < yyn)
                break;
            }
        }

      /* Pop the current state because it cannot handle the error token.  */
      if (yyssp == yyss)
        YYABORT;


      yydestruct ("Error: popping",
                  YY_ACCESSING_SYMBOL (yystate), yyvsp);
      YYPOPSTACK (1);
      yystate = *yyssp;
      YY_STACK_PRINT (yyss, yyssp);
    }

  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  *++yyvsp = yylval;
  YY_IGNORE_MAYBE_UNINITIALIZED_END


  /* Shift the error token.  */
  YY_SYMBOL_PRINT ("Shifting", YY_ACCESSING_SYMBOL (yyn), yyvsp, yylsp);

  yystate = yyn;
  goto yynewstate;


/*-------------------------------------.
| yyacceptlab -- YYACCEPT comes here.  |
`-------------------------------------*/
yyacceptlab:
  yyresult = 0;
  goto yyreturn;


/*-----------------------------------.
| yyabortlab -- YYABORT comes here.  |
`-----------------------------------*/
yyabortlab:
  yyresult = 1;
  goto yyreturn;


#if !defined yyoverflow
/*-------------------------------------------------.
| yyexhaustedlab -- memory exhaustion comes here.  |
`-------------------------------------------------*/
yyexhaustedlab:
  yyerror (YY_("memory exhausted"));
  yyresult = 2;
  /* Fall through.  */
#endif


/*-----------------------------------------------------.
| yyreturn -- parsing is finished, return the result.  |
`-----------------------------------------------------*/
yyreturn:
  if (yychar != YYEMPTY)
    {
      /* Make sure we have latest lookahead translation.  See comments at
         user semantic actions for why this is necessary.  */
      yytoken = YYTRANSLATE (yychar);
      yydestruct ("Cleanup: discarding lookahead",
                  yytoken, &yylval);
    }
  /* Do not reclaim the symbols of the rule whose action triggered
     this YYABORT or YYACCEPT.  */
  YYPOPSTACK (yylen);
  YY_STACK_PRINT (yyss, yyssp);
  while (yyssp != yyss)
    {
      yydestruct ("Cleanup: popping",
                  YY_ACCESSING_SYMBOL (+*yyssp), yyvsp);
      YYPOPSTACK (1);
    }
#ifndef yyoverflow
  if (yyss != yyssa)
    YYSTACK_FREE (yyss);
#endif

  return yyresult;
}

#line 264 "../../../gmp/demos/calc/calc.y"


yyerror (char *s)
{
  fprintf (stderr, "%s\n", s);
}

int calc_option_readline = -1;

int
main (int argc, char *argv[])
{
  int  i;

  for (i = 1; i < argc; i++)
    {
      if (strcmp (argv[i], "--readline") == 0)
        calc_option_readline = 1;
      else if (strcmp (argv[i], "--noreadline") == 0)
        calc_option_readline = 0;
      else if (strcmp (argv[i], "--help") == 0)
        {
          printf ("Usage: calc [--option]...\n");
          printf ("  --readline    use readline\n");
          printf ("  --noreadline  don't use readline\n");
          printf ("  --help        this message\n");
          printf ("Readline is only available when compiled in,\n");
          printf ("and in that case it's the default on a tty.\n");
          exit (0);
        }
      else
        {
          fprintf (stderr, "Unrecognised option: %s\n", argv[i]);
          exit (1);
        }
    }

#if WITH_READLINE
  calc_init_readline ();
#else
  if (calc_option_readline == 1)
    {
      fprintf (stderr, "Readline support not available\n");
      exit (1);
    }
#endif

  for (i = 0; i < numberof (variable); i++)
    mpz_init (variable[i]);

  for (i = 0; i < numberof (stack); i++)
    mpz_init (stack[i]);

  return yyparse ();
}
