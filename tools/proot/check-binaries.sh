#!/bin/sh
# Check the committed full-flavor PRoot files without needing Gradle or an NDK.
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
LIB_DIR="$ROOT_DIR/android/app/src/full/jniLibs/arm64-v8a"

check_sha256() {
    file=$1
    expected=$2
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    else
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    fi
    [ "$actual" = "$expected" ] || {
        echo "checksum mismatch: $file" >&2
        exit 1
    }
}

check_sha256 "$LIB_DIR/libproot.so" \
    d0aa6583bec71963d992c32caca91b9da070aadfa19e50a694dba2db93b33934
check_sha256 "$LIB_DIR/libproot-loader.so" \
    ee2f686c978ef9edff446ba4b47c6fa0f6c05d363b2612453c8bfe4c07050073

if command -v llvm-readelf >/dev/null 2>&1; then
    READELF=llvm-readelf
elif command -v readelf >/dev/null 2>&1; then
    READELF=readelf
else
    echo "checksums match; readelf is unavailable, so 16 KB alignment was not checked"
    exit 0
fi

for file in "$LIB_DIR/libproot.so" "$LIB_DIR/libproot-loader.so"; do
    "$READELF" -lW "$file" | awk '
        /LOAD/ && $NF != "0x4000" { bad = 1 }
        END { exit bad }
    ' || {
        echo "16 KB LOAD alignment check failed: $file" >&2
        exit 1
    }
done
echo "checksums and 16 KB LOAD alignment match"
