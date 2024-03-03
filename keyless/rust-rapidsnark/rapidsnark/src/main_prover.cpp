#include <iostream>
#include <fstream>
#include <gmp.h>
#include <memory>
#include <stdexcept>
#include <nlohmann/json.hpp>

#include <alt_bn128.hpp>
#include "binfile_utils.hpp"
#include "zkey_utils.hpp"
#include "wtns_utils.hpp"
#include "groth16.hpp"
#include "fullprover.hpp"

using json = nlohmann::json;

#define handle_error(msg) \
           do { perror(msg); exit(EXIT_FAILURE); } while (0)

int main(int argc, char **argv) {
    if (argc != 5) {
        std::cerr << "Invalid number of parameters:\n";
        std::cerr << "Usage: prover <circuit.zkey> <witnessBinaryPath> <proof.json>\n";
        return EXIT_FAILURE;
    }


    try {
        const char *zkeyFilename = argv[1];
        const char *witnessBinaryPath = argv[2];
        std::string inputFilename = argv[3];
        std::string proofFilename = argv[4];

        FullProver fullProver(zkeyFilename, witnessBinaryPath);

        std::ifstream inputFile;
        inputFile.open(inputFilename);

        std::stringstream buffer;
        buffer << inputFile.rdbuf();

        ProverResponse response = fullProver.prove(buffer.str().c_str());
        std::cout << "Witness generation time: " << response.metrics.witness_generation_time << std::endl;
        std::cout << "Prover time: " << response.metrics.prover_time << std::endl;

        std::ofstream proofFile;
        proofFile.open (proofFilename);
        proofFile << response.raw_json;
        proofFile.close();


    } catch (std::exception* e) {
        std::cerr << e->what() << '\n';
        return EXIT_FAILURE;
    } catch (std::exception& e) {
        std::cerr << e.what() << '\n';
        return EXIT_FAILURE;
    }

    exit(EXIT_SUCCESS);
}
