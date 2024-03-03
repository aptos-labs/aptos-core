#ifndef CIRCOM_CALCWIT_H
#define CIRCOM_CALCWIT_H

#include "circom.hpp"
#include "fr.hpp"
#include <mutex>
#include <condition_variable>
#include <functional>
#include <nlohmann/json.hpp>
using json = nlohmann::json;

#define NMUTEXES 128

class Circom_CalcWit {

    bool *signalAssigned;

    // componentStatus -> For each component
    // >0 Signals required to trigger
    // == 0 Component triggered
    // == -1 Component finished
    int *inputSignalsToTrigger;
    std::mutex *mutexes;
    std::condition_variable *cvs;

    std::mutex printf_mutex;

    FrElement *signalValues;


    void triggerComponent(int newCIdx);
    void calculateWitness(void *input, void *output);

    void syncPrintf(const char *format, ...);
    bool isCanceled() { return isCanceledCB && isCanceledCB(); }

    void itFunc(int o, json val);
    void iterateArr(int o, Circom_Sizes sizes, json jarr);

public:
    std::function<bool()> isCanceledCB;
    Circom_Circuit *circuit;

// Functions called by the circuit
    Circom_CalcWit(Circom_Circuit *aCircuit);
    ~Circom_CalcWit();

    int getSubComponentOffset(int cIdx, u64 hash);
    Circom_Sizes getSubComponentSizes(int cIdx, u64 hash);
    int getSignalOffset(int cIdx, u64 hash);
    Circom_Sizes getSignalSizes(int cIdx, u64 hash);

    void getSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value);
    void setSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value);
    void multiGetSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value, int n);

    void checkConstraint(int currentComponentIdx, PFrElement value1, PFrElement value2, char const *err);
    void checkAssert(int currentComponentIdx, PFrElement value1, char const *err);

    void log(PFrElement value);

    void finished(int cIdx);
    void join();


// Public functions
    inline void setInput(int idx, PFrElement val) {
        setSignal(0, 0, circuit->wit2sig[idx], val);
    }
    inline void getWitness(int idx, PFrElement val) {
        Fr_copy(val, &signalValues[circuit->wit2sig[idx]]);
    }

    void reset();

    void calculateProve(void *wtns, json &input, std::function<bool()> _isCanceledCB);
    void calculateProve(void *wtns, std::string &input, std::function<bool()> _isCanceledCB);

};



#endif // CIRCOM_CALCWIT_H
