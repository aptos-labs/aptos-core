---
title: Asynchronous HTTP programming
---

<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani
SPDX-FileCopyrightText: 2021 Andrea Pappacoda

SPDX-License-Identifier: Apache-2.0
-->

Interfaces provided by Pistaches are _asynchronous_ and _non-blocking_. Asynchronous programming allows for code to continue executing even if the result of a given call is not available yet. Calls that provide an asynchronous interface are referred to _asynchronous calls_.

An example of such a call is the `send()` function provided by the `ResponseWriter` interface. This function returns the number of bytes written to the socket file descriptor associated to the connection. However, instead of returning directly the value to the caller and thus blocking the caller, it wraps the value into a component called a `Promise`.

A `Promise` is the Pistache’s implementation of the [Promises/A+](https://promisesaplus.com) standard available in many JavaScript implementations. Simply put, during an asynchronous call, a `Promise` separates the launch of an asynchronous operation from the retrieval of its result. While the asynchronous might still be running, a `Promise<T>` is directly returned to the caller to retrieve the final result when it becomes available. A so called continuation can be attach to a `Promise` to execute a callback when the result becomes available (when the `Promise` has been resolved or fulfilled).

```cpp
auto res = response.send(Http::Code::Ok, "Hello World");
res.then(
    [](ssize_t bytes) { std::cout << bytes << " bytes have been sent\n" },
    Async::NoExcept
);
```

The `then()` member is used to attach a callback to the `Promise`. The first argument is a callable that will be called when the `Promise` has been **succesfully** resolved. If, for some reason, an error occurs during the asynchronous operation, a `Promise` can be **rejected** and will then fail. In this case, the second callable will be called. `Async::NoExcept` is a special callback that will call [`std::terminate()`](https://en.cppreference.com/w/cpp/error/terminate) if the promise failed. This is the equivalent of the `noexcept` keyword.

Other generic callbacks can also be used in this case:

- `Async::IgnoreException` will simply ignore the exception and let the program continue
- `Async::Throw` will "rethrow" the exception up to an eventual promise call-chain. This has the same effect than the `throw` keyword, except that it is suitable for promises

Exceptions in promises callbacks are propagated through an `exception_ptr`. Promises can also be chained together to create a whole asynchronous pipeline:

```cpp
auto fetchOp = fetchDatabase();
fetchOp
    .then(
        [](const User& user) { return fetchUserInfo(user); },
        Async::Throw)
    .then(
        [](const UserInfo& info) { std::cout << "User name = " << info.name << '\n'; },
        [](exception_ptr ptr) { std::cerr << "An exception occured during user retrieval\n";}
);
```

Line 5 will propagate the exception if `fetchDatabase()` failed and rejected the promise.
