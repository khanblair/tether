# Session creation: scope for one New session screen

Status: scope written 28 Nov 2026. Nothing below is built. Base: `main` at v0.0.12. Blair asked for the duplicate session-creation screens to be scoped, so this is a plan to approve, not a change.

## 1. How this was checked

- Read every Kotlin caller of `SessionStarter` and every route that can end in a new session: `nav/HomeRoutes.kt`, `nav/NewSessionRoutes.kt`, `nav/SessionRoute.kt`, `nav/CommandScreens.kt`, `nav/TetherNavHost.kt`, `home/ProjectHomeScreen.kt`, `home/ProjectHomeViewModel.kt`, `newsession/NewSessionViewModel.kt`, `session/SessionStarter.kt`.
- Not available: a device run. The behavior below comes from the code.

## 2. What exists today

There are three ways to start a session, and two of them are full screens that overlap.

| # | Entry point | What it does | Code |
|---|---|---|---|
| 1 | Drawer, New session | Opens the New session screen: what the session will load (instructions file, memory, skills), the last session, a project picker, and a composer. The session is created on the first Send and the screen is replaced by the session. | `newsession/`, `nav/NewSessionRoutes.kt` |
| 2 | Project home, New session button | Creates an empty session at once and opens it. Before that it shows its own start banners: no key, no models, testing the connection, test failed, start failed. | `home/ProjectHomeScreen.kt`, `home/ProjectHomeViewModel.kt` (`StartPhase`) |
| 3 | `/new` in a session | Creates an empty session at once in the same project and opens it on top of the current one. | `nav/CommandScreens.kt` (`startNewSession`) |

Both 1 and 2 call `SessionStarter.start(projectId)`, so session creation itself is shared. The duplication is in the screens around it.

### Where they overlap

- Two start paths with different results. From the drawer the person lands on a composer with context. From Project home the person lands in an empty transcript.
- Two copies of the start failure handling. `StartPhase` (Project home) and `NewSessionStartError` (New session) both handle no key, no models, and a failed start, with different wording and different recovery. Only Project home can test the connection from the banner.
- `newSessionPath(projectId)` in `nav/NewSessionRoutes.kt` already builds the route with a project selected, and nothing calls it. It was written for this merge.
- `/new` stacks a new session over the old one. The old session keeps its connection until it leaves the back stack.

## 3. Proposal

One screen, the New session screen, is the only place that starts a session. The other two entry points open it.

1. Project home, New session: open `newSessionPath(projectId)` instead of calling `home::startSession`. Remove `StartPhase`, `startSession`, `StartBlock`, and the matching strings from Project home.
2. `/new`: open `newSessionPath(projectId)` for the current session's project. Remove `startNewSession` from `CommandScreens.kt`.
3. Move the one useful piece of Project home into the New session screen: Test connection for the no-models and failed-connection cases (today only a snackbar with an Open Providers action).
4. Keep the drawer entry as it is. With no project given, the screen picks the most recent project, as it does now.
5. Delete the dead code and strings this leaves, and update `docs/ui-registry.md` and `docs/ui-rules.md`.

### Done when

- Every control that starts a session opens the New session screen, with the project selected when the control knows it.
- No second start path or second set of start banners remains in the code.
- No key, no models, a failed connection, and a failed start each show one message with one recovery, on the one screen.
- Android tests cover the three entry points and the failure states; `just check` and both assemble tasks pass; Blair checks the flows in the emulator.

## 4. Open questions for Blair

- Should the Project home button open the screen with that project selected and locked, or selected but changeable? Proposed: selected and changeable.
- Should `/new` keep the old session open under the new one, or replace it? Proposed: replace it, so a session you left does not keep running tools unseen.

## 5. Size

Small to medium. About 8 Kotlin files change, no core change, no new dependency. One pull request, written by Claude with Haiku for the mechanical string and test moves. No Codex task, because there is no Rust.
