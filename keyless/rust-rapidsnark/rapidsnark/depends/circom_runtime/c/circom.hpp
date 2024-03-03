#ifndef __CIRCOM_H
#define __CIRCOM_H

#include <gmp.h>
#include <stdint.h>
#include "fr.hpp"

class Circom_CalcWit;
typedef unsigned long long u64;
typedef uint32_t u32;
typedef uint8_t u8;

typedef int Circom_Size;
typedef Circom_Size *Circom_Sizes;

struct __attribute__((__packed__)) Circom_HashEntry {
    u64 hash;
    int pos;
};
typedef Circom_HashEntry *Circom_HashTable;

typedef enum  { _typeSignal=0, _typeComponent=1} Circom_EntryType;

struct __attribute__((__packed__)) Circom_ComponentEntry {
    Circom_Sizes sizes;
    uint32_t offset;
    Circom_EntryType type;
};
typedef Circom_ComponentEntry *Circom_ComponentEntries;

typedef void (*Circom_ComponentFunction)(Circom_CalcWit *ctx, int __cIdx);

struct Circom_Component {
    Circom_HashTable hashTable;
    Circom_ComponentEntries entries;
    Circom_ComponentFunction fn;
    uint32_t inputSignals;
    uint32_t newThread;
};

struct __attribute__((__packed__)) Circom_Circuit {
    unsigned int *wit2sig;
    Circom_Component *components;
    u32 *mapIsInput;
    PFrElement constants;
    const char *P;
    Circom_ComponentEntry *componentEntries;
    int NSignals;
    int NComponents;
    int NInputs;
    int NOutputs;
    int NVars;
    int NComponentEntries;
    int NPublic;
};

#define BITMAP_ISSET(m, b) (m[b>>5] & (1 << (b&0x1F)))
extern Circom_ComponentFunction _functionTable[];
#endif
