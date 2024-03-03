#!/usr/bin/env bash

set -e

NPROC=8
fetch_cmd=$( (type wget > /dev/null 2>&1 && echo "wget") || echo "curl -O" )

usage()
{
    echo "USAGE: $0 <target>"
    echo "where target is one of:"
    echo "    ios:            build for iOS arm64"
    echo "    ios_simulator:  build for iPhone Simulator for arm64/x86_64 (fat binary)"
    echo "    macos:          build for macOS for arm64/x86_64 (fat binary)"
    echo "    macos_arm64:    build for macOS arm64"
    echo "    macos_x86_64:   build for macOS x86_64"
    echo "    android:        build for Android arm64"
    echo "    android_x86_64: build for Android x86_64"
    echo "    host:           build for this host"
    echo "    host_noasm:     build for this host without asm optimizations (e.g. needed for macOS)"
    echo "    aarch64:        build for Linux aarch64"

    exit 1
}

get_gmp()
{
    GMP_NAME=gmp-6.2.1
    GMP_ARCHIVE=${GMP_NAME}.tar.xz
    GMP_URL=https://ftp.gnu.org/gnu/gmp/${GMP_ARCHIVE}

    if [ ! -f ${GMP_ARCHIVE} ]; then

        $fetch_cmd ${GMP_URL}
    fi


    if [ ! -d gmp ]; then

        tar -xvf ${GMP_ARCHIVE}
        mv ${GMP_NAME} gmp
    fi
}

build_aarch64()
{
    PACKAGE_DIR="$GMP_DIR/package_aarch64"
    BUILD_DIR=build_aarch64

    if [ -d "$PACKAGE_DIR" ]; then
        echo "aarch64 package is built already. See $PACKAGE_DIR"
        return 1
    fi


    export TARGET=aarch64-linux-gnu

    echo $TARGET

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --host $TARGET --prefix="$PACKAGE_DIR" --with-pic --disable-fft &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_host()
{
    PACKAGE_DIR="$GMP_DIR/package"
    BUILD_DIR=build

    if [ -d "$PACKAGE_DIR" ]; then
        echo "Host package is built already. See $PACKAGE_DIR"
        return 1
    fi

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --prefix="$PACKAGE_DIR" --with-pic --disable-fft &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_host_noasm()
{
    PACKAGE_DIR="$GMP_DIR/package"
    BUILD_DIR=build

    if [ -d "$PACKAGE_DIR" ]; then
        echo "Host package is built already. See $PACKAGE_DIR"
        return 1
    fi

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --prefix="$PACKAGE_DIR" --with-pic --disable-fft --disable-assembly &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_android()
{
    PACKAGE_DIR="$GMP_DIR/package_android_arm64"
    BUILD_DIR=build_android_arm64

    if [ -d "$PACKAGE_DIR" ]; then
        echo "Android package is built already. See $PACKAGE_DIR"
        return 1
    fi

    if [ -z "$ANDROID_NDK" ]; then

        echo "ERROR: ANDROID_NDK environment variable is not set."
        echo "       It must be an absolute path to the root directory of Android NDK."
        echo "       For instance /home/test/Android/Sdk/ndk/23.1.7779620"
        return 1
    fi

    export TOOLCHAIN=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64

    export TARGET=aarch64-linux-android
    export API=21

    export AR=$TOOLCHAIN/bin/llvm-ar
    export CC=$TOOLCHAIN/bin/$TARGET$API-clang
    export AS=$CC
    export CXX=$TOOLCHAIN/bin/$TARGET$API-clang++
    export LD=$TOOLCHAIN/bin/ld
    export RANLIB=$TOOLCHAIN/bin/llvm-ranlib
    export STRIP=$TOOLCHAIN/bin/llvm-strip

    echo "$TOOLCHAIN"
    echo "$TARGET"

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --host $TARGET --prefix="$PACKAGE_DIR" --with-pic --disable-fft &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_android_x86_64()
{
    PACKAGE_DIR="$GMP_DIR/package_android_x86_64"
    BUILD_DIR=build_android_x86_64

    if [ -d "$PACKAGE_DIR" ]; then
        echo "Android package is built already. See $PACKAGE_DIR"
        return 1
    fi

    if [ -z "$ANDROID_NDK" ]; then

        echo "ERROR: ANDROID_NDK environment variable is not set."
        echo "       It must be an absolute path to the root directory of Android NDK."
        echo "       For instance /home/test/Android/Sdk/ndk/23.1.7779620"
        return 1
    fi

    export TOOLCHAIN=$ANDROID_NDK/toolchains/llvm/prebuilt/linux-x86_64

    export TARGET=x86_64-linux-android
    export API=21

    export AR=$TOOLCHAIN/bin/llvm-ar
    export CC=$TOOLCHAIN/bin/$TARGET$API-clang
    export AS=$CC
    export CXX=$TOOLCHAIN/bin/$TARGET$API-clang++
    export LD=$TOOLCHAIN/bin/ld
    export RANLIB=$TOOLCHAIN/bin/llvm-ranlib
    export STRIP=$TOOLCHAIN/bin/llvm-strip

    echo "$TOOLCHAIN"
    echo $TARGET

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --host $TARGET --prefix="$PACKAGE_DIR" --with-pic --disable-fft &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_ios()
{
    PACKAGE_DIR="$GMP_DIR/package_ios_arm64"
    BUILD_DIR=build_ios_arm64

    if [ -d "$PACKAGE_DIR" ]; then
        echo "iOS package is built already. See $PACKAGE_DIR"
        return 1
    fi

    export SDK="iphoneos"
    export TARGET=arm64-apple-darwin
    export MIN_IOS_VERSION=8.0

    export ARCH_FLAGS="-arch arm64 -arch arm64e"
    export OPT_FLAGS="-O3 -g3 -fembed-bitcode"
    HOST_FLAGS="${ARCH_FLAGS} -miphoneos-version-min=${MIN_IOS_VERSION} -isysroot $(xcrun --sdk ${SDK} --show-sdk-path)"

    CC=$(xcrun --find --sdk "${SDK}" clang)
    export CC
    CXX=$(xcrun --find --sdk "${SDK}" clang++)
    export CXX
    CPP=$(xcrun --find --sdk "${SDK}" cpp)
    export CPP
    export CFLAGS="${HOST_FLAGS} ${OPT_FLAGS}"
    export CXXFLAGS="${HOST_FLAGS} ${OPT_FLAGS}"
    export LDFLAGS="${HOST_FLAGS}"

    echo $TARGET

    rm -rf "$BUILD_DIR"
    mkdir "$BUILD_DIR"
    cd "$BUILD_DIR"

    ../configure --host $TARGET --prefix="$PACKAGE_DIR" --with-pic --disable-fft --disable-assembly &&
    make -j${NPROC} &&
    make install

    cd ..
}

build_ios_simulator()
{
	libs=()
	for ARCH in "arm64" "x86_64"; do
		case "$ARCH" in
			"arm64" )
				echo "Building for iPhone Simulator arm64"
				ARCH_FLAGS="-arch arm64 -arch arm64e"
				;;
			"x86_64" )
				echo "Building for iPhone Simulator x86_64"
				ARCH_FLAGS="-arch x86_64"
				;;
			* )
				echo "Incorrect iPhone Simulator arch"
				exit 1
		esac
		
		BUILD_DIR="build_iphone_simulator_${ARCH}"
		PACKAGE_DIR="$GMP_DIR/package_iphone_simulator_${ARCH}"
		libs+=("${PACKAGE_DIR}/lib/libgmp.a")

		if [ -d "$PACKAGE_DIR" ]; then
			echo "iPhone Simulator ${ARCH} package is built already. See $PACKAGE_DIR. Skip building this ARCH."
			continue
		fi

		rm -rf "$BUILD_DIR"
		mkdir "$BUILD_DIR"
		cd "$BUILD_DIR"

		../configure --prefix="${PACKAGE_DIR}" \
					 CC="$(xcrun --sdk iphonesimulator --find clang)" \
					 CFLAGS="-O3 -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) ${ARCH_FLAGS} -fvisibility=hidden -mios-simulator-version-min=8.0" \
					 LDFLAGS="" \
					 --host ${ARCH}-apple-darwin --disable-assembly --enable-static --disable-shared --with-pic &&
			make -j${NPROC} &&
			make install
		
		cd ..
	done

	mkdir -p "${GMP_DIR}/package_iphone_simulator/lib"
	lipo "${libs[@]}" -create -output "${GMP_DIR}/package_iphone_simulator/lib/libgmp.a"
	echo "Wrote universal fat library for iPhone Simulator arm64/x86_64 to ${GMP_DIR}/package_iphone_simulator/lib/libgmp.a"
}

build_macos_arch()
{
  ARCH="$1"
  case "$ARCH" in
    "arm64" )
      ARCH_FLAGS="-arch arm64 -arch arm64e"
      ;;
    "x86_64" )
      ARCH_FLAGS="-arch x86_64"
      ;;
    * )
      echo "Incorrect arch"
      exit 1
  esac

  BUILD_DIR="build_macos_${ARCH}"
  PACKAGE_DIR="$GMP_DIR/package_macos_${ARCH}"
  if [ -d "$PACKAGE_DIR" ]; then
    echo "macOS ${ARCH} package is built already. See $PACKAGE_DIR. Skip building this ARCH."
    return
  fi
  rm -rf "$BUILD_DIR"
  mkdir "$BUILD_DIR"
  cd "$BUILD_DIR"
  ../configure --prefix="${PACKAGE_DIR}" \
         CC="$(xcrun --sdk macosx --find clang)" \
         CFLAGS="-O3 -isysroot $(xcrun --sdk macosx --show-sdk-path) ${ARCH_FLAGS} -fvisibility=hidden -mmacos-version-min=14.0" \
         LDFLAGS="" \
         --host "${ARCH}-apple-darwin" --disable-assembly --enable-static --disable-shared --with-pic &&
    make -j${NPROC} &&
    make install
  cd ..
}

build_macos_fat()
{
  echo "Building for macOS arm64"
  build_macos_arch "arm64"
  echo "Building for macOS x86_64"
  build_macos_arch "x86_64"

  gmp_lib_arm64="$GMP_DIR/package_macos_arm64/lib/libgmp.a"
  gmp_lib_x86_64="$GMP_DIR/package_macos_x86_64/lib/libgmp.a"
  gmp_lib_fat="$GMP_DIR/package_macos/lib/libgmp.a"

	mkdir -p "${GMP_DIR}/package_macos/lib"
	lipo "${gmp_lib_arm64}" "${gmp_lib_x86_64}" -create -output "${gmp_lib_fat}"
	mkdir -p "${GMP_DIR}/package_macos/include"
	cp  "${GMP_DIR}/package_macos_arm64/include/gmp.h" "${GMP_DIR}/package_macos/include/"
	echo "Wrote universal fat library for macOS arm64/x86_64 to ${GMP_DIR}/package_macos/lib/libgmp.a"
}

if [ $# -ne 1 ]; then
    usage
fi

TARGET_PLATFORM=$(echo "$1" | tr "[:upper:]" "[:lower:]")

cd depends

get_gmp

cd gmp

GMP_DIR=$PWD

case "$TARGET_PLATFORM" in

    "ios" )
        echo "Building for ios"
        build_ios
    ;;

    "ios_simulator" )
        echo "Building for iPhone Simulator"
        build_ios_simulator
    ;;

    "macos" )
        echo "Building fat library for macOS"
        build_macos_fat
    ;;

    "macos_arm64" )
        echo "Building library for macOS arm64"
        build_macos_arch "arm64"
    ;;

    "macos_x86_64" )
        echo "Building library for macOS x86_64"
        build_macos_arch "x86_64"
    ;;

    "android" )
        echo "Building for android"
        build_android
    ;;

    "android_x86_64" )
        echo "Building for android x86_64"
        build_android_x86_64
    ;;

    "host" )
        echo "Building for this host"
        build_host
    ;;

    "host_noasm" )
        echo "Building for this host without asm optimizations (e.g. needed for macOS)"
        build_host_noasm
    ;;

    "aarch64" )
        echo "Building for linux aarch64"
        build_aarch64
    ;;

    * )
    usage

esac
