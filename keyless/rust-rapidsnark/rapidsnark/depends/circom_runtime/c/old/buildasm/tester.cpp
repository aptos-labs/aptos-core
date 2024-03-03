#include <string>
#include <iostream>

#include <regex>
#include <string>
#include <iostream>
#include <stdexcept>
#include <sstream>

#include <stdio.h>      /* printf, NULL */
#include <stdlib.h>
#include <cassert>


#include "fr.h"


typedef void (*Func1)(PFrElement, PFrElement);
typedef void (*Func2)(PFrElement, PFrElement, PFrElement);
typedef void *FuncAny;

typedef struct {
    FuncAny fn;
    int nOps;
} FunctionSpec;

std::map<std::string, FunctionSpec> functions;
std::vector<FrElement> stack;

void addFunction(std::string name, FuncAny f, int nOps) {
    FunctionSpec fs;
    fs.fn = f;
    fs.nOps = nOps;
    functions[name] = fs;
}

void fillMap() {
    addFunction("add", (FuncAny)Fr_add, 2);
    addFunction("sub", (FuncAny)Fr_sub, 2);
    addFunction("neg", (FuncAny)Fr_neg, 1);
    addFunction("mul", (FuncAny)Fr_mul, 2);
    addFunction("square", (FuncAny)Fr_square, 1);
    addFunction("idiv", (FuncAny)Fr_idiv, 2);
    addFunction("inv", (FuncAny)Fr_inv, 1);
    addFunction("div", (FuncAny)Fr_div, 2);
    addFunction("band", (FuncAny)Fr_band, 2);
    addFunction("bor", (FuncAny)Fr_bor, 2);
    addFunction("bxor", (FuncAny)Fr_bxor, 2);
    addFunction("bnot", (FuncAny)Fr_bnot, 1);
    addFunction("eq", (FuncAny)Fr_eq, 2);
    addFunction("neq", (FuncAny)Fr_neq, 2);
    addFunction("lt", (FuncAny)Fr_lt, 2);
    addFunction("gt", (FuncAny)Fr_gt, 2);
    addFunction("leq", (FuncAny)Fr_leq, 2);
    addFunction("geq", (FuncAny)Fr_geq, 2);
    addFunction("land", (FuncAny)Fr_land, 2);
    addFunction("lor", (FuncAny)Fr_lor, 2);
    addFunction("lnot", (FuncAny)Fr_lnot, 1);
    addFunction("shl", (FuncAny)Fr_shl, 2);
    addFunction("shr", (FuncAny)Fr_shr, 2);
}

u_int64_t readInt(std::string &s) {
    if (s.rfind("0x", 0) == 0) {
        return std::stoull(s.substr(2), 0, 16);
    } else {
        return std::stoull(s, 0, 10);
    }
}

void pushNumber(std::vector<std::string> &v) {
    u_int64_t a;
    if ((v.size()<1) || (v.size() > (Fr_N64+1))) {
        printf("Invalid Size: %d  -  %d \n", v.size(), Fr_N64);
        throw std::runtime_error("Invalid number of parameters for number");
    }
    FrElement e;
    a = readInt(v[0]);
    *(u_int64_t *)(&e) = a;
    for (int i=0; i<Fr_N64; i++) {
        if (i+1 < v.size()) {
            a = readInt(v[i+1]);
        } else {
            a = 0;
        }
        e.longVal[i] = a;
    }
    stack.push_back(e);
}

void callFunction(FunctionSpec fs) {
    if (stack.size() < fs.nOps) {
        throw new std::runtime_error("Not enough elements in stack");
    }
    if (fs.nOps == 1) {
        FrElement a = stack.back();
        stack.pop_back();
        FrElement c;
        (*(Func1)fs.fn)(&c, &a);
        stack.push_back(c);
    } else if (fs.nOps == 2) {
        FrElement b = stack.back();
        stack.pop_back();
        FrElement a = stack.back();
        stack.pop_back();
        FrElement c;
        (*(Func2)fs.fn)(&c, &a, &b);
        stack.push_back(c);
    } else {
        assert(false);
    }
}

void processLine(std::string &line) {
    std::regex re("(\\s*[,;]\\s*)|\\s+"); // whitespace

    std::sregex_token_iterator begin( line.begin(), line.end(), re ,-1);
    std::sregex_token_iterator end;
    std::vector<std::string> tokens;

    std::copy(begin, end, std::back_inserter(tokens));

    // Remove initial empty tokens
    while ((tokens.size() > 0)&&(tokens[0] == "")) {
        tokens.erase(tokens.begin());
    }

    // Empty lines are valid but are not processed
    if (tokens.size() == 0) return;

    auto search = functions.find(tokens[0]);
    if (search == functions.end()) {
        pushNumber(tokens);
    } else {
        if (tokens.size() != 1) {
            throw std::runtime_error("Functions does not accept parameters");
        }
        callFunction(search->second);
    }
}

int main(void)
{
    Fr_init();
    fillMap();
    std::string line;
    int i=0;
    while (std::getline(std::cin, line)) {
        processLine(line);
        // if (i%1000 == 0) printf("%d\n", i);
        // printf("%d\n", i);
        i++;
    }
    // Print the elements in the stack
    //
    for (int i=0; i<stack.size(); i++) {
        char *s;
        s = Fr_element2str(&stack[i]);
        printf("%s\n", s);
        free(s);
    }
    return EXIT_SUCCESS;
}



/*


#include <stdlib.h>
#include <string.h>
#include "fr.h"

typedef void (*Func2)(PFrElement, PFrElement, PFrElement);

typedef struct {
    const char *fnName;
    Func2 fn;
} FN;


#define NFN 2
FN fns[NFN] = {
    {"add", Fr_add},
    {"mul", Fr_mul},
};

int main(int argc, char **argv) {

    if (argc <= 1) {
        fprintf( stderr, "invalid number of parameters");
        return 1;
    }

    for (int i=0; i< NFN;i++) {
        if (strcmp(argv[1], fns[i].fnName) == 0) {
            if (argc != 4) {
                fprintf( stderr, "invalid number of parameters");
                return 1;
            }
            FrElement a;
            FrElement b;

            Fr_str2element(&a, argv[2]);
            Fr_str2element(&b, argv[3]);
            FrElement c;
            fns[i].fn(&c, &a, &b);

            char *s;
            s = Fr_element2str(&c);
            printf("%s", s);
            free(s);
            return 0;
        }
    }
    fprintf( stderr, "invalid operation %s", argv[1]);
    return 1;
}

*/
