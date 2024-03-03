---
id: http-handler
title: HTTP handler
---

<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani
SPDX-FileCopyrightText: 2021 Andrea Pappacoda

SPDX-License-Identifier: Apache-2.0
-->

Requests that are received by Pistache are handled by a special class called `Http::Handler`. This class declares a bunch of virtual methods that can be overriden to handle special events that occur on the socket and/or connection.

The `onRequest()` function must be overriden. This function is called whenever Pistache received data and correctly parsed it as an HTTP request.

```cpp
virtual void onRequest(const Http::Request& request, Http::ResponseWriter response);
```

The first argument is an object of type `Http::Request` representing the request itself. It contains a bunch of informations including:

- The resource associated to the request
- The query parameters
- The headers
- The body of the request

The `Request` object gives a read-only access to these informations. You can access them through a couple of getters but can not modify them. An HTTP request is **immutable**.

## Sending a response

`ResponseWriter` is an object from which the final HTTP response is sent to the client. The `onRequest()` function does not return anything (`void`). Instead, the response is sent through the `ResponseWriter` class. This class provides a bunch of `send()` function overloads to send the response:

```cpp
Async::Promise<ssize_t> send(Code code);
```

You can use this overload to send a response with an empty body and a given HTTP Code (e.g `Http::Code::Ok`)

```cpp
Async::Promise<ssize_t> send(
    Code code,
    const std::string& body,
    const Mime::MediaType &mime = Mime::MediaType()
);
```

This overload can be used to send a response with static, fixed-size content (body). A MIME type can also be specified, which will be sent through the `Content-Type` header.

```cpp
template<size_t N>
Async::Promise<ssize_t> send(
    Code code,
    const char (&arr)[N],
    const Mime::MediaType& mime = Mime::MediaType()
);
```

This version can also be used to send a fixed-size response with a body except that it does not need to construct a string (no memory is allocated). The size of the content is directly deduced by the compiler. This version only works with raw string literals.

These functions are asynchronous, meaning that they do not return a plain old `ssize_t` value indicating the number of bytes being sent, but instead a `Promise` that will be fulfilled later on. See the next section for more details on asynchronous programming with Pistache.

## Response streaming

Sometimes, content that is to be sent back to the user can not be known in advance, thus the length can not be determined in advance. For that matter, the HTTP specification defines a special data-transfer mechanism called [chunked encoding](https://tools.ietf.org/html/rfc7230#section-4.1) where data is sent in a series of _chunks_. This mechanism uses the `Transfer-Encoding` HTTP header in place of the `Content-Length` one.

To stream content, Pistache provides a special `ResponseStream` class. To get a `ResponseStream` from a `ResponseWriter`, call the `stream()` member function:

```cpp
auto stream = response.stream(Http::Code::Ok);
```

To initate a stream, you have to pass the HTTP status code to the stream function (here `Http::Code::Ok` or `HTTP 200`). The `ResponseStream` class provides an `iostream` like interface that overloads the `<<` operator.

```cpp
stream << "PO"
stream << "NG"
```

The first line will write a chunk of size 2 with the content _PO_ to the stream's buffer. The second line will write a second chunk of size 2 with the content _NG_. To end the stream and flush the content, use the special `ends` marker:

```cpp
stream << ends
```

The `ends` marker will write the last chunk of size 0 and send the final data over the network. To simply flush the stream's buffer without ending the stream, you can use the `flush` marker:

```cpp
stream << flush
```

:::caution Headers writing

After starting a stream, headers become immutable. They must be written to the response before creating a `ResponseStream`:

```cpp
response.headers()
    .add<Header::Server>("lys")
    .add<Header::ContentType>(MIME(Text, Plain));

auto stream = response.stream();
stream << "PO" << "NG" << ends;
```

:::

## Static file serving

In addition to text content serving, Pistache provides a way to serve static files through the `Http::serveFile` function:

```cpp
if (request.resource() == "/doc" && request.method() == Http::Method::Get) {
    Http::serveFile(response, "README.md");
}
```

:::note Return value

`serveFile` also returns a `Promise` representing the total number of bytes being sent to the wire

:::

## Controlling timeout

Sometimes, you might require to timeout after a certain amount of time. For example, if you are designing an HTTP API with soft real-time constraints, you will have a time constraint to send a response back to the client. That is why Pistache provides the ability to control the timeout on a per-request basis. To arm a timeout on a response, you can use the `timeoutAfter()` member function directly on the `ResponseWriter` object:

```cpp
response.timeoutAfter(std::chrono::milliseconds(500));
```

This will trigger a timeout if a response has not been sent within 500 milliseconds. `timeoutAfter` accepts any kind of duration.

When a timeout triggers, the `onTimeout()` function from your handler will be called. By default, this method does nothing. If you want to handle your timeout properly, you should then override this function inside your own handler:

```cpp
void onTimeout(const Http::Request& request, Http::ResponseWriter writer) {
    request.send(Http::Code::No_Content);
}
```

The `Request` object that is passed to the `onTimeout` is the exact same request that triggered the timeout. The `ResponseWriter` is a complete new writer object.

:::note ResponseWriter state

Since the `ResponseWriter` object is a complete new object, state is not preserved with the `ResponseWriter` from the `onRequest()` callback, which means that you will have to write the complete response again, including headers and cookies.

:::
