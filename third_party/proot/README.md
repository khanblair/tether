# PRoot for Tether

Tether ships the Termux fork of PRoot in the `full` Android flavor. It lets the
Rust core start the Alpine Linux root filesystem inside the app process. No
Termux packages or Termux app code are used.

The source is pinned to commit
`c0b2d098469a64a88c0a7e240b1a1dc252988cdb` from
`https://github.com/termux/proot`. The one patch is
`patches/termux-0001-ashmem_memfd-include-string-h.diff`.

talloc 2.5.0 comes from
`https://www.samba.org/ftp/talloc/talloc-2.5.0.tar.gz`.
Its SHA-256 is
`912afa237510ae542a7733998eb18a12bcda35ab6729c8e2ddb43e8d0ebab007`.

The committed arm64 binaries have these SHA-256 values:

| File | SHA-256 |
|---|---|
| `libproot.so` | `d0aa6583bec71963d992c32caca91b9da070aadfa19e50a694dba2db93b33934` |
| `libproot-loader.so` | `ee2f686c978ef9edff446ba4b47c6fa0f6c05d363b2612453c8bfe4c07050073` |

## Rebuild

Install the pinned Android NDK. Run `just proot` from the repository root. The
script uses `ANDROID_NDK_HOME`, `NDK`, or the pinned NDK under the Android SDK.
It verifies the source commit and the talloc archive before building. The
output is copied to `android/app/src/full/jniLibs/arm64-v8a/`.

The build uses the Termux PRoot source, the patch above, talloc's hand-written
`config.h`, and the local `memset_explicit` shim. Both binaries use 16 KB ELF
page alignment. The build is arm64-only because there is no x86_64 binary.

## Licenses and source

PRoot is GPL-2.0-or-later. Its license is in `COPYING.proot`. talloc is
LGPL-3.0. Its license is in `talloc/LICENSE.talloc`.

The binaries in the full flavor are built from this source at this commit with
these patches. This folder and the build script are the corresponding source.

## Reproducibility check

A rebuild with `tools/proot/build.sh` on 2026-11-27, on macOS with NDK 27.1.12297006, from the pinned commit and the pinned talloc tarball, gives both binaries byte for byte identical to the committed ones (same SHA-256 as the table above). Two builds in a row also match each other.

An earlier rebuild differed in `libproot.so` only. The machine code was the same; two embedded strings were not:
- talloc prints its source location in error strings. The script compiled it by absolute path, so the binary held the checkout folder (`/Users/.../build/proot/work/talloc-2.5.0/talloc.c:1435`). It now maps the folder away (`-fmacro-prefix-map`), which gives `talloc.c:1435`.
- PRoot bakes `git describe` into the binary. A full clone also fetches upstream tags and gives `v5.1.107.102-46-gc0b2d098-dirty`. The script now clones with `--no-tags`, which gives `c0b2d098-dirty`.

The longer strings moved `.rodata`, so the PC-relative addresses in `.text` and `.plt` changed too, with the same sizes. After both fixes the output matches exactly. The binaries that were device-tested on Android 17 are therefore the ones this script builds. Run `tools/proot/check-binaries.sh` to confirm the committed files.
