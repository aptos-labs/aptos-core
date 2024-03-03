#ifndef FR_ELEMENT_HPP
#define FR_ELEMENT_HPP

#include <cstdint>

#define Fr_N64 4
#define Fr_SHORT           0x00000000
#define Fr_MONTGOMERY      0x40000000
#define Fr_SHORTMONTGOMERY 0x40000000
#define Fr_LONG            0x80000000
#define Fr_LONGMONTGOMERY  0xC0000000

typedef uint64_t FrRawElement[Fr_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    FrRawElement longVal;
} FrElement;

typedef FrElement *PFrElement;

#endif // FR_ELEMENT_HPP
