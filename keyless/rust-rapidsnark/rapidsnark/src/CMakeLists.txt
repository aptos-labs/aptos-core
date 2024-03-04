link_libraries(gmp)


if(USE_ASM)
    if(ARCH MATCHES "arm64")
            add_definitions(-DUSE_ASM -DARCH_ARM64)
    elseif(ARCH MATCHES "x86_64")
            add_definitions(-DUSE_ASM -DARCH_X86_64)
    endif()
endif()

if(USE_ASM AND ARCH MATCHES "x86_64")

    if (CMAKE_HOST_SYSTEM_NAME MATCHES "Darwin")
        set(NASM_FLAGS -fmacho64 --prefix _)
    else()
        set(NASM_FLAGS -felf64 -DPIC)
    endif()

    add_custom_command(OUTPUT ${CMAKE_SOURCE_DIR}/build/fq_asm.o
        COMMAND nasm ${NASM_FLAGS} fq.asm -o fq_asm.o
        DEPENDS ${CMAKE_SOURCE_DIR}/build/fq.asm
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/build)

    add_custom_command(OUTPUT ${CMAKE_SOURCE_DIR}/build/fr_asm.o
        COMMAND nasm ${NASM_FLAGS} fr.asm -o fr_asm.o
        DEPENDS ${CMAKE_SOURCE_DIR}/build/fr.asm
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}/build)
endif()

set(FR_SOURCES
    ../build/fr.hpp
    ../build/fr.cpp
    )

if(USE_ASM)
    if(ARCH MATCHES "arm64")
        set(FR_SOURCES ${FR_SOURCES} ../build/fr_raw_arm64.s ../build/fr_raw_generic.cpp ../build/fr_generic.cpp)
    elseif(ARCH MATCHES "x86_64")
        set(FR_SOURCES ${FR_SOURCES} ../build/fr_asm.o)
    endif()
else()
    set(FR_SOURCES ${FR_SOURCES} ../build/fr_generic.cpp ../build/fr_raw_generic.cpp)
endif()

add_library(fr STATIC ${FR_SOURCES})
set_target_properties(fr PROPERTIES POSITION_INDEPENDENT_CODE ON)

link_libraries(fr)

set(FQ_SOURCES
    ../build/fq.hpp
    ../build/fq.cpp
    )

if(USE_ASM)
    if(ARCH MATCHES "arm64")
        set(FQ_SOURCES ${FQ_SOURCES} ../build/fq_raw_arm64.s ../build/fq_raw_generic.cpp ../build/fq_generic.cpp)
    elseif(ARCH MATCHES "x86_64")
        set(FQ_SOURCES ${FQ_SOURCES} ../build/fq_asm.o)
    endif()
else()
    set(FQ_SOURCES ${FQ_SOURCES} ../build/fq_raw_generic.cpp ../build/fq_generic.cpp)
endif()

add_library(fq STATIC ${FQ_SOURCES})
set_target_properties(fq PROPERTIES POSITION_INDEPENDENT_CODE ON)

link_libraries(fq)


if(OpenMP_CXX_FOUND)
    add_definitions(-DUSE_OPENMP)
    add_compile_options(${OpenMP_CXX_FLAGS})
endif()

set(LIB_SOURCES
    binfile_utils.hpp
    binfile_utils.cpp
    zkey_utils.hpp
    zkey_utils.cpp
    wtns_utils.hpp
    wtns_utils.cpp
    logger.hpp
    logger.cpp
    fileloader.cpp
    fileloader.hpp
    prover.cpp
    prover.h
    proverapi.hpp
    proverapi.cpp
    fullprover.hpp
    fullprover.cpp
    ../depends/ffiasm/c/misc.cpp
    ../depends/ffiasm/c/naf.cpp
    ../depends/ffiasm/c/splitparstr.cpp
    ../depends/ffiasm/c/alt_bn128.cpp
    )

if(USE_LOGGER)
    set(LIB_SOURCES ${LIB_SOURCES} logger.cpp)
    add_definitions(-DUSE_LOGGER)
endif()

include_directories(
    ../src
    ../build
    ../depends/ffiasm/c
    ../depends/json/single_include
    ../depends/cpp-httplib/)


add_library(rapidsnarkStatic STATIC ${LIB_SOURCES})
set_target_properties(rapidsnarkStatic PROPERTIES OUTPUT_NAME rapidsnark)

add_library(rapidsnarkStaticFrFq STATIC ${LIB_SOURCES} ${FQ_SOURCES} ${FR_SOURCES})
set_target_properties(rapidsnarkStaticFrFq PROPERTIES POSITION_INDEPENDENT_CODE ON)
set_target_properties(rapidsnarkStaticFrFq PROPERTIES OUTPUT_NAME rapidsnark-fr-fq)

add_executable(prover main_prover.cpp)
target_link_libraries(prover rapidsnarkStatic)

add_library(rapidsnark SHARED ${LIB_SOURCES})

if(USE_LOGGER OR NOT USE_OPENMP)
    target_link_libraries(prover pthread)
endif()

if(USE_SODIUM)
    target_link_libraries(prover sodium)
endif()


if(OpenMP_CXX_FOUND)

    if(TARGET_PLATFORM MATCHES "android")
        target_link_libraries(prover -static-openmp -fopenmp)
        target_link_libraries(rapidsnark -static-openmp -fopenmp)

    elseif(CMAKE_HOST_SYSTEM_NAME STREQUAL "Linux")
        target_link_libraries(prover OpenMP::OpenMP_CXX)
    endif()

endif()


add_executable(test_prover test_prover.cpp)
