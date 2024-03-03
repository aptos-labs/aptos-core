#include "splitparstr.hpp"


void removePars(std::string &s) {
    unsigned int ns = 0;
    while ((s.length() >=ns*2)&&(s[ns]=='(')&&(s[s.length()-1-ns] == ')')) ns++;

    int ins = 0;
    int minIns = 0;
    for (unsigned int i=ns; i<s.length()-ns*2; i++) {
        if (s[i] == '(') ins ++; 
        if (s[i] == ')') ins --; 
        if (ins < minIns) minIns = ins;
    }
    s.erase(0, ns+minIns);
    s.erase(s.length()-(ns+minIns), s.length());
}

std::vector<std::string> splitParStr(std::string s) {

    std::vector<std::string> res;
    std::string accS;
    bool scaped = false;
    int nPar=0;
    for (unsigned int i=0; i<s.length(); i++) {
        if (scaped) {
            accS += s[i];
            scaped = false;
            continue;
        }
        if (std::isspace(s[i])) continue;
        if ((s[i] == ',')&&(nPar==0)) {
            removePars(accS);
            res.push_back(accS);
            accS.clear();
            continue;
        }
        if (s[i] == '(') {
            nPar++;
        }
        if (s[i] == ')') {
            nPar--;
        }
        accS += s[i];
    }
    removePars(accS);
    res.push_back(accS);
    if (res.size()==1) {
        removePars(res[0]);
        return splitParStr(res[0]);
    } else {
        return res;
    }
}