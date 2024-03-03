#include <stdio.h>
#include <stdlib.h>
#include "fr.hpp"

int main(int argc, char **argv) {

    int N = atoi(argv[1]);

    RawFr F;

    RawFr::Element a;
    F.fromString(a, "99999999999");
    
    for (int i=0; i<N; i++) {
        F.copy(a, a, a);
    }

    /*
    char *c1 = Fr_element2str(&a);
    printf("Result: %s\n", a);
    free(c1);
    */

}
