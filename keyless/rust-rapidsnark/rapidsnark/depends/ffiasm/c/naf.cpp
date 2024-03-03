#include "naf.hpp"

static uint64_t NAFTable[1024];

bool buildNafTable() {
    for (int in=0; in<1024; in++) {
        bool carry = (in & 0x200);
        bool last = (in & 1);
        uint8_t res[8];
        for (int i=0; i<8; i++) {
            bool cur = in & (1 << (i+1));

            if (last) {
                if (cur) {
                    if (carry) {
                        last = false; carry = true;  res[i] = 1;
                    } else {
                        last = false; carry = true;  res[i] = 2; // -1
                    }
                } else {
                    if (carry) {
                        last = false; carry = true;  res[i] = 2; // -1
                    } else {
                        last = false; carry = false; res[i] = 1;
                    }
                }
            } else {
                if (cur) {
                    if (carry) {
                        last = false; carry = true;  res[i] = 0;
                    } else {
                        last = true; carry = false;  res[i] = 0;
                    }
                } else {
                    if (carry) {
                        last = true; carry = false;  res[i] = 0;
                    } else {
                        last = false; carry = false; res[i] = 0;
                    }
                }
            }
        }

        uint64_t r64 = (*((int64_t *)(res)));
        if (carry) r64 |= 0x4;
        if (last) r64 |= 0x8;

        NAFTable[in] = r64;
    }
    return true;
}



void buildNaf(uint8_t *r, uint8_t* scalar, unsigned int scalarSize) {
    int64_t *r64 = (int64_t *)r;

    bool carry = false;
    bool last = (scalar[0] & 1);
    int st;
    int64_t rs;

    for (unsigned int i=0; i<scalarSize+2; i++) {
        st = last ? 1 : 0;
        if (i<scalarSize) st += scalar[i] & 0xFE;
        if (i<scalarSize-1) st += (scalar[i+1] & 1) << 8;
        if (carry) st += 0x200;

        rs = NAFTable[st];
        carry = rs & 4;
        last = rs & 8;
        r64[i] = rs & 0x0303030303030303LL;
    }
}

static bool tableBulded = buildNafTable();
