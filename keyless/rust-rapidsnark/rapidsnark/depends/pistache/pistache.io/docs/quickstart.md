---
title: Getting started
slug: /
---

<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani
SPDX-FileCopyrightText: 2021 Andrea Pappacoda

SPDX-License-Identifier: Apache-2.0
-->

Pistache is a web framework written in Modern C++ that focuses on performance and provides an elegant and asynchronous API.

```cpp
#include <pistache/pistache.h>
```

## Installing Pistache

[git](https://git-scm.com) is needed to retrieve the sources. Compiling the sources will require [CMake](https://cmake.org) to generate build files and a recent compiler that supports C++17.

If you're on Ubuntu and want to skip the compilation process you can add the official PPA providing nightly builds:

```shell
sudo add-apt-repository ppa:pistache+team/unstable
sudo apt update
sudo apt install libpistache-dev
```

Otherwise, here's how to build and install the latest release:

```shell
git clone https://github.com/pistacheio/pistache.git
cd pistache
meson setup build
meson install -C build
```

Also, Pistache does not support Windows yet, but should work fine under [WSL](https://docs.microsoft.com/windows/wsl/about).

## Serving requests

### Include

First, let’s start by including the right header.

```cpp
#include <pistache/endpoint.h>
```

### Hello world

Requests received by Pistache are handled with an `Http::Handler`.

Let’s start by defining a simple `HelloHandler`:

```cpp
using namespace Pistache;

class HelloHandler : public Http::Handler {
public:

    HTTP_PROTOTYPE(HelloHandler)

    void onRequest(const Http::Request& request, Http::ResponseWriter response) {
        response.send(Http::Code::Ok, "Hello, World\n");
    }
};
```

Handlers must inherit the `Http::Handler` class and at least define the `onRequest` member function. They must also define a `clone()` member function. Simple handlers can use the special `HTTP_PROTOTYPE` macro, passing in the name of the class. The macro will take care of defining the `clone()` member function for you.

### Final touch

After defining the handler, the server can now be started:

```cpp
int main() {
    Address addr(Ipv4::any(), Port(9080));

    auto opts = Http::Endpoint::options().threads(1);
    Http::Endpoint server(addr);
    server.init(opts);
    server.setHandler(Http::make_handler<HelloHandler>());
    server.serve();
}
```

For simplicity, you can also use the special `listenAndServe` function that will automatically create an endpoint and instantiate your handler:

```cpp
int main() {
    Http::listenAndServe<HelloHandler>("*:9080");
}
```

And that’s it, now you can fire up your favorite curl request and observe the final result:

```shell
curl http://localhost:9080/
Hello, World
```

Complete code for this example can be found on GitHub: [examples/hello_server.cc](https://github.com/pistacheio/pistache/blob/master/examples/hello_server.cc)
