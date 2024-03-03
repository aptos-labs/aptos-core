---
title: Routing
---

<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani
SPDX-FileCopyrightText: 2021 Andrea Pappacoda

SPDX-License-Identifier: Apache-2.0
-->

HTTP routing consists of binding an HTTP route to a C++ callback. A special component called an HTTP router will be in charge of dispatching HTTP requests to the right C++ callback. A route is composed of an HTTP verb associated to a resource:

```javascript
GET /users/1
```

Here, `GET` is the verb and `/users/1` is the associated resource.

## HTTP methods

A bunch of HTTP methods (verbs) are supported by Pistache:

- _GET_: The `GET` method is used by the client (e.g browser) to retrieve a resource identified by an URI. For example, to retrieve an user identified by an id, a client will issue a `GET` to the `/users/:id` Request-URI.
- _POST_: the `POST` method is used to post or send new information to a certain resource. The server will then read and store the data associated to the request. `POST` is a common way of transmitting data from an HTML form. `POST` can also be used to create a new resource or update information of an existing resource. For example, to create a new user, a client will issue a `POST` to the `/users` path with the data of the user to create in its body.
- _PUT_: `PUT` is very similar to `POST` except that `PUT` is idempotent, meaning that two requests to the same Request-URI with the same identical content should have the same effect and should produce the same result.
- _DELETE_: the `DELETE` method is used to delete a resource associated to a given Request-URI. For example, to remove an user, a client might issue a `DELETE` call to the `/users/:id` Request-URI.

To sum up, `POST` and `PUT` are used to Create and/or Update, `GET` is used to Read and `DELETE` is used to Delete information.

## Route patterns

### Static routes

Static routes are the simplest ones as they do rely on dynamic parts of the Request-URI. For example `/users/all` is a static route that will exactly match the `/users/all` Request-URI.

### Dynamic routes

However, it is often useful to define routes that have dynamic parts. For example, to retrieve a specific user by its id, the id is needed to query the storage. Dynamic routes thus have parameters that are then matched one by one by the HTTP router. In a dynamic route, parameters are identified by a column `:`

`/users/:id`

Here, `:id` is a dynamic parameter. When a request comes in, the router will try to match the `:id` parameter to the corresponding part of the request. For example, if the server receives a request to `/users/13`, the router will match the `13` value to the `:id` parameter.

Some parameters, like `:id` are named. However, Pistache also allows _splat_ (wildcard) parameters, identified by a star `*`:

`/link/*/to/*`

## Defining routes

To define your routes, you first have to instantiate an HTTP router:

```cpp
Http::Router router;
```

Then, use the `Routes::<Method>()` functions to add some routes:

```cpp
Routes::Get(router, "/users/all", Routes::bind(&UsersApi::getAllUsers, this));
Routes::Post(router, "/users/:id", Routes::bind(&UsersApi::getUserId, this));
Routes::Get(router, "/link/*/to/*", Routes::bind(&UsersApi::linkUsers, this));
```

`Routes::bind` is a special function that will generate a corresponding C++ callback that will then be called by the router if a given route matches the Request-URI.

### Callbacks

A C++ callback associated to a route must have the following signature:

```cpp
void(const Rest::Request&, Http::ResponseWriter);
```

A callback can either be a non-static free or member function. For member functions, a pointer to the corresponding instance must be passed to the Routes::bind function so that the router knows on which instance to invoke the member function.

The first parameter of the callback is `Rest::Request` and not an `Http::Request`. A `Rest::Request` is an `Http::Request` with additional functions. Named and splat parameters are for example retrieved through this object:

```cpp
void UsersApi::getUserId(const Rest::Request& request, Http::ResponseWriter response) {
    auto id = request.param(":id").as<int>();
    // ...
}

void UsersApi::linkUsers(const Rest::Request& request, Http::ResponseWriter response) {
    auto u1 = request.splatAt(0).as<std::string>();
    auto u2 = request.splatAt(1).as<std::string>();
    // ...
}
```

As you can see, parameters are also typed. To cast a parameter to the appropriate type, use the `as<T>` member template.

:::note Cast safety

An exception will be thrown if the parameter can not be casted to the right type

:::

### Installing the handler

Once the routes have been defined, the final `Http::Handler` must be set to the HTTP Endpoint. To retrieve the handler, just call the `handler()` member function on the router object:

```cpp
endpoint.setHandler(router.handler());
```
