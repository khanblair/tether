# Session creation: one way to start a session

Status: built on 28 Nov 2026 on the branch of pull request #224. Scope written the same day, then changed after Blair tried it: the first plan kept the New session screen and removed the other start paths, and the screen turned out to be the broken one. Blair chose the reverse (option A below).

## 1. What was wrong

There were three ways to start a session, and two had their own screens.

| # | Entry point | What it did |
|---|---|---|
| 1 | Drawer, New session | Opened a separate New session screen with its own composer. The session was created on the first Send. |
| 2 | Project home, New session | Created an empty session and opened the real session screen. Worked. |
| 3 | `/new` in a session | Created an empty session and opened the real session screen. Worked. |

The separate screen (1) was a stripped-down copy of the session screen. Its model chip and mode chip did nothing, it had no slash or mention panel and no image attach, its status line was fixed at zero, and the composer cleared the typed text on Send even when the session could not start, so the message was lost. The real session screen had all of these working.

## 2. What was built (option A)

- Every way to start a session creates the session and opens the real session screen: the drawer's New session, New session on a project home, and `/new`.
- The drawer starts in the only project, opens a project picker first when there are several, and opens New project when there are none (`nav/NewSessionLauncher.kt`).
- One failure path for the drawer and `/new` (`startNewSession`): no key and no models show a message with an action to Providers, and any other failure says so. Project home keeps its own inline banners with Test connection.
- The empty transcript of a new session shows what it loads (`session/loads/SessionLoadsCard.kt`): the instructions file, memory entries, skills, and the last session of the project.
- A session opened and left with no message is deleted when its screen closes (`nav/DiscardEmptySession.kt`), so a session nobody used leaves nothing in the lists.
- Deleted: the New session screen, its view model, its route, the first-message handoff between the two screens, and the strings only they used. Nothing is left that sends a message from anywhere but the real composer.

## 3. Done when

- Every control that starts a session ends on the real session screen. True in the code; Blair checks the flows in the emulator.
- Tests cover the start notices, the discard rule, and the loads detail rules. They pass in `just check`.

## 4. Not done, and open

- Project home still has its own start banners and a Test connection button. They work, so they stay. Merging them into the shared message path is possible later.
- A renamed empty session is also deleted on leave. This is rare and loses nothing but the name.
- The first plan asked two questions (a locked project on the Project home button, and `/new` replacing or stacking). Both are moot: `/new` opens the new session on top of the old one as before, and the old one is discarded only when it has no message.
