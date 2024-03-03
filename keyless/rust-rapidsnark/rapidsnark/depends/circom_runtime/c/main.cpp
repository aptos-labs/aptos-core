#include <iostream>
#include <fstream>
#include <sstream>
#include <string>
#include <iomanip>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/mman.h>
#include <sys/ipc.h>
#include <sys/shm.h>
#include <fcntl.h>
#include <unistd.h>
#include <nlohmann/json.hpp>
#include <sys/time.h>
using json = nlohmann::json;

#include "calcwit.hpp"
#include "circom.hpp"
#include "utils.hpp"

Circom_Circuit *circuit;


#define handle_error(msg) \
           do { perror(msg); exit(EXIT_FAILURE); } while (0)

#define SHMEM_WITNESS_KEY (123456)

// assumptions
// 1) There is only one key assigned for shared memory. This means
//      that only one witness can be computed and used at a time. If several witness
//      are computed before calling the prover, witness memory will be overwritten.
// 2) Prover is responsible for releasing memory once is done with witness
//
// File format:
// Type     : 4B (wshm)
// Version  : 4B
// N Section : 4B
// HDR1     : 12B
// N8       : 4B
// Fr       : N8 B
// NVars    : 4B
// HDR2     : 12B
// ShmemKey : 4B
// Status   : 4B  (0:OK, 0xFFFF: KO)
// ShmemID  : 4B
void writeOutShmem(Circom_CalcWit *ctx, std::string filename) {
    FILE *write_ptr;
    u64 *shbuf;
    int shmid, status = 0;

    write_ptr = fopen(filename.c_str(),"wb");

    fwrite("wshm", 4, 1, write_ptr);

    u32 version = 2;
    fwrite(&version, 4, 1, write_ptr);

    u32 nSections = 2;
    fwrite(&nSections, 4, 1, write_ptr);

    // Header
    u32 idSection1 = 1;
    fwrite(&idSection1, 4, 1, write_ptr);

    u32 n8 = Fr_N64*8;

    u64 idSection1length = 8 + n8;
    fwrite(&idSection1length, 8, 1, write_ptr);

    fwrite(&n8, 4, 1, write_ptr);

    fwrite(Fr_q.longVal, Fr_N64*8, 1, write_ptr);

    u32 nVars = circuit->NVars;
    fwrite(&nVars, 4, 1, write_ptr);

    // Data
    u32 idSection2 = 2;
    fwrite(&idSection2, 4, 1, write_ptr);

    u64 idSection2length = n8*circuit->NVars;
    fwrite(&idSection2length, 8, 1, write_ptr);


    // generate key
    key_t key = SHMEM_WITNESS_KEY;
    fwrite(&key, sizeof(key_t), 1, write_ptr);

    // Setup shared memory
    if ((shmid = shmget(key, circuit->NVars * Fr_N64 * sizeof(u64), IPC_CREAT | 0666)) < 0) {
       // preallocated shared memory segment is too small => Retrieve id by accesing old segment
       // Delete old segment and create new with corret size
       shmid = shmget(key, 4, IPC_CREAT | 0666);
       shmctl(shmid, IPC_RMID, NULL);
       if ((shmid = shmget(key, circuit->NVars * Fr_N64 * sizeof(u64), IPC_CREAT | 0666)) < 0){
         status = -1;
         fwrite(&status, sizeof(status), 1, write_ptr);
         fclose(write_ptr);
         return ;
      }
    }

    // Attach shared memory
    if ((shbuf = (u64 *)shmat(shmid, NULL, 0)) == (u64 *) -1) {
      status = -1;
      fwrite(&status, sizeof(status), 1, write_ptr);
      fclose(write_ptr);
      return;
    }
    fwrite(&status, sizeof(status), 1, write_ptr);

    fwrite(&shmid, sizeof(u32), 1, write_ptr);
    fclose(write_ptr);


    #pragma omp parallel for
    for (int i=0; i<circuit->NVars;i++) {
    	FrElement v;
        ctx->getWitness(i, &v);
        Fr_toLongNormal(&v, &v);
        memcpy(&shbuf[i*Fr_N64], v.longVal, Fr_N64*sizeof(u64));
    }
}


void loadBin(Circom_CalcWit *ctx, std::string filename) {
    int fd;
    struct stat sb;

    // map input
    fd = open(filename.c_str(), O_RDONLY);
    if (fd == -1)
        handle_error("open");

    if (fstat(fd, &sb) == -1)           /* To obtain file size */
        handle_error("fstat");


    u8 *in;

    in = (u8 *)mmap(NULL, sb.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if (in == MAP_FAILED)
        handle_error("mmap");

    close(fd);

    FrElement v;
    u8 *p = in;
    for (int i=0; i<circuit->NInputs; i++) {
        v.type = Fr_LONG;
        for (int j=0; j<Fr_N64; j++) {
            v.longVal[j] = *(u64 *)p;
        }
        p += 8;
        ctx->setSignal(0, 0, circuit->wit2sig[1 + circuit->NOutputs + i], &v);
    }
}


typedef void (*ItFunc)(Circom_CalcWit *ctx, int idx, json val);

void iterateArr(Circom_CalcWit *ctx, int o, Circom_Sizes sizes, json jarr, ItFunc f) {
  if (!jarr.is_array()) {
    assert((sizes[0] == 1)&&(sizes[1] == 0));
    f(ctx, o, jarr);
  } else {
    int n = sizes[0] / sizes[1];
    for (int i=0; i<n; i++) {
      iterateArr(ctx, o + i*sizes[1], sizes+1, jarr[i], f);
    }
  }
}

void itFunc(Circom_CalcWit *ctx, int o, json val) {

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
        handle_error("Invalid JSON type");
    }

    Fr_str2element (&v, s.c_str());

    ctx->setSignal(0, 0, o, &v);
}

void loadJson(Circom_CalcWit *ctx, std::string filename) {
    std::ifstream inStream(filename);
    json j;
    inStream >> j;

    u64 nItems = j.size();
    printf("Items : %llu\n",nItems);
    for (json::iterator it = j.begin(); it != j.end(); ++it) {
//      std::cout << it.key() << " => " << it.value() << '\n';
      u64 h = fnv1a(it.key());
      int o;
      try {
        o = ctx->getSignalOffset(0, h);
      } catch (std::runtime_error e) {
        std::ostringstream errStrStream;
        errStrStream << "Error loadin variable: " << it.key() << "\n" << e.what();
        throw std::runtime_error(errStrStream.str() );
      }
      Circom_Sizes sizes = ctx->getSignalSizes(0, h);
      iterateArr(ctx, o, sizes, it.value(), itFunc);
    }
}


void writeOutBin(Circom_CalcWit *ctx, std::string filename) {
    FILE *write_ptr;

    write_ptr = fopen(filename.c_str(),"wb");

    fwrite("wtns", 4, 1, write_ptr);

    u32 version = 2;
    fwrite(&version, 4, 1, write_ptr);

    u32 nSections = 2;
    fwrite(&nSections, 4, 1, write_ptr);

    // Header
    u32 idSection1 = 1;
    fwrite(&idSection1, 4, 1, write_ptr);

    u32 n8 = Fr_N64*8;

    u64 idSection1length = 8 + n8;
    fwrite(&idSection1length, 8, 1, write_ptr);

    fwrite(&n8, 4, 1, write_ptr);

    fwrite(Fr_q.longVal, Fr_N64*8, 1, write_ptr);

    u32 nVars = circuit->NVars;
    fwrite(&nVars, 4, 1, write_ptr);


    // Data
    u32 idSection2 = 2;
    fwrite(&idSection2, 4, 1, write_ptr);

    u64 idSection2length = (u64)n8*(u64)circuit->NVars;
    fwrite(&idSection2length, 8, 1, write_ptr);

    FrElement v;

    for (int i=0;i<circuit->NVars;i++) {
        ctx->getWitness(i, &v);
        Fr_toLongNormal(&v, &v);
        fwrite(v.longVal, Fr_N64*8, 1, write_ptr);
    }
    fclose(write_ptr);

}


void writeOutJson(Circom_CalcWit *ctx, std::string filename) {

    std::ofstream outFile;
    outFile.open (filename);

    outFile << "[\n";

    FrElement v;

    for (int i=0;i<circuit->NVars;i++) {
        ctx->getWitness(i, &v);
        char *pcV = Fr_element2str(&v);
        std::string sV = std::string(pcV);
        outFile << (i ? "," : " ") << "\"" << sV << "\"\n";
        free(pcV);
    }

    outFile << "]\n";
    outFile.close();
}

bool hasEnding (std::string const &fullString, std::string const &ending) {
    if (fullString.length() >= ending.length()) {
        return (0 == fullString.compare (fullString.length() - ending.length(), ending.length(), ending));
    } else {
        return false;
    }
}

#define ADJ_P(a) *((void **)&a) = (void *)(((char *)circuit)+ (uint64_t)(a))

Circom_Circuit *loadCircuit(std::string const &datFileName) {
    Circom_Circuit *circuitF;
    Circom_Circuit *circuit;

    int fd;
    struct stat sb;

    fd = open(datFileName.c_str(), O_RDONLY);
    if (fd == -1) {
        std::cout << ".dat file not found: " << datFileName << "\n";
        throw std::system_error(errno, std::generic_category(), "open");
    }

    if (fstat(fd, &sb) == -1) {         /* To obtain file size */
        throw std::system_error(errno, std::generic_category(), "fstat");
    }

    circuitF = (Circom_Circuit *)mmap(NULL, sb.st_size, PROT_READ , MAP_PRIVATE, fd, 0);
    close(fd);

    circuit = (Circom_Circuit *)malloc(sb.st_size);
    memcpy((void *)circuit, (void *)circuitF, sb.st_size);

    munmap(circuitF, sb.st_size);

    ADJ_P(circuit->wit2sig);
    ADJ_P(circuit->components);
    ADJ_P(circuit->mapIsInput);
    ADJ_P(circuit->constants);
    ADJ_P(circuit->P);
    ADJ_P(circuit->componentEntries);

    for (int i=0; i<circuit->NComponents; i++) {
        ADJ_P(circuit->components[i].hashTable);
        ADJ_P(circuit->components[i].entries);
        circuit->components[i].fn = _functionTable[  (uint64_t)circuit->components[i].fn];
    }

    for (int i=0; i<circuit->NComponentEntries; i++) {
        ADJ_P(circuit->componentEntries[i].sizes);
    }

    return circuit;
}

int main(int argc, char *argv[]) {
    if (argc!=3) {
        std::string cl = argv[0];
        std::string base_filename = cl.substr(cl.find_last_of("/\\") + 1);
        std::cout << "Usage: " << base_filename << " <input.<bin|json>> <output.<wtns|json|wshm>>\n";
    } else {

        struct timeval begin, end;
        long seconds, microseconds; 
        double elapsed;

	gettimeofday(&begin,0);

        std::string datFileName = argv[0];
        datFileName += ".dat";

        circuit = loadCircuit(datFileName);

        // open output
        Circom_CalcWit *ctx = new Circom_CalcWit(circuit);

        std::string infilename = argv[1];
	    gettimeofday(&end,0);
            seconds = end.tv_sec - begin.tv_sec;
            microseconds = end.tv_usec - begin.tv_usec;
            elapsed = seconds + microseconds*1e-6;

            printf("Up to loadJson %.20f\n", elapsed);

        if (hasEnding(infilename, std::string(".bin"))) {
            loadBin(ctx, infilename);
        } else if (hasEnding(infilename, std::string(".json"))) {
            loadJson(ctx, infilename);
        } else {
            handle_error("Invalid input extension (.bin / .json)");
        }

        ctx->join();

        // printf("Finished!\n");

        std::string outfilename = argv[2];

        if (hasEnding(outfilename, std::string(".wtns"))) {
	    gettimeofday(&end,0);
            seconds = end.tv_sec - begin.tv_sec;
            microseconds = end.tv_usec - begin.tv_usec;
            elapsed = seconds + microseconds*1e-6;

            printf("Up to WriteWtns %.20f\n", elapsed);
            writeOutBin(ctx, outfilename);
        } else if (hasEnding(outfilename, std::string(".json"))) {
            writeOutJson(ctx, outfilename);
        } else if (hasEnding(outfilename, std::string(".wshm"))) {
	    gettimeofday(&end,0);
            seconds = end.tv_sec - begin.tv_sec;
            microseconds = end.tv_usec - begin.tv_usec;
            elapsed = seconds + microseconds*1e-6;

            printf("Up to WriteShmem %.20f\n", elapsed);
            writeOutShmem(ctx, outfilename);
        } else {
            handle_error("Invalid output extension (.bin / .json)");
        }

        delete ctx;
	    gettimeofday(&end,0);
            seconds = end.tv_sec - begin.tv_sec;
            microseconds = end.tv_usec - begin.tv_usec;
            elapsed = seconds + microseconds*1e-6;

            printf("Total %.20f\n", elapsed);
        exit(EXIT_SUCCESS);
    }
}
