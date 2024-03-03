const { sh, cli } = require("tasksfile");

function cleanAll() {
    sh("rm -rf build");
}

function downloadGoogleTest() {
    sh("mkdir -p build");
    sh("wget https://github.com/google/googletest/archive/release-1.10.0.tar.gz", {cwd: "build"});
    sh("tar xzf release-1.10.0.tar.gz", {cwd: "build"});
    sh("rm  release-1.10.0.tar.gz", {cwd: "build"});
}

function compileGoogleTest() {
    sh("g++ -Igoogletest -Igoogletest/include -c googletest/src/gtest-all.cc", {cwd: "build/googletest-release-1.10.0"});
    sh("ar -rv libgtest.a gtest-all.o",{cwd: "build/googletest-release-1.10.0"});
}

function createFieldSources() {
    sh("node ../src/buildzqfield.js -q 21888242871839275222246405745257275088696311157297823662689037894645226208583 -n Fq", {cwd: "build"});
    sh("node ../src/buildzqfield.js -q 21888242871839275222246405745257275088548364400416034343698204186575808495617 -n Fr", {cwd: "build"});

    if (process.platform === "darwin") {
        sh("nasm -fmacho64 --prefix _ fq.asm", {cwd: "build"});
    }  else if (process.platform === "linux") {
        sh("nasm -felf64 fq.asm", {cwd: "build"});
    } else throw("Unsupported platform");

    if (process.platform === "darwin") {
        sh("nasm -fmacho64 --prefix _ fr.asm", {cwd: "build"});
    }  else if (process.platform === "linux") {
        sh("nasm -felf64 fr.asm", {cwd: "build"});
    } else throw("Unsupported platform");
}

function testSplitParStr() {
    sh("g++" +
        " -Igoogletest-release-1.10.0/googletest/include"+
        " -I."+
        " -I../src"+
        " ../c/splitparstr.cpp"+
        " ../c/splitparstr_test.cpp"+
        " googletest-release-1.10.0/libgtest.a"+
        " -pthread -std=c++11 -o splitparsestr_test", {cwd: "build", nopipe: true}
    );
    sh("./splitparsestr_test", {cwd: "build", nopipe: true});
}

function testAltBn128() {
    sh("g++" +
        " -Igoogletest-release-1.10.0/googletest/include"+
        " -I."+
        " -I../c"+
        " ../c/naf.cpp"+
        " ../c/splitparstr.cpp"+
        " ../c/alt_bn128.cpp"+
        " ../c/alt_bn128_test.cpp"+
        " ../c/misc.cpp"+
        " fq.cpp"+
        " fq.o"+
        " fr.cpp"+
        " fr.o"+
        " googletest-release-1.10.0/libgtest.a"+
        " -o altbn128_test" +
        " -fmax-errors=5 -pthread -std=c++11 -fopenmp -lgmp -g", {cwd: "build", nopipe: true}
    );
    sh("./altbn128_test", {cwd: "build", nopipe: true});
}


function benchMultiExpG1() {
    sh("g++ -O3 -g" +
        " -Igoogletest-release-1.10.0/googletest/include"+
        " -I."+
        " -I../c"+
        " ../c/naf.cpp"+
        " ../c/splitparstr.cpp"+
        " ../c/alt_bn128.cpp"+
        " ../c/misc.cpp"+
        " ../benchmark/multiexp_g1.cpp"+
        " fq.cpp"+
        " fq.o"+
        " fr.cpp"+
        " fr.o"+
        // " googletest-release-1.10.0/libgtest.a"+
        " -o multiexp_g1_benchmark" +
        " -lgmp -pthread -std=c++11 -fopenmp" , {cwd: "build", nopipe: true}
    );
    sh("./multiexp_g1_benchmark 16777216", {cwd: "build", nopipe: true});
}

function benchMultiExpG2() {
    sh("g++" +
        " -Igoogletest-release-1.10.0/googletest/include"+
        " -I."+
        " -I../c"+
        " ../c/naf.cpp"+
        " ../c/splitparstr.cpp"+
        " ../c/alt_bn128.cpp"+
        " ../c/misc.cpp"+
        " ../benchmark/multiexp_g2.cpp"+
        " fq.cpp"+
        " fq.o"+
        " fr.cpp"+
        " fr.o"+
        // " googletest-release-1.10.0/libgtest.a"+
        " -o multiexp_g2_benchmark" +
        " -lgmp -pthread -std=c++11 -fopenmp" , {cwd: "build", nopipe: true}
    );
    sh("./multiexp_g2_benchmark 16777216", {cwd: "build", nopipe: true});
}

cli({
    cleanAll,
    downloadGoogleTest,
    compileGoogleTest,
    createFieldSources,
    testSplitParStr,
    testAltBn128,
    benchMultiExpG1,
    benchMultiExpG2,
});
