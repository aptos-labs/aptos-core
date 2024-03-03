#include <string>
#include <stdexcept>
#include <sstream>
#include <iostream>
#include <iomanip>
#include <stdlib.h>
#include <assert.h>
#include <stdarg.h>
#include <thread>
#include <chrono>
#include "calcwit.hpp"
#include "utils.hpp"

using namespace std::chrono_literals;

Circom_CalcWit::Circom_CalcWit(Circom_Circuit *aCircuit) {
    circuit = aCircuit;

    signalAssigned = new bool[circuit->NSignals];
    signalAssigned[0] = true;

    mutexes = new std::mutex[NMUTEXES];
    cvs = new std::condition_variable[NMUTEXES];
    inputSignalsToTrigger = new int[circuit->NComponents];
    signalValues = new FrElement[circuit->NSignals];

    // Set one signal
    Fr_copy(&signalValues[0], circuit->constants + 1);

    reset();
}


Circom_CalcWit::~Circom_CalcWit() {

    delete signalAssigned;

    delete[] cvs;
    delete[] mutexes;

    delete[] signalValues;
    delete[] inputSignalsToTrigger;

}

void Circom_CalcWit::syncPrintf(const char *format, ...) {
    va_list args;
    va_start(args, format);

    printf_mutex.lock();
    vprintf(format, args);
    printf_mutex.unlock();

    va_end(args);
}

void Circom_CalcWit::reset() {

    #pragma omp parallel for
    for (int i=1; i<circuit->NSignals; i++) {
        signalAssigned[i] = false;
    }

    #pragma omp parallel for
    for (int i=0; i<circuit->NComponents; i++) {
        inputSignalsToTrigger[i] = circuit->components[i].inputSignals;
    }

    for (int i=0; i<circuit->NComponents; i++) {
        if (inputSignalsToTrigger[i] == 0) triggerComponent(i);
    }
}


int Circom_CalcWit::getSubComponentOffset(int cIdx, u64 hash) {
    int hIdx;
    for(hIdx = int(hash & 0xFF); hash!=circuit->components[cIdx].hashTable[hIdx].hash; hIdx++) {
        if (!circuit->components[cIdx].hashTable[hIdx].hash) throw std::runtime_error("hash not found: " + int_to_hex(hash));
    }
    int entryPos = circuit->components[cIdx].hashTable[hIdx].pos;
    if (circuit->components[cIdx].entries[entryPos].type != _typeComponent) {
        throw std::runtime_error("invalid type");
    }
    return circuit->components[cIdx].entries[entryPos].offset;
}


Circom_Sizes Circom_CalcWit::getSubComponentSizes(int cIdx, u64 hash) {
    int hIdx;
    for(hIdx = int(hash & 0xFF); hash!=circuit->components[cIdx].hashTable[hIdx].hash; hIdx++) {
        if (!circuit->components[cIdx].hashTable[hIdx].hash) throw std::runtime_error("hash not found: " + int_to_hex(hash));
    }
    int entryPos = circuit->components[cIdx].hashTable[hIdx].pos;
    if (circuit->components[cIdx].entries[entryPos].type != _typeComponent) {
        throw std::runtime_error("invalid type");
    }
    return circuit->components[cIdx].entries[entryPos].sizes;
}

int Circom_CalcWit::getSignalOffset(int cIdx, u64 hash) {
    int hIdx;
    for(hIdx = int(hash & 0xFF); hash!=circuit->components[cIdx].hashTable[hIdx].hash; hIdx++) {
        if (!circuit->components[cIdx].hashTable[hIdx].hash) throw std::runtime_error("hash not found: " + int_to_hex(hash));
    }
    int entryPos = circuit->components[cIdx].hashTable[hIdx].pos;
    if (circuit->components[cIdx].entries[entryPos].type != _typeSignal) {
        throw std::runtime_error("invalid type");
    }
    return circuit->components[cIdx].entries[entryPos].offset;
}

Circom_Sizes Circom_CalcWit::getSignalSizes(int cIdx, u64 hash) {
    int hIdx;
    for(hIdx = int(hash & 0xFF); hash!=circuit->components[cIdx].hashTable[hIdx].hash; hIdx++) {
        if (!circuit->components[cIdx].hashTable[hIdx].hash) throw std::runtime_error("hash not found: " + int_to_hex(hash));
    }
    int entryPos = circuit->components[cIdx].hashTable[hIdx].pos;
    if (circuit->components[cIdx].entries[entryPos].type != _typeSignal) {
        throw std::runtime_error("invalid type");
    }
    return circuit->components[cIdx].entries[entryPos].sizes;
}

void Circom_CalcWit::getSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value) {
    // char *s = Fr_element2str(value);
    // syncPrintf("getSignal: %d %s\n", sIdx, s);
    // delete s;
    if ((circuit->components[cIdx].newThread)&&(currentComponentIdx != cIdx)) {
        std::unique_lock<std::mutex> lk(mutexes[cIdx % NMUTEXES]);
        while (!signalAssigned[sIdx]) {
            cvs[sIdx % NMUTEXES].wait_for(lk, 10ms);
        }
        lk.unlock();
    }
    if (signalAssigned[sIdx] == false) {
        fprintf(stderr, "Accessing a not assigned signal: %d\n", sIdx);
        assert(false);
    }
    Fr_copy(value, signalValues + sIdx);
    /*
    char *valueStr = mpz_get_str(0, 10, *value);
    syncPrintf("%d, Get %d --> %s\n", currentComponentIdx, sIdx, valueStr);
    free(valueStr);
    */
}

void Circom_CalcWit::multiGetSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value, int n) {
    for (int i=0; i<n; i++) {
        getSignal(currentComponentIdx, cIdx, sIdx+i, value + i);
    }
}

void Circom_CalcWit::finished(int cIdx) {
    {
        std::lock_guard<std::mutex> lk(mutexes[cIdx % NMUTEXES]);
        inputSignalsToTrigger[cIdx] = -1;
    }
    // syncPrintf("Finished: %d\n", cIdx);
    cvs[cIdx % NMUTEXES].notify_all();
}

void Circom_CalcWit::setSignal(int currentComponentIdx, int cIdx, int sIdx, PFrElement value) {
    // syncPrintf("setSignal: %d\n", sIdx);

    if (signalAssigned[sIdx] == true) {
        fprintf(stderr, "Signal assigned twice: %d\n", sIdx);
        assert(false);
    }
    // Log assignement
    /*
    char *valueStr = mpz_get_str(0, 10, *value);
    syncPrintf("%d, Set %d --> %s\n", currentComponentIdx, sIdx, valueStr);
    free(valueStr);
    */
    Fr_copy(signalValues + sIdx, value);
    signalAssigned[sIdx] = true;
    if ( BITMAP_ISSET(circuit->mapIsInput, sIdx) ) {
        if (inputSignalsToTrigger[cIdx]>0) {
            inputSignalsToTrigger[cIdx]--;
            if (inputSignalsToTrigger[cIdx] == 0) triggerComponent(cIdx);
        } else {
            fprintf(stderr, "Input signals does not match with map: %d\n", sIdx);
            assert(false);
        }
    }
    if ((circuit->components[currentComponentIdx].newThread)&&(currentComponentIdx == cIdx)) {
    // syncPrintf("Finished: %d\n", cIdx);
       cvs[sIdx % NMUTEXES].notify_all();
    }

}

void Circom_CalcWit::checkConstraint(int currentComponentIdx, PFrElement value1, PFrElement value2, char const *err) {
#ifdef SANITY_CHECK
    FrElement tmp;
    Fr_eq(&tmp, value1, value2);
    if (!Fr_isTrue(&tmp)) {
        char *pcV1 = Fr_element2str(value1);
        char *pcV2 = Fr_element2str(value2);
        // throw std::runtime_error(std::to_string(currentComponentIdx) + std::string(", Constraint doesn't match, ") + err + ". " + sV1 + " != " + sV2 );
        fprintf(stderr, "Constraint doesn't match, %s: %s != %s", err, pcV1, pcV2);
        free(pcV1);
        free(pcV2);
        assert(false);
    }
#endif
}

void Circom_CalcWit::checkAssert(int currentComponentIdx, PFrElement value1, char const *err) {
#ifdef SANITY_CHECK
    FrElement tmp;
    if (!Fr_isTrue(value1)) {
        char *pcV1 = Fr_element2str(value1);
        // throw std::runtime_error(std::to_string(currentComponentIdx) + std::string(", Constraint doesn't match, ") + err + ". " + sV1 + " != " + sV2 );
        fprintf(stderr, "Assert fail: %s", err);
        free(pcV1);
        assert(false);
    }
#endif
}

void Circom_CalcWit::triggerComponent(int newCIdx) {
    //int oldCIdx = cIdx;
    // cIdx = newCIdx;
    if (circuit->components[newCIdx].newThread) {
        // syncPrintf("Triggered: %d\n", newCIdx);
        std::thread t(circuit->components[newCIdx].fn, this, newCIdx);
        // t.join();
        t.detach();
    } else {
        (*(circuit->components[newCIdx].fn))(this, newCIdx);
    }
    // cIdx = oldCIdx;
}

void Circom_CalcWit::log(PFrElement value) {
    char *pcV = Fr_element2str(value);
    syncPrintf("Log: %s\n", pcV);
    free(pcV);
}

void Circom_CalcWit::join() {
    for (int i=0; i<circuit->NComponents; i++) {
        std::unique_lock<std::mutex> lk(mutexes[i % NMUTEXES]);
        while (inputSignalsToTrigger[i] != -1) {
            cvs[i % NMUTEXES].wait_for(lk, 10ms);
        }
        // cvs[i % NMUTEXES].wait(lk, [&]{return inputSignalsToTrigger[i] == -1;});
        lk.unlock();
        // syncPrintf("Joined: %d\n", i);
    }

}



void Circom_CalcWit::iterateArr(int o, Circom_Sizes sizes, json jarr) {
  if (!jarr.is_array()) {
    assert((sizes[0] == 1)&&(sizes[1] == 0));
    itFunc(o, jarr);
  } else {
    int n = sizes[0] / sizes[1];
    for (int i=0; i<n; i++) {
      iterateArr(o + i*sizes[1], sizes+1, jarr[i]);
      if (isCanceled()) return;
    }
  }
}


void Circom_CalcWit::itFunc(int o, json val) {

    FrElement v;

    std::string s;

    if (val.is_string()) {
        s = val.get<std::string>();
    } else if (val.is_number()) {

        double vd = val.get<double>();
        std::stringstream stream;
        stream << std::fixed << std::setprecision(0) << vd;
        s = stream.str();
    } else {
        throw new std::runtime_error("Invalid JSON type");
    }

    Fr_str2element (&v, s.c_str());

    setSignal(0, 0, o, &v);
}


void Circom_CalcWit::calculateProve(void *wtns, json &input, std::function<bool()> _isCanceledCB) {
    isCanceledCB = _isCanceledCB;
    reset();
    for (json::iterator it = input.begin(); it != input.end(); ++it) {
//      std::cout << it.key() << " => " << it.value() << '\n';
        u64 h = fnv1a(it.key());
        int o;
        try {
            o = getSignalOffset(0, h);
        } catch (std::runtime_error e) {
            std::ostringstream errStrStream;
            errStrStream << "Error loading variable: " << it.key() << "\n" << e.what();
            throw new std::runtime_error(errStrStream.str());
        }
        Circom_Sizes sizes = getSignalSizes(0, h);
        iterateArr(o, sizes, it.value());
        if (isCanceled()) break;
    }

    join();

    if (!isCanceled()) {

        #pragma omp raw for
        for (unsigned int i=0;i<circuit->NVars;i++) {
            FrElement v;
            getWitness(i, &v);
            Fr_toLongNormal(&v, &v);
            memcpy((uint8_t*)wtns + i*Fr_N64*8, v.longVal, Fr_N64*8);
        }
    }

    if (isCanceled()) throw new std::runtime_error("Aborted");
}

void Circom_CalcWit::calculateProve(void *wtns, std::string &input, std::function<bool()> _isCanceledCB) {

    json inputJson = json::parse(input, nullptr, false);
    if (inputJson.is_discarded()) throw new std::runtime_error("JSonParseError");
    calculateProve(wtns, inputJson, _isCanceledCB);
}



