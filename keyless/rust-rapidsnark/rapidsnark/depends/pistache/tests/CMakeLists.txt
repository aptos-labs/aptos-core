# SPDX-FileCopyrightText: 2015 Mathieu Stefani
#
# SPDX-License-Identifier: Apache-2.0

find_package(CURL REQUIRED)

function(pistache_test test_name)
    set(TEST_EXECUTABLE run_${test_name})
    set(TEST_SOURCE ${test_name}.cc)

    add_executable(${TEST_EXECUTABLE} ${TEST_SOURCE})
    target_include_directories(${TEST_EXECUTABLE} PRIVATE
        ${CURL_INCLUDE_DIRS}
        ${CMAKE_SOURCE_DIR}/subprojects/hinnant-date/include
        ${CMAKE_SOURCE_DIR}/subprojects/cpp-httplib
    )
    # CMake 3.20 and upstream GTest define GTest::gtest, older versions define GTest::GTest
    if (TARGET GTest::gtest)
        target_link_libraries(${TEST_EXECUTABLE} GTest::gtest GTest::gtest_main pistache_static ${CURL_LIBRARIES})
    else ()
        target_link_libraries(${TEST_EXECUTABLE} GTest::GTest GTest::Main pistache_static ${CURL_LIBRARIES})
    endif ()
    add_test(${test_name} ${TEST_EXECUTABLE})
endfunction()


pistache_test(mime_test)
pistache_test(headers_test)
pistache_test(async_test)
pistache_test(typeid_test)
pistache_test(router_test)
pistache_test(cookie_test)
pistache_test(cookie_test_2)
pistache_test(cookie_test_3)
pistache_test(view_test)
pistache_test(http_parsing_test)
pistache_test(http_uri_test)
pistache_test(http_server_test)
pistache_test(http_client_test)
if (PISTACHE_ENABLE_NETWORK_TESTS)
    pistache_test(net_test)
endif (PISTACHE_ENABLE_NETWORK_TESTS)
pistache_test(listener_test)
pistache_test(request_size_test)
pistache_test(streaming_test)
pistache_test(rest_server_test)
pistache_test(mailbox_test)
pistache_test(stream_test)
pistache_test(reactor_test)
pistache_test(threadname_test)
pistache_test(log_api_test)
pistache_test(string_logger_test)
pistache_test(endpoint_initialization_test)

if (PISTACHE_USE_SSL)

    configure_file("certs/server.crt" "certs/server.crt" COPYONLY)
    configure_file("certs/server.key" "certs/server.key" COPYONLY)
    configure_file("certs/client.key" "certs/client.key" COPYONLY)
    configure_file("certs/client.crt" "certs/client.crt" COPYONLY)
    configure_file("certs/rootCA.crt" "certs/rootCA.crt" COPYONLY)
    configure_file("certs/server_from_intermediate_with_chain.crt" "certs/server_from_intermediate_with_chain.crt" COPYONLY)
    configure_file("certs/server_from_intermediate.crt" "certs/server_from_intermediate.crt" COPYONLY)
    configure_file("certs/server_from_intermediate.key" "certs/server_from_intermediate.key" COPYONLY)
    configure_file("certs/intermediateCA.crt" "certs/intermediateCA.crt" COPYONLY)
    configure_file("certs/server_protected.crt" "certs/server_protected.crt" COPYONLY)
    configure_file("certs/server_protected.key" "certs/server_protected.key" COPYONLY)

    pistache_test(https_server_test)
endif (PISTACHE_USE_SSL)
