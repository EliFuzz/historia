# Historia

Small app. Big relief.

> Copy once. Find it later.

<img src="https://github.com/user-attachments/assets/54fcc790-340e-4ca8-92dd-d0f39164b27c" />

Historia is a native macOS clipboard history app that keeps your recent copies close and your screen clean.

Open it with `⌘ + ⇧ + V`, search what you need, and bring back text, links, images, colors, and file references in a second.

Because the thing you copied 30 seconds ago always disappears exactly when you need it.

## Why it feels good

- **Fast** — the UI stays out of your way
- **Native** — built for macOS, not wrapped in a browser shell
- **Useful** — search, preview, and re-copy in a click
- **Local-first** — your history stays on your Mac
- **Simple** — set retention and history size in seconds

## Quick start

1. Download the latest `.dmg`
2. Move **Historia** to **Applications**
3. Launch the app
4. Press `⌘ + ⇧ + V` to open the panel and see your recent copies.

> Tip: while the panel is open, use `⌘1` to `⌘9` to instantly re-copy the visible items.

## Technical summary

| Area      | Details                                                       |
| --------- | ------------------------------------------------------------- |
| UI        | Native AppKit HUD built with `objc2`                          |
| Storage   | Local `clipboard.jsonl`                                       |
| Captures  | Text, colors, images, file references, etc.                   |
| Shortcuts | `⌘ + ⇧ + V` to open, `⌘1` - `⌘9` to re-copy visible items     |
| Settings  | Retention: 1 / 7 / 30 days / never; Limit: 10 / 20 / 50 / 100 |

### Repo layout

- `src/hud` — app state, persistence, settings
- `src/platform/macos` — native app, panel, events, clipboard monitoring
- `src/updater.rs` — release version checking

### Build

```bash
cargo build --release
```
