/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/*
   Mathieu Stefani, 07 février 2016

 * Http client example
*/

#include <atomic>

#include <pistache/client.h>
#include <pistache/http.h>
#include <pistache/net.h>

using namespace Pistache;

int main(int argc, char* argv[])
{
    if (argc < 2)
    {
        std::cerr << "Usage: http_client page [count]\n";
        return 1;
    }

    std::string page = argv[1];
    int count        = 1;
    if (argc == 3)
    {
        count = std::stoi(argv[2]);
    }

    Http::Experimental::Client client;

    auto opts = Http::Experimental::Client::options().threads(1).maxConnectionsPerHost(8);
    client.init(opts);

    std::vector<Async::Promise<Http::Response>> responses;

    std::atomic<size_t> completedRequests(0);
    std::atomic<size_t> failedRequests(0);

    auto start = std::chrono::system_clock::now();

    for (int i = 0; i < count; ++i)
    {
        auto resp = client.get(page).cookie(Http::Cookie("FOO", "bar")).send();
        resp.then(
            [&](Http::Response response) {
                ++completedRequests;
                std::cout << "Response code = " << response.code() << std::endl;
                auto body = response.body();
                if (!body.empty())
                    std::cout << "Response body = " << body << std::endl;
            },
            [&](std::exception_ptr exc) {
                ++failedRequests;
                PrintException excPrinter;
                excPrinter(exc);
            });
        responses.push_back(std::move(resp));
    }

    auto sync = Async::whenAll(responses.begin(), responses.end());
    Async::Barrier<std::vector<Http::Response>> barrier(sync);
    barrier.wait_for(std::chrono::seconds(5));

    auto end = std::chrono::system_clock::now();
    std::cout << "Summary of execution\n"
              << "Total number of requests sent     : " << count << '\n'
              << "Total number of responses received: "
              << completedRequests.load() << '\n'
              << "Total number of requests failed   : " << failedRequests.load()
              << '\n'
              << "Total time of execution           : "
              << std::chrono::duration_cast<std::chrono::milliseconds>(end - start)
                     .count()
              << "ms" << std::endl;

    client.shutdown();
}
