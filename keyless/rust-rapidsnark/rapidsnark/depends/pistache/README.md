<!--
SPDX-FileCopyrightText: 2016 Mathieu Stefani

SPDX-License-Identifier: Apache-2.0
-->

# Pistache

[![N|Solid](pistache.io/static/img/logo.png)](https://www.github.com/pistacheio/pistache)

[![linux](https://github.com/pistacheio/pistache/actions/workflows/linux.yaml/badge.svg)](https://github.com/pistacheio/pistache/actions/workflows/linux.yaml)
[![autopkgtest](https://github.com/pistacheio/pistache/actions/workflows/autopkgtest.yaml/badge.svg)](https://github.com/pistacheio/pistache/actions/workflows/autopkgtest.yaml)
[![codecov](https://codecov.io/gh/pistacheio/pistache/branch/master/graph/badge.svg)](https://codecov.io/gh/pistacheio/pistache)
[![REUSE status](https://api.reuse.software/badge/github.com/pistacheio/pistache)](https://api.reuse.software/info/github.com/pistacheio/pistache)

Pistache is a modern and elegant HTTP and REST framework for C++. It is entirely written in pure-C++17[*](#linux-only) and provides a clear and pleasant API.

## Documentation

We are still looking for a volunteer to document fully the API. In the mean time, partial documentation is available at [pistacheio.github.io/pistache/](https://pistacheio.github.io/pistache/). If you are interested in helping with this, please open an issue ticket.

A comparison of Pistache to other C++ RESTful APIs was created by guteksan and is available [here](https://github.com/guteksan/REST-CPP-benchmark).

## Dependencies

Pistache has the following third party dependencies

- [Meson](https://mesonbuild.com)
- [Doxygen](https://www.doxygen.nl/)
- [Googletest](https://github.com/google/googletest)
- [OpenSSL](https://www.openssl.org/)
- [RapidJSON](https://rapidjson.org/)
- [Hinnant Date](https://github.com/HowardHinnant/date)

## Contributing

Pistache is released under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). Contributors are welcome!

Pistache was originally created by Mathieu Stefani (`@octal`). He continues to contribute to the maintainence and development of Pistache, supported by a team of volunteers. The maintainers can be reached  in `#pistache` on [Libera.Chat](https://libera.chat/) (ircs://irc.libera.chat:6697). Please come and join us!

The [Launchpad Team](https://launchpad.net/~pistache+team) administers the daily and stable Ubuntu pre-compiled packages.

### Versioning

The version of the library's public interface (ABI) is not the same as the release version, but we choose to always guarantee that the major release version and the soname version will match. The interface version is primarily associated with the _external_ interface of the library. Different platforms handle this differently, such as AIX, GNU/Linux, and Solaris.

GNU Libtool abstracts each platform's idiosyncrasies away because it is more portable than using `ar(1)` or `ranlib(1)` directly. However, it is [not supported in Meson](https://mesonbuild.com/FAQ.html#how-do-i-do-the-equivalent-of-libtools-exportsymbol-and-exportregex) so we made do without it by setting the SONAME directly.

When Pistache is installed it will normally ship:

- `libpistache.so.X.Y.Z`: This is the actual shared-library binary file. The _X_, _Y_ and _Z_ values are the major, minor and patch interface versions respectively.

- `libpistache.so.X`: This is the _soname_ soft link that points to the binary file. It is what other programs and other libraries reference internally. You should never need to directly reference this file in your build environment.

- `libpistache.so`: This is the _linker name_ entry. This is also a soft link that refers to the soname with the highest major interface version. This linker name is what is referred to on the linker command line.

- `libpistache.a`: This is the _static archive_ form of the library. Since when using a static library all of its symbols are normally resolved before runtime, an interface version in the filename is unnecessary.

If your contribution has modified the interface, you may need to update the major or minor interface versions. Otherwise user applications and build environments will eventually break. This is because they will attempt to link against an incorrect version of the library -- or worse, link correctly but with undefined runtime behaviour.

The major version should be incremented when you make incompatible API or ABI changes. The minor version should be incremented when you add functionality in a backwards compatible manner. The patch version should be incremented when you make backwards compatible bug fixes. This can be done by modifying `version.txt` accordingly. Also remember to always update the commit date in the aformentioned file.

## Precompiled Packages

If you have no need to modify the Pistache source, you are strongly recommended to use precompiled packages for your distribution. This will save you time.

### Debian and Ubuntu

We have submitted a [Request for Packaging](https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=929593) downstream to Debian. Once we have an official Debian package maintainer intimately familiar with the [Debian Policy Manual](https://www.debian.org/doc/debian-policy/), we can expect to eventually see it become available in Debian and all derivatives (e.g. Ubuntu and many others).

But until then currently Pistache has partially compliant upstream Debianization. Our long term goal is to have our source package properly Debianized downstream by a Debian Policy Manual SME. In the mean time consider using our PPAs to avoid having to build from source.

#### Supported Architectures

Currently Pistache is built and tested on a number of [architectures](https://wiki.debian.org/SupportedArchitectures). Some of these are suitable for desktop or server use and others for embedded environments. As of this writing we do not currently have any MIPS related packages that have been either built or tested.

- amd64
- arm64
- armhf
- i386
- ppc64el
- riscv64
- s390x

#### Ubuntu PPA (Unstable)

The project builds [daily unstable snapshots](https://launchpad.net/~pistache+team/+archive/ubuntu/unstable) in a separate unstable PPA. To use it, run the following:

```sh
$ sudo add-apt-repository ppa:pistache+team/unstable
$ sudo apt update
$ sudo apt install libpistache-dev
```

#### Ubuntu PPA (Stable)

Currently there are no stable release of Pistache published into the [stable](https://launchpad.net/~pistache+team/+archive/ubuntu/stable) PPA. However, when that time comes, run the following to install a stable package:

```sh
$ sudo add-apt-repository ppa:pistache+team/stable
$ sudo apt update
$ sudo apt install libpistache-dev
```

### Other Distributions

Package maintainers, please insert instructions for users to install pre-compiled packages from your respective repositories here.

## Use via pkg-config

If you would like to automatically have your project's build environment use the appropriate compiler and linker build flags, [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/) can greatly simplify things. It is the portable international _de facto_ standard for determining build flags. The development packages include a pkg-config manifest.

### GNU Autotools

To [use](https://autotools.io/pkgconfig/pkg_check_modules.html) with the GNU Autotools, as an example, include the following snippet in your project's `configure.ac`:

```makefile
# Pistache...
PKG_CHECK_MODULES(
    [libpistache], [libpistache >= 0.0.2], [],
    [AC_MSG_ERROR([libpistache >= 0.0.2 missing...])])
YOURPROJECT_CXXFLAGS="$YOURPROJECT_CXXFLAGS $libpistache_CFLAGS"
YOURPROJECT_LIBS="$YOURPROJECT_LIBS $libpistache_LIBS"
```

### Meson

To use with Meson, you just need to add `dependency('libpistache')` as a dependency for your executable.

```meson
project(
    'MyPistacheProject',
    'cpp',
    meson_version: '>=0.55.0'
)

executable(
    'MyPistacheExecutable',
    sources: 'main.cpp',
    dependencies: dependency('libpistache')
)
```

If you want to build the library from source in case the dependency is not found on the system, you can add this repository as a submodule in the `subprojects` directory of your project, and edit the `dependency()` call as follows:

```meson
dependency('libpistache', fallback: 'pistache')
```

If you're using a Meson version older than 0.55.0 you'll have to use the "older" syntax for `dependency()`:

```meson
dependency('libpistache', fallback: ['pistache', 'pistache_dep'])
```

Lastly, if you'd like to build the fallback as a static library you can specify it with the `default_options` keyword:

```meson
dependency('libpistache', fallback: 'pistache', default_options: 'default_library=static')
```

### CMake

To use with a CMake build environment, use the [FindPkgConfig](https://cmake.org/cmake/help/latest/module/FindPkgConfig.html) module. Here is an example:

```cmake
cmake_minimum_required(VERSION 3.6)
project("MyPistacheProject")

find_package(PkgConfig)
pkg_check_modules(Pistache REQUIRED IMPORTED_TARGET libpistache)

add_executable(${PROJECT_NAME} main.cpp)
target_link_libraries(${PROJECT_NAME} PkgConfig::Pistache)
```

### Makefile

To use within a vanilla makefile, you can call `pkg-config` directly to supply compiler and linker flags using shell substitution.

```makefile
CFLAGS=-g3 -Wall -Wextra -Werror ...
LDFLAGS=-lfoo ...
...
CFLAGS+= $(pkg-config --cflags libpistache)
LDFLAGS+= $(pkg-config --libs libpistache)
```

## Building from source

To download the latest available release, clone the repository over GitHub.

```sh
$ git clone https://github.com/pistacheio/pistache.git
```

Now, compile the sources:

```sh
$ cd pistache
$ meson setup build \
    --buildtype=release \
    -DPISTACHE_USE_SSL=true \
    -DPISTACHE_BUILD_EXAMPLES=true \
    -DPISTACHE_BUILD_TESTS=true \
    -DPISTACHE_BUILD_DOCS=false \
    --prefix="$PWD/prefix"
$ meson compile -C build
$ meson install -C build
```

Optionally, you can also run the tests. You can skip tests requiring network access with `--no-suite=network`:

```sh
$ meson test -C build
```

Be patient, async_test can take some time before completing. And that's it, now you can start playing with your newly installed Pistache framework.

Some other Meson options:

| Option                        | Default | Description                                    |
| ----------------------------- | ------- | ---------------------------------------------- |
| PISTACHE_USE_SSL              | False   | Build server with SSL support                  |
| PISTACHE_BUILD_TESTS          | False   | Build all of the unit tests                    |
| PISTACHE_BUILD_EXAMPLES       | False   | Build all of the example apps                  |
| PISTACHE_BUILD_DOCS           | False   | Build Doxygen docs                             |

## Example

### Hello World (server)

```cpp
#include <pistache/endpoint.h>

using namespace Pistache;

struct HelloHandler : public Http::Handler {
  HTTP_PROTOTYPE(HelloHandler)
  void onRequest(const Http::Request&, Http::ResponseWriter writer) override {
    writer.send(Http::Code::Ok, "Hello, World!");
  }
};

int main() {
  Http::listenAndServe<HelloHandler>(Pistache::Address("*:9080"));
}
```

## Tutorials

* [Adding a REST API with Pistache](https://www.youtube.com/watch?v=9BCO5W_Kw3Q), Utah Cpp Programmers, 20 July 2022.

## Project status

Pistache hasn't yet hit the 1.0 release. This means that the project is _unstable_ but not _unusable_. In fact, most of the code is production ready; you can use Pistache to develop a RESTful API without issues, but the HTTP client has a few issues in it that make it buggy.

<b id="linux-only">\*</b> While most code uses modern C++, Pistache makes use of some Linux-specific APIs where the standard library doesn't provide alternatives, and works only on that OS. See [#6](https://github.com/pistacheio/pistache/issues/6#issuecomment-242398225) for details. If you know how to help, please contribute a PR to add support for your desired platform :)
