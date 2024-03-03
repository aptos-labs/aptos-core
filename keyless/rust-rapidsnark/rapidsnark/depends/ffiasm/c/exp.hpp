#include <stdint.h>
#include <iostream>

#include "naf.hpp"

template <typename BaseGroup, typename BaseGroupElementIn, typename BaseGroupElementOut>
void nafMulByScalar(BaseGroup &G, BaseGroupElementOut& r, BaseGroupElementIn& base, uint8_t* scalar, unsigned int scalarSize) {
    BaseGroupElementIn baseCopy;
    int nBits = (scalarSize*8)+2;
    uint8_t *naf = new uint8_t[(scalarSize+2)*8];
    buildNaf(naf, scalar, scalarSize);

    G.copy(baseCopy, base); // base and result can be the same
    G.copy(r, G.zero());
    int i = nBits-1;
    while ((i>=0)&&(naf[i] == 0)) i--;
    while (i>=0) {
        G.dbl(r, r);
        if (naf[i] == 1) {
            G.add(r, r, baseCopy);
        } else if (naf[i] == 2) {
            G.sub(r, r, baseCopy);
        }
        i--;
    }

    delete[] naf;
}

