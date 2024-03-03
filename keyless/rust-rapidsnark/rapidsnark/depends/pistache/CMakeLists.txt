# SPDX-FileCopyrightText: 2015 Mathieu Stefani
#
# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required (VERSION 3.11)

set(CMAKE_BUILD_TYPE_INIT Release)

file(READ "version.txt" VERSION_FILE_RAW)
string(STRIP "${VERSION_FILE_RAW}" VERSION_FILE)

project (pistache
    LANGUAGES C CXX
    VERSION ${VERSION_FILE})

add_compile_options(-Wall -Wextra -Wpedantic -Wconversion -Wno-sign-conversion -Wno-missing-field-initializers)

option(BUILD_SHARED_LIBS "build shared library" ON)
option(PISTACHE_BUILD_TESTS "build tests alongside the project" OFF)
option(PISTACHE_ENABLE_FLAKY_TESTS "if tests are built, also run ones that are known to be flaky" ON)
option(PISTACHE_ENABLE_NETWORK_TESTS "if tests are built, run ones needing network access" ON)
option(PISTACHE_USE_SSL "add support for SSL server" OFF)
option(PISTACHE_PIC "Enable pistache PIC" ON)
option(PISTACHE_BUILD_FUZZ "Build fuzzer for oss-fuzz" OFF)

# require fat LTO objects in static library
if(CMAKE_INTERPROCEDURAL_OPTIMIZATION OR CMAKE_CXX_FLAGS MATCHES "-flto" OR CMAKE_CXX_FLAGS MATCHES "-flto=thin")
    if(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
        add_compile_options(-ffat-lto-objects)
    elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
        add_compile_options(-flto=full)
    endif()
endif()

set(THREADS_PREFER_PTHREAD_FLAG ON)
find_package(Threads REQUIRED)

if (PISTACHE_USE_SSL)
    find_package(OpenSSL REQUIRED COMPONENTS SSL Crypto)
endif ()

# Set version numbers in a header file

set(VERSION_MAJOR    ${pistache_VERSION_MAJOR})
set(VERSION_MINOR    ${pistache_VERSION_MINOR})
set(VERSION_PATCH    ${pistache_VERSION_PATCH})
set(VERSION_GIT_DATE ${pistache_VERSION_TWEAK})
configure_file (
    "include/pistache/version.h.in"
    "include/pistache/version.h"
    @ONLY
)

add_subdirectory (src)

if (PISTACHE_BUILD_TESTS)
    include(CTest)
    find_package(GTest QUIET)
    if (NOT GTEST_FOUND)
    message("GoogleTest not found. Consider installing it on your system. Downloading it from source...")
    include(FetchContent)
        FetchContent_Declare(
            googletest
            GIT_REPOSITORY https://github.com/google/googletest.git
            GIT_TAG "origin/main"
            GIT_SHALLOW true
        )
        set(BUILD_GMOCK OFF CACHE BOOL "")
        FetchContent_GetProperties(googletest)
        if(NOT googletest_POPULATED)
            FetchContent_Populate(googletest)
            add_subdirectory(${googletest_SOURCE_DIR} ${googletest_BINARY_DIR})
        endif()
    endif()
    add_subdirectory(tests)
endif()

if (PISTACHE_BUILD_FUZZ) 
    add_subdirectory(tests/fuzzers)
endif()

# format target

add_custom_target(format
    COMMAND
        ./tools/format.sh
    WORKING_DIRECTORY
        ${CMAKE_CURRENT_SOURCE_DIR}
)

# CMake 3.21 defines this automatically
if (PROJECT_IS_TOP_LEVEL OR CMAKE_CURRENT_SOURCE_DIR STREQUAL CMAKE_SOURCE_DIR)
    install(CODE "message(FATAL_ERROR \"Please use Meson to install Pistache.
See the README for details: https://github.com/pistacheio/pistache#building-from-source\")")
endif()
