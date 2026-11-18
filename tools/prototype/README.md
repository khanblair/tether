# Mobile UI prototype

A clickable prototype of every Tether screen. Plain HTML, CSS, and JavaScript with no build step. It shows layout, flow, and states, not working features. All data is sample data.

Run it with `just prototype`, then open http://127.0.0.1:4173/.

- **Gallery:** every screen with its purpose, contents, and configuration.
- **Interactive:** tap through screens and flows. The live demo in Session starts a scripted turn when you press Send.
- **Settings in the top bar:** theme, device (phone and tablet, each in two orientations), text size, display size, network, and motion.

`tokens.css` is generated from `tokens/tokens.json` by `just prototype`; do not edit it. The fonts are copies of the ones in `android/designsystem/src/main/res/font`. Icons are Lucide (ISC license).

The Compose screens in `android/` follow these screens. When a screen changes, change the prototype and `docs/ui-rules.md` in the same pull request.
