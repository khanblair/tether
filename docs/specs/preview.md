# Preview: see what the agent built, inside the app

Status: scope written 2026-11-26, from Blair's test of a session on the emulator. Stage 1 is being built. Stage 2 starts with a feasibility spike. Every fact below was checked in the code on 2026-11-26.

## 1. What Blair asked for

Blair asked the agent to "create an HTML file, simple website and open it". The file was written, and then every way of opening it failed. Blair wants to create an artifact, run a project or a website, and preview it inside the app. Blair chose the full scope: static preview now, and running a dev server and previewing it as well.

## 2. What is true today

- The agent can write files into the project (`write_file` worked in the test session).
- The `run` tool does not run shell lines. The saved call was `termux-open-file index.html 2>/dev/null || am start ...`. The router answered "no executor is available ... set up Linux mode or CI". Pipes, `||` and redirects are shell syntax, and no executor offers a shell.
- The sandbox executor (Wasmtime, `core/crates/tether-sandbox`) is built. It runs programs from installed runtime packs (Python, QuickJS). It has no sockets, so it cannot serve a page to a WebView.
- Linux mode's Rust library is built (backend baseline, 2026-11-21 to 23: P7-02 to P7-05). `core/crates/tether-linux` has the signed rootfs manifest check, a hardened download and extract, base package install, the PRoot command builder, the runner with output caps and a network policy, a PTY shell, and about 1,900 lines of tests. The FFI has `configure_executors`, `setup_linux_mode`, and a `linux_ready` flag, and the `run` tool already routes to a Linux executor when it is ready.
- Linux mode is not wired into the app, so the router answers "no executor is available ... set up Linux mode or CI". Missing: no PRoot or talloc binaries (P7-01: nothing named `libproot` exists in `android/`); no signed root filesystem manifest, signing key, or hosting (the Executors screen is built, but `nav/ExecutorsRoutes.kt` passes `rootfsManifestUrl = null`); the app never calls `configure_executors`; and `/linux` (P7-07) is still a placeholder route.
- The app has no WebView and no preview screen. `docs/library-docs.md` lists "Any WebView-based UI for core screens" as rejected, because core screens stay Compose. A preview of the user's own web content is a different use and needs its own entry.
- The core reads project files only as text. `FilesRuntime.read_file` caps at 256 KiB and refuses binary files (`core/crates/tether-ffi/src/files.rs`). A page needs images, fonts, and scripts as bytes.
- All project file access in the core goes through `PathGuard` (`tether-workspace/src/path_guard.rs`: no symlinks, no `.git`, nothing outside the root). The review rules flag any file access outside it.
- A per-project network setting exists (`set_project_network`, `project_network` in `tether-ffi/src/executors.rs`, migration 0008).

## 3. Stage 1: static preview (building now)

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

Done when:
- A project with `index.html`, `style.css`, `app.js` and an image shows correctly, with the stylesheet and script applied.
- A request to `https://example.com` is blocked and shown in the banner. Turning the network on lets it through.
- `../../files/databases/tether.db` and a symlink in the project answer 404.
- The pure parts (URL to path, mime, request policy) have unit tests. Blair checks the screen in the emulator.

## 4. Stage 2: run a project, preview a dev server

What it needs, in order:
1. Wire the Linux mode that exists: PRoot and talloc built as `lib*.so` for arm64 and x86_64 (P7-01); an Ed25519 signing key and a signed rootfs manifest for an Alpine mini root filesystem, hosted somewhere stable, with the public key and URL in the build; the app calling `configure_executors` at start with the `nativeLibraryDir` PRoot path; and the `/linux` shell screen (P7-07). The base packages (`nodejs`, `npm`, `python3`) and the runner that binds the project at `/workspace` are already in the library.
2. Long-running processes in the `run` tool. Today a run ends or times out (60 s default). A dev server runs until stopped, so the tool needs a background mode, a log, a stop control, and a rule that all of a session's processes die with it.
3. Port discovery. The runner reads the server's output for its address (for example "Local: http://localhost:5173") and offers Preview. PRoot shares the phone's network, so `127.0.0.1` reaches it.
4. Cleartext to `127.0.0.1` only, through a network security config domain rule. Any other cleartext stays off.
5. A `preview` tool, so the agent can open the preview itself after it starts a server.

First step, a spike that decides if stage 2 is feasible: build PRoot for arm64, ship it as `libproot.so`, and run `proot --version` and an Alpine `sh -c 'echo ok'` on the emulator (Android 15 or later). The Rust side is ready for it, so the spike mostly proves the native binaries and the Android launch rules. If a current Android release blocks it, stop and report before anything else is built.

Risks, from `docs/product-scope.md`: Google Play may reject downloaded executable code, so Linux mode stays in the `full` flavor; a future Android release may block the PRoot technique; the root filesystem download is about 10 MB plus packages, so a first run needs the network.

## 5. Decisions and open questions

- D39: the transcript uses chat bubbles for the user's messages, an action row under messages, and collapsed thinking. This replaces the prototype's "inset user message, no chat bubbles". The prototype is not edited by rule; `docs/ui-rules.md` carries the change.
- D40: a WebView is allowed for previewing the user's own project, behind the rules in section 3, and never for core screens.
- Open: whether a fenced `html` block in a message previews with the network always off (recommended, it has no project behind it).

## 6. Status

- Stage 1: core `read_file_bytes`, the Preview screen, and the entry points are being built.
- Stage 2: the Rust library exists; the app wiring and the PRoot binaries do not. The spike is first.
