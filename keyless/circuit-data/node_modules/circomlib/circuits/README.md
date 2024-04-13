# CircomLib/Circuits

## Description

- This folder contains circuit templates for standard operations and many cryptographic primitives.
- Below you can find specifications of each function. In the representation of elements, there are three tyes:
    - Binary
    - String
    - Field element (the field is specified in each case. We consider 2 possible fields: Fp and Fr, where p... and r... .)

## Table of Contents

[TOC]

## Jordi

* compconstant - Returns 1 if `in` (expanded to binary array) > `ct`
* aliascheck - check if `in` (expanded to binary array) oveflowed its 254 bits (<= -1)
* babyjub - twisted Edwards curve 168700.x^2 + y^2 = 1 + 168696.x^2.y^2
  * BabyAdd - (`xout`,`yout`) = (`x1`,`y1`) + (`x2`,`y2`)
  * BabyDbl - (`xout`,`yout`) = 2*(`x`,`y`)
  * BabyCheck - check that (`x`,`y`) is on the curve
* binsub - binary subtraction
* gates - logical gates
* mimc - SNARK-friendly hash Minimal Multiplicative Complexity.
  * https://eprint.iacr.org/2016/492.pdf
  * zcash/zcash#2233
* smt - Sparse Merkle Tree
  * https://ethresear.ch/t/optimizing-sparse-merkle-trees/3751
* montgomery https://en.wikipedia.org/wiki/Montgomery_curve

## Circuits

### sha256

Folder containing the implementation of sha256 hash circuit.

### smt

Folder containing the circuit implementation of Sparse Merkle Trees.

### aliascheck

- `AliasCheck()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### babyjub

Arithmetic on [Baby Jubjub elliptic curve](https://github.com/barryWhiteHat/baby_jubjub) in twisted Edwards form. (TODO: Expose here the characteristics of the curve?)


- `BabyAdd()`

    - DESCRIPTION

      It adds two points on the Baby Jubjub curve. More specifically, given two points P1 = (`x1`, `y1`) and P2 = (`x2`, `y2`) it returns a point P3 = (`xout`, `yout`)  such that

        (`xout`, `yout`) =  (`x1`,`y1`) + (`x2`,`y2`)
             = ((`x1y2`+`y1x2`)/(1+`dx1x2y1y2`)),(`y1y2`-`ax1x2`)/(1-`dx1x2y1y2`))

    - SCHEMA
       ```
                                        var a     var d
                                          |         |
                                          |         |
                                    ______v_________v_______
                   input x1 ---->  |                        |
                   input y1 ---->  |        BabyAdd()       | ----> output xout
                   input x2 ---->  |                        | ----> output yout
                   input y2 ---->  |________________________|
       ```

    - INPUTS

      | Input         | Representation | Description         |                                             |
      | ------------- | -------------  | -------------       | -------------                               |
      | `x1`          | Bigint         | Field element of Fp | First coordinate of a point (x1, y1) on E.  |
      | `y1`          | Bigint         | Field element of Fp | Second coordinate of a point (x1, y1) on E. |
      | `x2`          | Bigint         | Field element of Fp | First coordinate of a point (x2, y2) on E.  |
      | `y2`          | Bigint         | Field element of Fp | Second coordinate of a point (x2, y2) on E. |

      Requirement: at least `x1`!=`x2` or `y1`!=`y2`.

    - OUTPUT

      | Input         | Representation | Description         |                                             |
      | ------------- | -------------  | -------------       | -------------                               |
      | `xout`          | Bigint         | Field element of Fp | First coordinate of the addition point (xout, yout) = (x1, y1) + (x2, y2).  |
      | `yout`          | Bigint         | Field element of Fp | Second coordinate of the addition point (xout, yout) = (x1, y1) + (x2, y2). |

    - BENCHMARKS (constraints)

    - EXAMPLE

- `BabyDbl()`
    - DESCRIPTION : doubles a point (`xout`,`yout`) = 2*(`x`,`y`).
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `BabyCheck()`

    - DESCRIPTION : checks if a given point is in the curve.
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `BabyPbk()`

    - DESCRIPTION: : given a private key, it returns the associated public key.
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE


### binsub

- `BinSub(n)`

    - DESCRIPTION: binary substraction.
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### binsum

- `nbits(a)`

    - DESCRIPTION : binary sum.
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `BinSum(n, ops)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### bitify

- `Num2Bits()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Num2Bits_strict()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Bits2Num()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Bits2Num_strict()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Num2BitsNeg()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### comparators

- `IsZero() `

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `IsEqual()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `ForceEqualIfEnabled()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `LessThan()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `GreaterThan()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `GreaterEqThan()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### compconstant

- `CompConstant(ct)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### eddsa

Edwards Digital Signature Algorithm in Baby Jubjbub (link a eddsa)

- `EdDSAVerifier(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### eddsamimc

- `EdDSAMiMCVerifier()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### eddsamimcsponge

- `EdDSAMiMCSpongeVerifier()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### eddsaposeidon

- `EdDSAPoseidonVerifier()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### escalarmul

- `EscalarMulWindow(base, k)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `EscalarMul(n, base)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### escalarmulany

- `Multiplexor2()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `BitElementMulAny()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `SegmentMulAny(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `EscalarMulAny(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### escalarmulfix

- `WindowMulFix()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `SegmentMulFix(nWindows)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `EscalarMulFix(n, BASE)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### escalarmulw4table

- `pointAdd`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `EscalarMulW4Table`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### gates

- `XOR`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `AND`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `OR`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `NOT`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `NAND`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `NOR`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `MultiAND`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mimc

Implementation of MiMC-7 hash in Fp being...  (link to description of the hash)

- `MiMC7(nrounds)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `MultiMiMC7(nInputs, nRounds)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mimcsponge

- `MiMCSponge(nInputs, nRounds, nOutputs)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `MiMCFeistel(nrounds)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### montgomery

- `Edwards2Montgomery()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Montgomery2Edwards()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `MontgomeryAdd()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `MontgomeryDouble()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### multiplexer

- `log2(a)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `EscalarProduct(w)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Decoder(w)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Multiplexer(wIn, nIn)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mux1

- `MultiMux1(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Mux1()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mux2

- `MultiMux2(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Mux2()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mux3

- `MultiMux3(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Mux3()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### mux4

- `MultiMux4(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Mux4()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### pedersen_old

Old version of the Pedersen hash (do not use any
more?).

### pedersen

- `Window4()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Segment(nWindows)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Pedersen(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### pointbits

- `sqrt(n)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Bits2Point()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Bits2Point_Strict()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Point2Bits`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Point2Bits_Strict`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### poseidon

Implementation of Poseidon hash function (LINK)

- `Sigma()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Ark(t, C, r)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Mix(t, M)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

- `Poseidon(nInputs)`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### sign

- `Sign()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE

### switcher

- `Switcher()`

    - DESCRIPTION
    - SCHEMA
    - INPUT
    - OUTPUT
    - BENCHMARKS
    - EXAMPLE
