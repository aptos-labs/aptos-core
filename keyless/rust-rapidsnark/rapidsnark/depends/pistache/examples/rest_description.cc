/*
 * SPDX-FileCopyrightText: 2016 Mathieu Stefani
 *
 * SPDX-License-Identifier: Apache-2.0
 */

/* rest_description.cc
   Mathieu Stefani, 27 février 2016
   
   Example of how to use the Description mechanism
*/

#include <pistache/description.h>
#include <pistache/endpoint.h>
#include <pistache/http.h>

#include <pistache/serializer/rapidjson.h>

using namespace Pistache;

namespace Generic
{

    void handleReady(const Rest::Request&, Http::ResponseWriter response)
    {
        response.send(Http::Code::Ok, "1");
    }

}

class BankerService
{
public:
    BankerService(Address addr)
        : httpEndpoint(std::make_shared<Http::Endpoint>(addr))
        , desc("Banking API", "0.1")
    { }

    void init(size_t thr = 2)
    {
        auto opts = Http::Endpoint::options()
                        .threads(static_cast<int>(thr));
        httpEndpoint->init(opts);
        createDescription();
    }

    void start()
    {
        router.initFromDescription(desc);

        Rest::Swagger swagger(desc);
        swagger
            .uiPath("/doc")
            .uiDirectory("/home/octal/code/web/swagger-ui-2.1.4/dist")
            .apiPath("/banker-api.json")
            .serializer(&Rest::Serializer::rapidJson)
            .install(router);

        httpEndpoint->setHandler(router.handler());
        httpEndpoint->serve();
    }

private:
    void createDescription()
    {
        desc
            .info()
            .license("Apache", "http://www.apache.org/licenses/LICENSE-2.0");

        auto backendErrorResponse = desc.response(Http::Code::Internal_Server_Error, "An error occured with the backend");

        desc
            .schemes(Rest::Scheme::Http)
            .basePath("/v1")
            .produces(MIME(Application, Json))
            .consumes(MIME(Application, Json));

        desc
            .route(desc.get("/ready"))
            .bind(&Generic::handleReady)
            .response(Http::Code::Ok, "Response to the /ready call")
            .hide();

        auto versionPath = desc.path("/v1");

        auto accountsPath = versionPath.path("/accounts");

        accountsPath
            .route(desc.get("/all"))
            .bind(&BankerService::retrieveAllAccounts, this)
            .produces(MIME(Application, Json), MIME(Application, Xml))
            .response(Http::Code::Ok, "The list of all account");

        accountsPath
            .route(desc.get("/:name"), "Retrieve an account")
            .bind(&BankerService::retrieveAccount, this)
            .produces(MIME(Application, Json))
            .parameter<Rest::Type::String>("name", "The name of the account to retrieve")
            .response(Http::Code::Ok, "The requested account")
            .response(backendErrorResponse);

        accountsPath
            .route(desc.post("/:name"), "Create an account")
            .bind(&BankerService::createAccount, this)
            .produces(MIME(Application, Json))
            .consumes(MIME(Application, Json))
            .parameter<Rest::Type::String>("name", "The name of the account to create")
            .response(Http::Code::Ok, "The initial state of the account")
            .response(backendErrorResponse);

        auto accountPath = accountsPath.path("/:name");
        accountPath.parameter<Rest::Type::String>("name", "The name of the account to operate on");

        accountPath
            .route(desc.post("/budget"), "Add budget to the account")
            .bind(&BankerService::creditAccount, this)
            .produces(MIME(Application, Json))
            .response(Http::Code::Ok, "Budget has been added to the account")
            .response(backendErrorResponse);
    }

    void retrieveAllAccounts(const Rest::Request&, Http::ResponseWriter response)
    {
        response.send(Http::Code::Ok, "No Account");
    }

    void retrieveAccount(const Rest::Request&, Http::ResponseWriter response)
    {
        response.send(Http::Code::Ok, "The bank is closed, come back later");
    }

    void createAccount(const Rest::Request&, Http::ResponseWriter response)
    {
        response.send(Http::Code::Ok, "The bank is closed, come back later");
    }

    void creditAccount(const Rest::Request&, Http::ResponseWriter response)
    {
        response.send(Http::Code::Ok, "The bank is closed, come back later");
    }

    std::shared_ptr<Http::Endpoint> httpEndpoint;
    Rest::Description desc;
    Rest::Router router;
};

int main(int argc, char* argv[])
{
    Port port(9080);

    int thr = 2;

    if (argc >= 2)
    {
        port = static_cast<uint16_t>(std::stol(argv[1]));

        if (argc == 3)
            thr = std::stoi(argv[2]);
    }

    Address addr(Ipv4::any(), port);

    std::cout << "Cores = " << hardware_concurrency() << std::endl;
    std::cout << "Using " << thr << " threads" << std::endl;

    BankerService banker(addr);

    banker.init(thr);
    banker.start();
}
