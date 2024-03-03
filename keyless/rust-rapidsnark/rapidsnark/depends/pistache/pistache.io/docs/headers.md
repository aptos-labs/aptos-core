---
title: Headers
---

<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani
SPDX-FileCopyrightText: 2021 Andrea Pappacoda

SPDX-License-Identifier: Apache-2.0
-->

## Overview

Inspired by the [Rust](https://www.rust-lang.org) eco-system and [Hyper](https://hyper.rs), HTTP headers are represented as _type-safe_ plain objects. Instead of representing headers as a pair of `(key: string, value: value)`, the choice has been made to represent them as plain objects. This greatly reduces the risk of typo errors that can not catched by the compiler with plain old strings.

Instead, objects give the compiler the ability to catch errors directly at compile-time, as the user can not add or request a header through its name: it has to use the whole **type**. Types being enforced at compile-time, it helps reducing common typo errors.

With Pistache, each HTTP Header is a class that inherits from the `Http::Header` base class and use the `NAME()` macro to define the name of the header. List of all headers inside an HTTP request or response are stored inside an internal [`std::unordered_map`](https://en.cppreference.com/w/cpp/container/unordered_map), wrapped in an `Header::Collection` class. Invidual headers can be retrieved or added to this object through the whole type of the header:

```cpp
auto headers = request.headers();
auto ct = headers.get<Http::Header::ContentType>();
```

`get<H>` will return a `std::shared_ptr<H>` where `H: Header` (`H` inherits from `Header`). If the header does not exist, `get<H>` will throw an exception. `tryGet<H>` provides a non-throwing alternative that, instead, returns a null pointer.

:::note Built-in headers

Headers provided by Pistache live in the `Http::Header` namespace

:::

## Defining your own header

Common headers defined by the HTTP RFC ([RFC2616](https://pretty-rfc.herokuapp.com/RFC2616)) are already implemented and available. However, some APIs might define extra headers that do not exist in Pistache. To support your own header types, you can define and register your own HTTP Header by first declaring a class that inherits the `Http::Header` class:

```cpp
class XProtocolVersion : public Http::Header {
};
```

Since every header has a name, the `NAME()` macro must be used to name the header properly:

```cpp
class XProtocolVersion : public Http::Header {
    NAME("X-Protocol-Version")
};
```

The `Http::Header` base class provides two virtual methods that you must override in your own implementation:

```cpp
void parse(const std::string& data);
```

This function is used to parse the header from the string representation. Alternatively, to avoid allocating memory for the string representation, a _raw_ version can be used:

```cpp
void parseRaw(const char* str, size_t len);
```

`str` will directly point to the header buffer from the raw http stream. The len parameter is the total length of the header's value.

```cpp
void write(std::ostream& stream) const
```

When writing the response back to the client, the `write` function is used to serialize the header into the network buffer.

Let’s combine these functions together to finalize the implementation of our previously declared header:

```cpp
class XProtocolVersion : public Http::Header {
public:

    NAME("X-Protocol-Version")

    XProtocolVersion()
     : minor(-1)
     , major(-1)
    { }

    void parse(const std::string& data) {
        auto pos = data.find('.');
        if (pos != std::string::npos) {
            minor = std::stoi(data.substr(0, pos));
            major = std::stoi(data.substr(pos + 1));
        }
    }

    void write(std::ostream& os) const {
        os << minor << "." << major;
    }
private:
    int minor;
    int major;
};
```

And that’s it. Now all we have to do is registering the header to the registry system:

```cpp
Header::Registry::registerHeader<XProtocolVersion>();
```

:::note Header instantation

You should always provide a default constructor for your header so that it can be instantiated by the registry system

:::

Now, the `XProtocolVersion` can be retrieved and added like any other header in the `Header::Collection` class.

:::note Unknown headers

Headers that are not known to the registry system are stored as a raw pair of strings in the `Collection` class. `getRaw()` can be used to retrieve a raw header:

```cpp
auto myHeader = request.headers().getRaw("x-raw-header");
myHeader.name() // x-raw-header
myHeader.value() // returns the value of the header as a string
```

:::

## MIME types

[MIME Types](https://en.wikipedia.org/wiki/Media_type) (or Media Type) are also fully typed. Such types are for example used in an HTTP request or response to describe the data contained in the body of the message (`Content-Type` header, …) and are composed of a _type_, _subtype_, and optional _suffix_ and parameters.

MIME Types are represented by the `Mime::MediaType` class, implemented in the `mime.h` header. A MIME type can be directly constructed from a string:

```cpp
auto mime = Http::Mime::MediaType::fromString("application/json");
```

However, to enforce type-safety, common types are all represented as enumerations:

```cpp
Http::Mime::MediaType m1(Http::Mime::Type::Application, Http::Mime::Subtype::Json);
```

To avoid such a typing pain, a `MIME` macro is also provided:

```cpp
auto m1 = MIME(Application, Json);
```

For suffix MIMEs, use the special `MIME3` macro:

```cpp
auto m1 = MIME3(Application, Json, Zip);
```

If you like typing, you can also use the long form:

```cpp
Http::Mime::MediaType m1(Http::Mime::Type::Application, Http::Mime::Subtype::Json, Http::Mime::Suffix::Zip);
```

The `toString()` function can be used to get the string representation of a given MIME type:

```cpp
auto m1 = MIME(Text, Html);
m1.toString(); // text/html
```
