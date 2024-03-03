#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include "fr.h"

int main() {
    Fr_init();
/*
    FrElement a = { 0, Fr_LONGMONTGOMERY, {1,1,1,1}};
    FrElement b = { 0, Fr_LONGMONTGOMERY, {2,2,2,2}};


    FrElement a={0x43e1f593f0000000ULL,0x2833e84879b97091ULL,0xb85045b68181585dULL,0x30644e72e131a029ULL};
    FrElement b = {3,0,0,0};

    FrElement c;
*/
//    Fr_add(&(c[0]), a, a);
//    Fr_add(&(c[0]), c, b);

/*
    for (int i=0; i<1000000000; i++) {
        Fr_mul(&c, &a, &b);
    }

    Fr_mul(&c,&a, &b);
*/

/*
    FrElement a1[10];
    FrElement a2[10];
    for (int i=0; i<10; i++) {
        a1[i].type = Fr_LONGMONTGOMERY;
        a1[i].shortVal =0;
        for (int j=0; j<Fr_N64; j++) {
            a2[i].longVal[j] = i;
        }
    }

    Fr_copyn(a2, a1, 10);

    for (int i=0; i<10; i++) {
        char *c1 = Fr_element2str(&a1[i]);
        char *c2 = Fr_element2str(&a2[i]);
        printf("%s\n%s\n\n", c1, c2);
        free(c1);
        free(c2);
    }
*/

    int tests[7] = { 0, 1, 2, -1, -2, 0x7FFFFFFF, (int)0x80000000};
    for (int i=0; i<7;i++) {
        FrElement a = { tests[i], Fr_SHORT, {0,0,0,0}};
        Fr_toLongNormal(&a);
        int b = Fr_toInt(&a);
        int c = Fr_isTrue(&a);
        printf("%d, %d, %d\n", tests[i], b, c);
    }

    FrElement err = { 0, Fr_LONGMONTGOMERY, {1,1,1,1}};
    Fr_toInt(&err);

    // printf("%llu, %llu, %llu, %llu\n", c.longVal[0], c.longVal[1], c.longVal[2], c.longVal[3]);
}
