# Linux mode spike: PRoot on Android 17

Status: spike and foreground-service implementation done 2026-11-27. The result is a decision (D42) and a list of work in section 6. Facts below were measured on an Android 17 emulator (API 37, arm64, 16 KB pages, a `user` build with no root) in a real app process, with a test app that targets API 36. The build scripts, patches, and logs are kept outside the repo in the session folder and are summarized here. Not tested on a device: a release build, the phantom process limit, and x86_64.

## 1. Question

Can PRoot start Alpine programs from an app that targets API 36, with the PRoot launcher in `nativeLibraryDir`? Linux mode's Rust library (`core/crates/tether-linux`) is built, but the app has no PRoot binaries, so nothing ran.

## 2. Result

Two PRoot builds were compiled with NDK r27 against a hand-built talloc 2.5.0 and run through the same tests, with the arguments and environment that `tether-linux/src/proot.rs` builds.

| Test | Upstream v5.5.0 | Termux fork |
|---|---|---|
| `--version` | pass | pass |
| musl shell: `echo ok; uname -m` | pass | pass |
| `apk add nodejs`, then `node -e` | fail | pass |
| Node HTTP server, reached from the guest and from the Mac with `adb forward` | fail | pass |

Start times in the app process: `--version` 8 to 10 ms, a musl shell 33 to 36 ms, `node -v` 56 to 110 ms. Node `child_process`, `worker_threads`, `fetch('https://example.com')` (200), and `ln` under `--link2symlink` also passed with the Termux fork.

## 3. The Android rules that decided it

- **Rule A.** Executing a file from the app's data folder is denied (`avc: denied { execute_no_trans } ... app_data_file`). Mapping it as executable through PRoot's loader is allowed today and is audited (`avc: granted { execute }`). Both builds run musl programs only when `PROOT_LOADER` points at a loader in `nativeLibraryDir`. Without it, upstream extracts its loader into the data folder and the exec fails, and the unbundled Termux build fails with "No such file".
- **Rule B.** The app's seccomp filter raises SIGSYS for syscalls such as io_uring, `futex_waitv`, `rseq`, `set_robust_list`, `kcmp`, and `mount_setattr`. Upstream passes SIGSYS to the program, so Node dies. The Termux fork catches it and returns ENOSYS, so Node runs. Upstream also fails `apk add` on the `link` of an unnamed temp file (`avc: denied { link }`); why the fork avoids that was not found.
- **Rule C.** The app freezer stops child processes when the app goes to the cached state (`do_freezer_trap`). A Linux session needs a foreground state while it runs.
- `run-as` is not a faithful test: it runs in another SELinux domain with no seccomp filter and blocks UDP. All results above come from the real app process.

## 4. Recipe, for the Termux fork

- Source: `github.com/termux/proot` at commit `c0b2d098469a64a88c0a7e240b1a1dc252988cdb` (2026-11-24). Source only: no Termux packages or app code are used. Patch: add `#include <string.h>` in `ashmem_memfd.c` (clang 18).
- talloc 2.5.0 from samba.org (SHA-256 `912afa237510ae542a7733998eb18a12bcda35ab6729c8e2ddb43e8d0ebab007`, signature verified against the Samba distribution key), built as a static library with a hand-written `config.h` and a small `memset_explicit` shim. No waf.
- Build: `CC=aarch64-linux-android29-clang`, `PROOT_UNBUNDLE_LOADER=/nonexistent`, and `-Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384` for both `proot` and the loader. A 4 KB build is refused by the Android linker on a 16 KB device. Run `make` with `V=1`: in quiet mode the feature checks fail silently.
- macOS build trap: `loader-info.c` needs GNU `readelf` and `gawk`. Without them the build succeeds and bakes in a wrong offset (0, not 1016). A Linux CI build with GNU tools does not hit this.
- Sizes (stripped): `proot` 311,480 bytes, loader 17,624 bytes. Licenses: PRoot is GPL-2.0-or-later; talloc is LGPL-3.0. Shipping the binaries means publishing the source and patches for them.

## 5. What Tether must do

- Ship `libproot.so` and `libproot-loader.so` in `jniLibs/arm64-v8a` of the `full` flavor. The app build file sets `android.packaging.jniLibs.useLegacyPackaging = true`, because the default leaves no file to execute. AGP 9.5.1 rejects `android:extractNativeLibs` in a manifest (found when `assembleFullDebug` failed) and its variant API has no per-variant switch, so the setting applies to both flavors: the play flavor installs larger, with `libtether_core.so` extracted.
- Read `ApplicationInfo.nativeLibraryDir` at every start. The path changed between two installs. Never store it.
- Always set `PROOT_LOADER=<nativeLibraryDir>/libproot-loader.so`. Make `loader_path` required in `proot.rs` (it is optional now).
- Keep the app in a foreground state while Linux runs. The full flavor has a `specialUse` foreground service with the `linux-userland` subtype, an ongoing low-importance `linux_session` notification, and the `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_SPECIAL_USE` permissions. The Executors screen offers a setting for only while a shell or server runs, or whenever Linux mode is installed. The Play flavor uses a no-op controller.
- Tether's own `libtether_core.so` is already 16 KB aligned, and the APK passes `zipalign -P 16`.

## 6. Risks

- Rule A is the main risk: if a future Android release denies `execute` on `app_data_file` for mapping, PRoot with a data-folder root filesystem stops working. Mitigation: the CI executor stays available, and Linux mode is labeled best effort.
- Rule B depends on the fork's SIGSYS handling. A guest program that needs a blocked syscall must cope with ENOSYS.
- The app freezer risk is addressed by the foreground service and its setting. A real device test still needs to confirm the service across screen off, app backgrounding, and process events. The phantom process limit also needs a device test.
- x86_64 has no build yet. Intel emulators and CI use x86_64, so Linux mode shows as not available there.

## 7. Work that follows

1. **Done.** `proot_loader_path` is required in `tether-linux` and `configure_executors`, and the runner sets `PROOT_LOADER`.
2. **Done for arm64.** `third_party/proot/pins.sh` and `tools/proot/build.sh` pin the source and talloc checksum, apply the patch, build both binaries, and publish the GPL source offer.
3. **Done, 2026-11-27.** The app build extracts the arm64 `jniLibs`, app start configures the executor paths from `nativeLibraryDir`, and the full resources hold the manifest URL and public key. `just linux-key` made the key (kept in `~/.tether-release/`, outside the repo), `just linux-manifest` signed the Alpine 3.22.2 manifest, `tether-linux-manifest verify` accepted the signature and rejected a changed manifest, and the release `rootfs-3.22.2` holds the four files, marked not latest so app updates are unaffected. The in-app setup flow has not been run on a device yet.
4. **Foreground service built.** Background processes in the `run` tool, port discovery, cleartext to `127.0.0.1`, and the Preview of a localhost server remain.
5. **Done.** The `/linux` shell screen accepts input with its terminal key row, keeps bounded scrollback, and shows the capability message in the Play flavor (P7-07).
