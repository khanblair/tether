// Area: toasts, snackbars, and banners. All colors come from tokens; the inverse surface flips with the theme.
area("toasts", "Toasts, snackbars, and banners", "Feedback that does not need a screen. Rule of thumb: a snackbar for an action the user can undo, a toast for a small confirmation, a banner for a state that lasts, and an inline block in the transcript for any error. Errors never use a toast, because a toast disappears before it can be read or acted on.");

const SNACK_RULES = ["Undoable actions only. Four seconds, one at a time, Undo is a text button", "Sits above the composer and bottom bars, never over them", "Announced politely to TalkBack; Undo stays reachable after the visual timeout when TalkBack is on", "Colors are the inverse surface: dark with a mint action in the light theme, light with a plum action in the dark theme"];
const snackFrame = (id, title, why, base, text, action, bottom, has) => frame({
  id, area: "toasts", title, why, overlay: base, has, config: SNACK_RULES,
  overlayHtml: () => snack(text, action, bottom),
});

snackFrame("snack-archive", "Session archived", "Archiving is reversible, so a snackbar with Undo is enough. No confirmation dialog.", "project-home", "Session archived", "Undo", 112, ["Short past-tense statement", "Undo restores the session in place", "Same result from a swipe or the row menu"]);
snackFrame("snack-commit", "Committed", "A commit is undoable until it is pushed. Say what happened, offer Undo.", "commit", "Committed 3 files", "Undo", 136, ["Count of files, not a hash", "Undo resets the commit and keeps the changes"]);
snackFrame("snack-hunk", "Hunk undone", "Per-hunk Undo changes files, so it is undoable as well.", "diff", "Hunk undone", "Undo", 72, ["Fires on Keep and Undo for a hunk", "Undo restores the lines"]);
snackFrame("snack-rewind", "Rewound", "A rewind is the biggest undo in the app, so its own undo gets a snackbar too.", "session-diffcard", "Rewound 2 files and 4 messages", "Undo", 112, ["Says exactly what was restored", "Undo re-applies the later checkpoint"]);
snackFrame("snack-key", "Key saved", "Confirms a save with no action to undo. It names the next step instead.", "add-key", "Key saved. Testing it now.", null, 100, ["No action button", "Replaced by the result of the test"]);
snackFrame("snack-sync", "Vault synced", "Background work reports its result once, quietly.", "vault", "Vault synced. 4 notes updated.", null, 28, ["No action button", "Conflicts are reported in the vault, not here"]);

frame({
  id: "toast-copied", area: "toasts", title: "Toast: copied", overlay: "file-view",
  why: "A toast is for a tiny confirmation that cannot be undone and needs no action.",
  has: ["One short phrase, centered, pill shape", "Two seconds, no action"],
  config: ["The system shows its own toast for clipboard on Android 13 and later; Tether skips its own there", "Never used for errors or anything the user may need to act on", "Same inverse surface colors as snackbars"],
  overlayHtml: () => toast("Path copied", 96),
});

frame({
  id: "banners", area: "toasts", title: "Banners",
  why: "A banner is for a state that lasts until it is fixed. It sits under the status line, keeps the screen usable, and says what to do.",
  has: ["Error (danger): push failed, with Retry", "Warning: offline, the turn continues when you reconnect", "Information: an update, dismissible"],
  config: ["At most two at once, ordered by severity", "A banner stays until the cause is gone, or until the user dismisses an informational one", "Every banner has a text label and an icon, not only color"],
  render: () => sessionScreen({
    banner: `<div class="banner err" role="alert">${I("circle-x", "s")}<span class="grow">Push failed: GitHub returned 403. Check the token scopes.</span><button>Retry</button></div><div class="banner">${I("wifi-off", "s")}<span class="grow">Offline. The turn will continue when you reconnect.</span></div>`,
    transcript: `<div class="transcript">${T_BASE}<div class="card" style="background:var(--tt-color-danger-soft)"><div class="col gap12"><div class="row" style="align-items:flex-start"><span class="cdanger">${I("circle-x")}</span><div class="grow"><b class="t-title-s cdanger">Push failed</b><div class="t-body-s cdanger">GitHub rejected the push: 403. The token lacks the workflow scope.</div></div></div><div class="row">${btn("Retry", { sm: true })}${btn("Open GitHub settings", { kind: "text", sm: true, go: "set-github" })}${btn("Copy error ID", { kind: "plain", sm: true, snack: "toast:Error ID copied" })}</div></div></div></div>`,
    bottom: composer({}),
  }),
});

frame({
  id: "banner-info", area: "toasts", title: "Banner: information",
  why: "Informational banners can be dismissed. They never block work and never repeat after dismissal.",
  has: ["Update available with Update and dismiss", "Neutral tone: no danger or warning color"],
  config: ["Shown at most once per version", "Update opens the release page for the full build; the Play build updates through the store"],
  render: () => sessionScreen({
    banner: `<div class="banner" style="background:var(--tt-color-accent-soft);color:var(--tt-color-accent-ink)">${I("info", "s")}<span class="grow">Tether 0.1.0 is available.</span><button>Update</button><button aria-label="Dismiss">${I("x", "s")}</button></div>`,
    transcript: `<div class="transcript">${T_BASE}</div>`, bottom: composer({}),
  }),
});
