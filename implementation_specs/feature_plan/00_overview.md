# yClippy Feature Improvement Plan

> **Purpose**: A step-by-step guide for an AI agent to implement 6 features/fixes in the yClippy app.  
> **Tech Stack**: Tauri v2 · Svelte 5 (Runes) · TypeScript · Tailwind CSS v4 · Rust (rusqlite) · YouTube IFrame API  
> **Code Style**: Follow `AGENTS.md` — no comments, 4-space indent, semicolons, `$state()` / `$derived()` runes.

---

## Features at a Glance

| # | Feature | Effort | Files Impacted |
|---|---------|--------|----------------|
| 1 | [Background audio (screen-off)](./01_background_audio.md) | Medium | `VideoPlayer.svelte`, `app.css`, Android manifest |
| 2 | [Remove long-press context menu on Dashboard](./02_remove_longpress_menu.md) | Small | `Dashboard.svelte` |
| 3 | [Replace auto-sync with manual sync button](./03_manual_sync.md) | Medium | `state.svelte.ts`, `Dashboard.svelte`, `SettingsModal.svelte` |
| 4 | [Fix blurred pause overlay](./04_fix_pause_overlay.md) | Small | `VideoPlayer.svelte` |
| 5 | [Timer format hh:mm:ss + fix Dashboard last_position](./05_timer_format.md) | Small | `VideoPlayer.svelte`, `Dashboard.svelte` |
| 6 | [Fix vertical phone UI & clips scroll direction](./06_vertical_ui.md) | Medium | `ClipList.svelte`, `VideoPlayer.svelte`, `Dashboard.svelte` |

## Recommended Implementation Order

1. **Feature 4** — Fix pause overlay blur (1 line CSS change, instant win)
2. **Feature 5** — Timer format + dashboard last_position (isolated, no side effects)
3. **Feature 2** — Remove long-press menu (simple removal)
4. **Feature 3** — Replace auto-sync with manual sync (state + UI changes)
5. **Feature 6** — Vertical phone UI fix (layout restructuring)
6. **Feature 1** — Background audio (Android-specific, most complex)

## Key File Map

```
src/
├── App.svelte                 ← Root, routes between Dashboard and VideoPlayer
├── app.css                    ← Global styles (Tailwind v4 import)
├── global.d.ts                ← Window.YT type declarations
├── lib/
│   ├── state.svelte.ts        ← AppState class (sync logic, settings, navigation)
│   ├── db.ts                  ← Tauri invoke wrappers, TS interfaces
│   ├── Dashboard.svelte       ← Miller columns, folders/videos, context menus, sync status
│   ├── VideoPlayer.svelte     ← YouTube IFrame player, pause overlay, timer, clip controls
│   ├── ClipList.svelte         ← Clip items (horizontal on mobile, vertical on desktop)
│   ├── SettingsModal.svelte   ← GitHub sync settings, auto-sync toggle, data management
│   ├── ContextMenu.svelte     ← Generic right-click/long-press context menu
│   ├── TitleBar.svelte        ← Custom title bar (hidden on hover for desktop, always visible on Android)
│   ├── AddVideoModal.svelte
│   ├── EditVideoModal.svelte
│   ├── EditClipModal.svelte
│   ├── ClipSaveModal.svelte
│   └── Sidebar.svelte

src-tauri/
├── tauri.conf.json            ← App config, window settings, bundle targets
├── Cargo.toml                 ← Dependencies
├── src/
│   ├── lib.rs                 ← Tauri setup, plugin registration, invoke handlers
│   ├── db.rs                  ← SQLite schema, CRUD operations, all Tauri commands
│   ├── sync.rs                ← start_github_sync command
│   ├── sync_engine.rs         ← SyncEngine: fetch remote, merge, push to GitHub
│   ├── github_api.rs          ← GitHub API client (get_file_content, update_file)
│   └── main.rs                ← Entry point (calls lib::run())
```
