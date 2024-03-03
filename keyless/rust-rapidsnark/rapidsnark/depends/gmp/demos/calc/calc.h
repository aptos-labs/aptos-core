/* A Bison parser, made by GNU Bison 3.6.4.  */

/* Bison interface for Yacc-like parsers in C

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

/* DO NOT RELY ON FEATURES THAT ARE NOT DOCUMENTED in the manual,
   especially those whose name start with YY_ or yy_.  They are
   private implementation details that can be changed or removed.  */

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

#line 131 "calc.h"

};
typedef union YYSTYPE YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif


extern YYSTYPE yylval;

int yyparse (void);

#endif /* !YY_YY_CALC_H_INCLUDED  */
