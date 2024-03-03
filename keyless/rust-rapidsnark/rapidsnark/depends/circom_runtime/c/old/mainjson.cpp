#include <iostream>
#include <nlohmann/json.hpp>
#include <map>
using json = nlohmann::json;


#include "utils.h"
#include "circom.h"
#include "calcwit.h"

auto j = R"(
  {
    "in": "314"
  }
)"_json;

typedef void (*ItFunc)(int idx, json val);

void iterateArr(int o, Circom_Sizes sizes, json jarr, ItFunc f) {
  if (!jarr.is_array()) {
    assert((sizes[0] == 1)&&(sizes[1] == 0));
    f(o, jarr);
  } else {
    int n = sizes[0] / sizes[1];
    for (int i=0; i<n; i++) {
      iterateArr(o + i*sizes[1], sizes+1, jarr[i], f);
    }
  }
}

void itFunc(int o, json v) {
  std::cout << o << " <-- " << v << '\n';
}


int main(int argc, char **argv) {

    Circom_CalcWit *ctx = new Circom_CalcWit(&_circuit);

    for (json::iterator it = j.begin(); it != j.end(); ++it) {
//      std::cout << it.key() << " => " << it.value() << '\n';
      u64 h = fnv1a(it.key());
      int o = ctx->getSignalOffset(0, h);
      Circom_Sizes sizes = ctx->getSignalSizes(0, h);
      iterateArr(o, sizes, it.value(), itFunc);
    }
}

