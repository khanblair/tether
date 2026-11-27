# Preview: see what the agent built, inside the app

Status: scope written 2026-11-26, updated 2026-11-27. Static preview is built. Android wiring for local servers is built and the signed root file system is published (release `rootfs-3.22.2`); the emulator check remains.

## 1. What Blair asked for

Blair asked the agent to "create an HTML file, simple website and open it". The file was written, and then every way of opening it failed. Blair wants to create an artifact, run a project or a website, and preview it inside the app. Blair chose the full scope: static preview now, and running a dev server and previewing it as well.

## 2. What is true today

- The agent can write files into the project (`write_file` worked in the test session).
- The `run` tool does not run shell lines. The saved call was `termux-open-file index.html 2>/dev/null || am start ...`. The router answered "no executor is available ... set up Linux mode or CI". Pipes, `||` and redirects are shell syntax, and no executor offers a shell.
- The sandbox executor (Wasmtime, `core/crates/tether-sandbox`) is built. It runs programs from installed runtime packs (Python, QuickJS). It has no sockets, so it cannot serve a page to a WebView.
- Linux mode's Rust library is built (backend baseline, 2026-11-21 to 23: P7-02 to P7-05). `core/crates/tether-linux` has the signed rootfs manifest check, a hardened download and extract, base package install, the PRoot command builder, the runner with output caps and a network policy, a PTY shell, and about 1,900 lines of tests. The FFI has `configure_executors`, `setup_linux_mode`, and a `linux_ready` flag, and the `run` tool already routes to a Linux executor when it is ready.
- The signed root file system is published and the app wiring is present in the full build. Linux setup still needs the emulator check on a device.
- The app has a WebView preview for the user's own project. Core screens remain Compose. The preview is a separate, guarded use of WebView.
- The core reads project files only as text. `FilesRuntime.read_file` caps at 256 KiB and refuses binary files (`core/crates/tether-ffi/src/files.rs`). A page needs images, fonts, and scripts as bytes.
- All project file access in the core goes through `PathGuard` (`tether-workspace/src/path_guard.rs`: no symlinks, no `.git`, nothing outside the root). The review rules flag any file access outside it.
- A per-project network setting exists (`set_project_network`, `project_network` in `tether-ffi/src/executors.rs`, migration 0008).

## 3. Stage 1: static preview (built)

Behavior:
- A Preview screen shows a project's HTML file in a WebView, served only from that project's folder. It opens `index.html`, or the file the user picked.
- Entry points: the file view of an `.html` file, the "Preview" row in the session menu when the project has an `index.html`, and a Preview button on the tool card of a `write_file` or `edit_file` call on an `.html` file. A fenced `html` block in an agent message gets a Preview button in a later task, because the Markdown renderer is being changed.
- The toolbar has Back, the file name, Reload, and a network chip. The chip starts from the project's network setting, which is off by default. A banner says when a request was blocked.

How it is served (decision D40):
- A new core call, `FilesRuntime.read_file_bytes(project_id, path, limit)`, returns the bytes of one file through `PathGuard`, up to 10 MiB. The WebView intercepts each request to the synthetic origin `https://preview.tether.invalid/` and answers it from that call. The mime type comes from the file extension on the Android side.
- A path of `/` or a folder serves its `index.html`. A path with `..`, a query that tries to escape, or a file the guard refuses answers 404.
- Nothing else is reachable. Any request to another host is blocked and counted, unless the user turned the network on for this preview. Cleartext is never allowed in stage 1.

WebView rules (security):
- JavaScript on, because sites need it. No JavaScript interface to the app. File access, content access, file URL access, geolocation, and downloads off. Every permission request is denied. Navigation to another host is blocked, and a tap on an external link shows its address and does nothing else.
- Cookies and storage belong to the synthetic origin, and `WebStorage` is cleared when the screen closes.

The Android screen, file entry points, WebView rules, path guard, and pure URL tests are built.
Cleartext is disabled for this stage.

Done when:
- A project with `index.html`, `style.css`, `app.js` and an image shows correctly, with the stylesheet and script applied.
- A request to `https://example.com` is blocked and shown in the banner. Turning the network on lets it through.
- `../../files/databases/tether.db` and a symlink in the project answer 404.
- The pure parts (URL to path, mime, request policy) have unit tests. Blair checks the screen in the emulator.

## 4. Stage 2: run a project, preview a dev server (Android wiring built)

What it needs, in order:
1. Finish Linux setup: PRoot and talloc libraries, the signed root file system, the app setup call, and the `/linux` screen. The base packages and `/workspace` runner are in the core.
2. Long-running processes are wired to the session screen. The bar loads the process list, follows `ProcessChanged`, offers Preview and Stop, and reports nonzero exits once.
3. Port discovery is wired to the bar and the server preview. PRoot shares the phone's loopback, so `127.0.0.1` reaches the server.
4. Cleartext is allowed only for the exact server origin `http://127.0.0.1:<owned-port>`. The Android network security config denies cleartext by default and permits only the 127.0.0.1 domain. Other HTTP, other hosts, other ports, user info, fragments, and lookalike hosts stay blocked.
5. The `preview` tool opens a file or an owned server without taking the user away from the transcript. A `PreviewRequested` event shows a message with an Open action and a transcript card line.

First step, a spike that decides if stage 2 is feasible: build PRoot for arm64, ship it as `libproot.so`, and run `proot --version` and an Alpine `sh -c 'echo ok'` on the emulator (Android 15 or later). The Rust side is ready for it, so the spike mostly proves the native binaries and the Android launch rules. If a current Android release blocks it, stop and report before anything else is built.

Risks, from `docs/product-scope.md`: Google Play may reject downloaded executable code, so Linux mode stays in the `full` flavor; a future Android release may block the PRoot technique; the root filesystem download is about 10 MB plus packages, so a first run needs the network.

## 5. Decisions and open questions

- D39: the transcript uses chat bubbles for the user's messages, an action row under messages, and collapsed thinking. This replaces the prototype's "inset user message, no chat bubbles". The prototype is not edited by rule; `docs/ui-rules.md` carries the change.
- D40: a WebView is allowed for previewing the user's own project, behind the rules in section 3, and never for core screens.
- Open: whether a fenced `html` block in a message previews with the network always off (recommended, it has no project behind it).

## 6. Status

- Stage 1: core `read_file_bytes`, the Preview screen, and the entry points are built.
- Stage 2 core and Android app wiring: background Linux processes, port discovery, process tools, server preview validation, process state, and `ProcessChanged` / `PreviewRequested` handling are built. PRoot packaging and the signed root file system release are done. The emulator check remains.
