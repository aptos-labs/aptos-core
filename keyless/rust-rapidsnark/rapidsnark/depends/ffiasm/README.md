## Important note 

**This is a new implementation of ffiasm. If you need access to the original (and now obsoleted) implemenation at  https://github.com/iden3/ffiasm-old .**

# ffiasm

This package is a script that generates a Finite field Library in Intel64 / ARM64 Assembly

## Usage

install g++ nasm ang gmp library if you don't have it.

```
npm install -g ffiasm
mkdir myProject
cd myProject
buildzqfield -q 21888242871839275222246405745257275088548364400416034343698204186575808495617 -n Fr
```

You now will have two files fr.cpp, fr.hpp and fr.asm

```
ls
```

If you are in linux:

```
nasm -felf64 fr.asm
```

If you are in a mac:

```
nasm -fmacho64 --prefix _ fr.asm
```

Create a file named main.cpp to use the library

```C
#include <stdio.h>
#include <stdlib.h>
#include "fr.hpp"

int main() {
    Fr_init();

    FrElement a;
    a.type = Fr_SHORT;
    a.shortVal = 2;

    FrElement b;
    b.type = Fr_SHORT;
    b.shortVal = 6;

    FrElement c;

    Fr_mul(&c, &a, &b);

    char *c1 = Fr_element2str(&c);
    printf("Result: %s\n", c1);
    free(c1);
}
```

Compile it

```
g++ main.cpp fr.o fr.cpp -o example -lgmp
```

Run it
```
./example
```

# Benchmark

```
npm run benchmark
```

## License

ffiasm is part of the iden3 project copyright 2020 0KIMS association and published with GPL-3 license. Please check the COPYING file for more details.

