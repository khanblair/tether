#!/bin/sh
# Build the pinned Termux PRoot fork and its loader for the full arm64 flavor.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
# shellcheck disable=SC1091
. "$ROOT_DIR/third_party/proot/pins.sh"

NDK_VERSION=$(sed -n 's/^tether.ndkVersion=//p' "$ROOT_DIR/android/gradle.properties")
if [ -z "$NDK_VERSION" ]; then
    echo "tether.ndkVersion is missing from android/gradle.properties" >&2
    exit 1
fi

if [ -n "${NDK:-}" ]; then
    NDK_DIR=$NDK
elif [ -n "${ANDROID_NDK_HOME:-}" ]; then
    NDK_DIR=$ANDROID_NDK_HOME
else
    SDK_DIR=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
    if [ -z "$SDK_DIR" ]; then
        case $(uname -s) in
            Darwin) SDK_DIR=${HOME}/Library/Android/sdk ;;
            *) SDK_DIR=${HOME}/Android/Sdk ;;
        esac
    fi
    NDK_DIR="$SDK_DIR/ndk/$NDK_VERSION"
fi

case $(uname -s) in
    Darwin) HOST_TAG=darwin-x86_64 ;;
    Linux) HOST_TAG=linux-x86_64 ;;
    *) echo "unsupported host: $(uname -s)" >&2; exit 1 ;;
esac

TOOLCHAIN="$NDK_DIR/toolchains/llvm/prebuilt/$HOST_TAG/bin"
CC="$TOOLCHAIN/aarch64-linux-android29-clang"
AR="$TOOLCHAIN/llvm-ar"
STRIP="$TOOLCHAIN/llvm-strip"
OBJCOPY="$TOOLCHAIN/llvm-objcopy"
OBJDUMP="$TOOLCHAIN/llvm-objdump"
READELF="$TOOLCHAIN/llvm-readelf"

for tool in "$CC" "$AR" "$STRIP" "$OBJCOPY" "$OBJDUMP" "$READELF"; do
    if [ ! -x "$tool" ]; then
        echo "NDK tool is missing: $tool" >&2
        exit 1
    fi
done

BUILD_DIR="$ROOT_DIR/build/proot"
OUTPUT_DIR=${OUT_DIR:-${1:-$BUILD_DIR}}
case "$OUTPUT_DIR" in
    /*) ;;
    *) OUTPUT_DIR="$ROOT_DIR/$OUTPUT_DIR" ;;
esac
WORK_DIR="$BUILD_DIR/work"
TARBALL="$WORK_DIR/talloc-$TALLOC_VERSION.tar.gz"
PROOT_DIR="$WORK_DIR/proot"
TALLOC_DIR="$WORK_DIR/talloc-$TALLOC_VERSION"
TALLOC_BUILD="$WORK_DIR/talloc-build"

rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR" "$OUTPUT_DIR"

# No tags: PRoot bakes `git describe` into the binary, and a tag added upstream later would change it.
git clone --no-tags "$PROOT_REPOSITORY" "$PROOT_DIR"
git -C "$PROOT_DIR" checkout --detach "$PROOT_COMMIT"
ACTUAL_COMMIT=$(git -C "$PROOT_DIR" rev-parse HEAD)
if [ "$ACTUAL_COMMIT" != "$PROOT_COMMIT" ]; then
    echo "PRoot checkout is not pinned to $PROOT_COMMIT" >&2
    exit 1
fi

curl --fail --location --retry 3 --output "$TARBALL" "$TALLOC_URL"
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_SHA256=$(sha256sum "$TARBALL" | awk '{print $1}')
else
    ACTUAL_SHA256=$(shasum -a 256 "$TARBALL" | awk '{print $1}')
fi
if [ "$ACTUAL_SHA256" != "$TALLOC_SHA256" ]; then
    echo "talloc checksum mismatch" >&2
    exit 1
fi
tar -xzf "$TARBALL" -C "$WORK_DIR"

cp "$ROOT_DIR/third_party/proot/talloc/config.h" "$TALLOC_DIR/config.h"
mkdir -p "$TALLOC_BUILD/include" "$TALLOC_BUILD/lib" "$TALLOC_BUILD/obj"
cp "$TALLOC_DIR/talloc.h" "$TALLOC_BUILD/include/talloc.h"
if [ -f "$TALLOC_DIR/talloc_compat1.h" ]; then
    cp "$TALLOC_DIR/talloc_compat1.h" "$TALLOC_BUILD/include/talloc_compat1.h"
fi

TALLOC_CFLAGS='-D_GNU_SOURCE -D__STDC_WANT_LIB_EXT1__=1 -DHAVE_CONFIG_H -fPIC -O2'
# talloc embeds __FILE__ in its error strings. Map the build folder away so the binary does not
# depend on where the repository is checked out.
"$CC" $TALLOC_CFLAGS "-fmacro-prefix-map=$TALLOC_DIR/=" "-ffile-prefix-map=$TALLOC_DIR/=" \
    -I"$TALLOC_DIR" -I"$TALLOC_DIR/lib/replace" -c "$TALLOC_DIR/talloc.c" \
    -o "$TALLOC_BUILD/obj/talloc.o"
"$CC" $TALLOC_CFLAGS "-fmacro-prefix-map=$ROOT_DIR/=" "-ffile-prefix-map=$ROOT_DIR/=" \
    -I"$TALLOC_DIR" -I"$TALLOC_DIR/lib/replace" -c \
    "$ROOT_DIR/third_party/proot/talloc/memset_explicit.c" \
    -o "$TALLOC_BUILD/obj/memset_explicit.o"
"$AR" rcs "$TALLOC_BUILD/lib/libtalloc.a" \
    "$TALLOC_BUILD/obj/talloc.o" "$TALLOC_BUILD/obj/memset_explicit.o"

for patch in "$ROOT_DIR"/third_party/proot/patches/*.diff; do
    git -C "$PROOT_DIR" apply "$patch"
done
export CC
export STRIP
export OBJCOPY
export OBJDUMP
export PROOT_UNBUNDLE_LOADER=/nonexistent
export CPPFLAGS="-I$TALLOC_BUILD/include"
export LDFLAGS="-L$TALLOC_BUILD/lib -Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384"
export LOADER_LDFLAGS='-Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384'

cd "$PROOT_DIR/src"
make clean >/dev/null 2>&1 || true
make V=1 loader/loader

if [ "$(uname -s)" = "Darwin" ]; then
    START=$("$READELF" -s loader/loader | awk '$8 == "_start" {print $2; exit}')
    POKEDATA=$("$READELF" -s loader/loader | awk '$8 == "pokedata_workaround" {print $2; exit}')
    if [ -z "$START" ] || [ -z "$POKEDATA" ]; then
        echo "could not find loader symbols for loader-info.c" >&2
        exit 1
    fi
    OFFSET=$((0x$POKEDATA - 0x$START))
    printf '%s\n' '#include <unistd.h>' \
        "const ssize_t offset_to_pokedata_workaround=$OFFSET;" > loader/loader-info.c
fi

make V=1 proot
"$STRIP" proot loader/loader
cp proot "$OUTPUT_DIR/libproot.so"
cp loader/loader "$OUTPUT_DIR/libproot-loader.so"

echo "built PRoot libraries in $OUTPUT_DIR"
