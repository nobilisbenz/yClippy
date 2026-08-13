# yClippy — integration plan, UI redesign, and bug inventory

yClippy is the video surface of the yalive ecosystem: the place a YouTube clip is
watched, trimmed, and named, on the desktop and on the phone. Today it is a
standalone app with its own database and its own GitHub sync. This plan makes it
a member of the ecosystem instead — one repository, one play protocol, one way
of naming a moment in a video — and then rebuilds the desktop and Android
interfaces on top of that.

Three deliverables, in order of dependency:

1. **[Integration](#1-integration)** — how yClippy joins the vault, and how the
   TUI, Neovim, and Brain Dock start playback in it.
2. **[Redesign](#2-desktop-redesign)** / **[Android](#3-android-redesign)** — what
   the two interfaces should become.
3. **[Bugs](#5-bug-inventory)** — 47 findings, located and ranked, for you to fix.

---

## 0. What exists today

**yalive** (this repo's root) is the TUI: a Markdown vault, a disposable SQLite
index, FSRS review state, and Git-based sync (`src/sync.rs`) that commits,
rebases, and pushes the vault. It already exposes a versioned JSON surface for
editors — `yalive editor capabilities | sections | relations | diagnostics`
(`src/main.rs:123`) — and a documented config file at `.notes/config.toml`.

**yGraphy** talks back to a running TUI through an atomic command file:
write `.notes/ygraphy-open.pending`, rename it to `.notes/ygraphy-open.json`
(`yGraphy/src/vault.rs:47`), and the TUI drains it on its next tick
(`src/app.rs:290`). Rename is atomic within a filesystem; a partial write is not.
**This is the pattern to copy.**

**yReviewy** is the Android reviewer, and it already solved the two problems
yClippy is about to hit: the GitHub token lives in Rust app-private storage and
only a `token_present` boolean crosses into the webview
(`yReviewy/src-tauri/src/lib.rs:30`), and each phone owns one append-only
mailbox file so two devices never conflict.

**yy** (Brain Dock) parses `@video URL HH:MM:SS` out of notes at index time and
launches it through a configured opener with a `{url}` / `{seconds}` template
(`yy/crates/brain-engine/src/actions.rs:380`). It already has an `OpenVideo`
action with a fallback chain. **This means yy needs no code at all to use
yClippy — one config line.**

**yClippy** is a Tauri 2 + Svelte 5 app, ~5k lines, with local SQLite
(videos / clips / folders), a Miller-column library, a YouTube iframe player, an
Android share-intent import, a ReVanced handoff, and a GitHub Contents API sync
that pushes three JSON files to a repository root. It is not referenced in the
yalive README, has no release workflow, and no tests.

---

## 1. Integration

### 1.1 The idea

One line of Markdown is the whole contract:

```markdown
@video https://www.youtube.com/watch?v=dQw4w9WgXcQ 06:54  Chapter on borrowing
```

Brain Dock already parses it. yalive's TUI and the Neovim plugin will learn it.
yClippy writes it (from the clipboard template) and plays it (from the CLI).
A moment in a video becomes a first-class citizen of the vault: searchable,
linkable, graphable, reviewable — without yClippy having to understand Markdown
and without the vault having to understand SQLite.

```
  nvim ──┐
  TUI  ──┼──▶  yclippy play <url> --at <sec>  ──▶  running yClippy (single instance)
  yy   ──┘                                              │
                                                        ▼
                        vault repo  ◀── git (desktop) ── SQLite library
                             ▲
                             └── GitHub Contents API ── phone
```

### 1.2 The play protocol

Add argument parsing to the yClippy binary *before* `tauri::Builder` runs, plus
`tauri-plugin-single-instance` and `tauri-plugin-deep-link`.

```
yclippy play <url|videoId> [--at SECONDS] [--end SECONDS] [--clip CLIP_UID]
yclippy add  <url> [--folder PATH] [--title TITLE]
yclippy list [--json] [--query Q]        # videos and clips, for pickers
```

Behaviour:

- **An instance is running** → single-instance forwards `argv`; the running app
  emits `yclippy://play` to the frontend, which opens the video and seeks.
- **No instance** → the app starts and drains the argv in `setup()`.
- **The phone** → the same intent arrives as a `yclippy://play?v=ID&t=90` deep
  link, so anything on the device (share sheet, another app, a note-taking app)
  can hand off to it. Register the scheme in the manifest alongside the existing
  `ACTION_SEND` filter.

*Optional fallback,* if you'd rather not depend on `PATH`: yClippy polls
`<vault>/.notes/yclippy-open.json` and drains it exactly like the TUI drains
yGraphy's file. Same pending-then-rename discipline. Worth adding only if you
want a play request to survive yClippy being closed.

### 1.3 The three callers

**Brain Dock (yy) — zero code.** One line in `config/brain.toml`:

```toml
[openers]
video = ["yclippy", "play", "{url}", "--at", "{seconds}"]
```

`video_template()` already falls back silently to mpv and then `xdg-open` if the
binary is missing, so this is safe to sync between machines.

**The TUI — one key and one config field.** Add to `.notes/config.toml`, using
the same template shape as yy so there is one mental model:

```toml
player = ["yclippy", "play", "{url}", "--at", "{seconds}"]
```

Then in `src/app.rs`, next to `open_selected_url` (`src/app.rs:1472`), add
`play_selected_video` bound to `v` on the Dashboard: parse `@video` out of the
selected section's body (URL plus optional `HH:MM:SS` / `MM:SS` / bare seconds),
fall back to the first YouTube URL in the section, expand the template, spawn
detached. Lift the ~40 lines of `expand()` / `with_timestamp()` from
`yy/crates/brain-engine/src/actions.rs:330` rather than reinventing the
timestamp rebuild — a handler without `{seconds}` needs `&t=` put back into the
URL, and that logic already has tests.

Also extend the editor protocol so Lua never has to parse Markdown:

```
yalive --vault ~/Notes editor videos [section_uid]
→ {"protocol_version":1,"items":[{"url":"…","seconds":414,"label":"…","section_uid":"…"}]}
```

**Neovim — three commands** in `nvim/lua/yalive/init.lua`, reusing the existing
`command` / `editor` / `pick` helpers:

| Command | Does |
| --- | --- |
| `:YClippyPlay` | Play the `@video` on the current line, else the URL under the cursor, else the first video in the current section. |
| `:YClippyLibrary` | `yclippy list --json` → picker → play. Find a clip you saved months ago without leaving the note. |
| `:YClippyInsert` | Picker → insert `@video <url> <hh:mm:ss>  <title>` at the cursor. |

`:YClippyInsert` is what closes the loop: clip in yClippy → drop the line into a
note → the clip is now indexed, graphed, and replayable from three places.

**And the reverse direction:** add a `{start_hms}` placeholder to yClippy's
clipboard template and ship a second preset next to the iframe one:

```
@video {url} {start_hms}  {title}
```

so "Copy Embed" can produce a line the vault understands.

### 1.4 One repository

Today the vault syncs with Git (desktop) and the Contents API (phone), while
yClippy syncs three JSON files to the root of a *separate* repository with a
token kept in `localStorage`. Collapse this onto the vault repo, following the
shape yReviewy already proved:

```
<vault>/.notes/yclippy/library.json          # canonical merged state
<vault>/.notes/yclippy/devices/<device>.jsonl # append-only op log, one per device
```

- **Desktop** syncs with Git, through the existing `sync::sync` in `src/sync.rs`
  — which already handles rebase-or-merge, aborts cleanly on conflict, and
  maintains `.gitignore`. yClippy desktop shells out to the same routine (or
  calls `yalive --vault … sync`).
- **The phone** never uses Git. It reads `library.json` plus any device logs it
  hasn't seen through the Contents API, and writes **only its own** `<device>.jsonl`.
  One file, one writer, no conflicts, ever. Exactly the mailbox model in the
  yalive README.
- **Compaction** happens on desktop sync: apply every device log in timestamp
  order, rewrite `library.json`, record a `compacted_through` watermark per
  device, then truncate. Tombstones get dropped once every device's watermark has
  passed them.

An operation is:

```json
{"op":"upsert","entity":"clip","uid":"9f2c…","at":1754960000123,
 "device":"desktop-a1b2","fields":{"video_id":"dQw4w9WgXcQ","start":414,"end":460,"title":"…"}}
```

**This is blocked on identity.** Folders and clips are currently identified by
local SQLite `AUTOINCREMENT` rowids (`src-tauri/src/db.rs:127`, `:155`) and the
sync engine merges on exactly that number (`sync_engine.rs:189`). Two devices
that each create a folder get id `1`; the merge treats them as the same folder,
one name silently overwrites the other, and videos re-parent into a folder that
was never theirs. Same for clips.

**Fix first, before any sync work:** add `uid TEXT UNIQUE` to `folders` and
`clips`, backfill with UUIDv4 on migration, key all sync on `uid`, keep the
integer id for local joins and UI only. Videos keep the YouTube ID as identity —
with one consequence worth deciding on now: the same video cannot appear in two
folders. If you want that, videos need a `library_items` table (uid, video_id,
folder_uid) and the video row becomes pure metadata.

And move the token: Rust app-config, `token_present: bool` over the bridge,
never in `localStorage` and never in the DOM.

### 1.5 Ecosystem chores

- Add yClippy to the app table in the yalive README, with a Linux `.deb` and an
  Android APK link.
- Copy `yReviewy/.github/workflows/release.yml` — tag-triggered build, APK
  signing from the four `ANDROID_KEYSTORE_*` secrets, `SHA256SUMS`.
- `src-tauri/gen/android` is tracked (correctly — `MainActivity.kt` is
  hand-written). Note in `AGENTS.md` that `tauri android init` will clobber it.

---

## 2. Desktop redesign

Keep the ideas that are right: black ground, Finder-style Miller columns,
keyboard-first, clips as a list beside the video. What follows is about making
them clean, reachable, and consistent.

### 2.1 Layout

Today opening a video *replaces* the entire library (`App.svelte:41`). On a
1080p screen there is room for both, and losing your place in the tree every
time you watch something is the single biggest structural annoyance.

```
┌────────────────────────────────────────────────────────────┐
│ ⌂ Library › Rust › Ownership                      ⌕  ⚙  ⟳ │  40px title/command bar
├──────────────┬───────────────────────────────┬─────────────┤
│ folders      │                               │ Clips   (7) │
│  videos      │           player              │  ▸ 06:54 …  │
│              │                               │  ▸ 12:03 …  │
│  (columns    ├───────────────────────────────┤             │
│   scroll     │ ▶ ━━━━━●───────────── 12:03   │  + New clip │
│   sideways)  │ ⏮ ⏯ ⏭   [ mark in ] [ out ]   │             │
└──────────────┴───────────────────────────────┴─────────────┘
   280px min          fluid, 16:9 capped          320px
```

Below 1024px the library collapses to an overlay drawer; below 768px it becomes
the Android layout. One responsive tree, three breakpoints, no separate desktop
and mobile component trees to keep in sync (`VideoPlayer.svelte` and
`NativePlayer.svelte` are 90% duplicated today and have already drifted — see
bug 21).

### 2.2 The transport bar

The most important missing thing in the whole app. There is no way to seek
(bug 23) — you set a clip by watching to the moment in real time.

- Scrub bar with **clip markers rendered on the track** — the existing clips of
  this video shown as tick ranges, so the timeline is a map of what you've
  already captured.
- In/out marking happens *on the scrub bar*: `[` sets in, `]` sets out, the
  pending range highlights, `Enter` names and saves it. The two-button plus modal
  flow stays available but stops being the only way.
- Current / total time in `tabular-nums`, playback speed, ±5s / ±10s.
- Keyboard: `space` play, `j`/`l` ∓10s, `,`/`.` frame-ish nudge, `[`/`]` mark,
  `Enter` save clip, `Esc` back to library, `1–9` jump to clip N. Re-enable
  YouTube's own fullscreen (`fs: 0` and `disablekb: 1` currently kill both,
  `VideoPlayer.svelte:44`).

### 2.3 Reaching what already works

Three finished features are unreachable from the UI: editing a video's
title/start/end (bug 36), renaming a video or folder (bug 37), and moving items
without a mouse drag (bug 29). Everything below is wiring, not new backend code.

- **Right-click menus** on videos and folders — Rename · Edit trim · Move to… ·
  Copy embed · Delete. `ContextMenu.svelte` already exists and works; it's only
  used by the clip list today.
- **Command palette** (`Ctrl+K`): fuzzy search across folders, videos, and clips;
  `Enter` opens, `Ctrl+Enter` plays a clip directly. This is also the fastest
  path to the actions above, and it's what makes the app feel keyboard-first
  rather than merely keyboard-tolerant.
- **Toasts and in-app dialogs** replacing `alert` / `confirm` / `prompt`
  (bug 25). Destructive actions get undo (a 5-second toast) instead of a
  confirm — the soft-delete columns already in the schema make this nearly free.

### 2.4 Visual system

The current styling is ad-hoc Tailwind: `zinc-950`, `zinc-900`, `zinc-800`, and
`black` all appear as "background", and blue is used both for selection and for
"this is a link". Define tokens once in `app.css` and use only those:

| Token | Role |
| --- | --- |
| `--bg` | app ground (`#08080a` — near-black with a hair of blue, not pure `#000`) |
| `--surface` / `--surface-hi` | panels; hovered rows |
| `--border` / `--border-hi` | hairlines; focused hairlines |
| `--text` / `--text-dim` / `--text-faint` | primary; secondary; timestamps |
| `--accent` | selection and primary buttons — one blue, one job |
| `--danger` | destructive only |

Plus: an 8px spacing scale, a 32px desktop row height (48px touch), one focus
ring (`outline: 2px solid var(--accent); outline-offset: 2px`) that is actually
visible — there is none today — and `prefers-reduced-motion` honoured on the
sidebar and sheet transitions.

---

## 3. Android redesign

The Android build today is the desktop layout with a couple of media queries.
The result: hover-only affordances, drag-and-drop that touch cannot trigger, a
library that shows one column with no visible hierarchy, and content that sits
under the status bar.

### 3.1 Fit any phone

This is the mechanical part, and it fixes bug 26 outright:

```html
<meta name="viewport"
      content="width=device-width, initial-scale=1, viewport-fit=cover" />
```

```css
:root {
  --safe-top: env(safe-area-inset-top, 0px);
  --safe-bottom: env(safe-area-inset-bottom, 0px);
  --safe-left: env(safe-area-inset-left, 0px);
  --safe-right: env(safe-area-inset-right, 0px);
}
```

- `100dvh`, never `100vh` — the app root hard-codes `height: 100vh`
  (`App.svelte:38`) and the system bars resize.
- Support 320px width up. Nothing gets a fixed 280px column
  (`Dashboard.svelte:391`); the type scale uses `clamp()`.
- Every touch target ≥48dp. The delete `×` on every row is currently a ~20px
  target sitting next to the row's own tap area — a mis-tap deletes a video.
- No hover-only affordance anywhere: `group-hover` reveal patterns must have a
  long-press equivalent.
- Handle landscape: player fills the screen, controls auto-hide after 3s, clips
  become an edge sheet.

### 3.2 Structure

```
┌─────────────────────┐   ┌─────────────────────┐
│ ‹  Rust › Ownership │   │      [ 16:9 ]       │  sticky player
├─────────────────────┤   ├─────────────────────┤
│ 📁 Borrowing      › │   │ ━━━━●───────  12:03 │
│ 📁 Lifetimes      › │   │  ⏮   ⏯   ⏭    ⏱   │  56px controls
│ ▸ Ownership intro   │   ├─────────────────────┤
│ ▸ Move semantics    │   │ ▲ Clips (7)         │  drag-up sheet
├─────────────────────┤   └─────────────────────┘
│  ⌕      +       ⚙  │  bottom bar, thumb-reachable
└─────────────────────┘
```

- **One column, drill-down.** The breadcrumb goes to the *top* bar with a back
  chevron; the bottom bar carries search / add / settings where thumbs are. Today
  the breadcrumb is in a wrapping footer and the deepest column is all you see.
- **Clips as a bottom sheet** with a drag handle and a scrim, not an absolutely
  positioned panel that covers the transport bar with no way out (bug 31).
- **Explicit move and reorder**, since drag-and-drop is desktop-only: long-press
  → "Move to folder…" (a folder picker) and a reorder mode with grab handles.
- **Back** is one stack: video → folder → parent folder → exit. Guard the
  popstate handler against a video that isn't loaded yet (bug 22).

### 3.3 Playback, honestly

`global.d.ts:7` declares eight native methods — background audio, wake lock, a
native player with seek — of which the Kotlin bridge implements two
(`MainActivity.kt:132`). Pick one:

- **Ship the handoff.** Delete the six phantom declarations, keep
  "Open in ReVanced", and *carry the timestamp* — the intent is built as a bare
  `watch?v=ID` (`MainActivity.kt:74`), so a handoff at 12:30 restarts at 0:00.
  Cheap, honest, works today.
- **Or implement background audio properly**: a foreground service with a
  `MediaSession`, notification controls, `POST_NOTIFICATIONS`, and a wake lock
  while the webview player runs. Real work; only worth it if you actually listen
  to clips with the screen off.

Do not leave the declarations without the implementations.

### 3.4 Import from anywhere

- Add an `ACTION_VIEW` filter for `youtube.com` / `youtu.be` links and accept
  `text/*`, not only `text/plain`, so yClippy appears in more share sheets.
- The share dialog must **not** clobber an existing video (bug 5) and should let
  you pick a folder instead of always dropping at the root.
- Register the `yclippy://` scheme from §1.2 here too.

### 3.5 Capabilities

Split `src-tauri/capabilities/` into `desktop.json` (`"platforms": ["linux",
"macOS", "windows"]`, keeping the window minimize/maximize/drag permissions) and
`mobile.json` (`"platforms": ["android"]`). The single file today references the
desktop schema and requests desktop-only permissions for the Android build
(bug 32).

---

## 4. Delivery order

Each phase is independently shippable and has a definition of done.

| # | Phase | Contents | Done when |
| --- | --- | --- | --- |
| **0** | Stop the bleeding | Bugs 5, 9, 10, 12, 16, 21, 22, 26, 28, and the dead code in §5D | Sharing a known video keeps its progress; the player stops leaking; safe areas hold on a notched phone |
| **1** | Identity + one repo | `uid` migration, op-log sync into `.notes/yclippy/`, token to Rust, phone mailbox | Two devices create folders offline, sync, and both survive with the right contents |
| **2** | The play protocol | `yclippy play/add/list`, single-instance, deep link, yy opener line, TUI `v` key + `player` config, `editor videos`, three Neovim commands | `:YClippyPlay` on an `@video` line starts that video at that second |
| **3** | Desktop redesign | Tokens, three-pane layout, transport bar with clip markers, context menus, command palette, toasts | No feature in `db.ts` is unreachable from the UI |
| **4** | Android redesign | Safe areas, one-column drill-down, bottom sheet, 48dp targets, move/reorder, share filters | Usable one-handed on a 360×640 phone and a tablet |
| **5** | *Optional:* videos as notes | Each video becomes a Markdown note, each clip a section with `@video` | Clips appear in yGraphy, `yalive` search, and FSRS review |

Phase 5 is the interesting one and the reason to keep the vault format in mind
while doing 1–4: if a clip is a section, then the graph, full-text search, the
review scheduler, and Brain Dock's ranking all apply to your video library for
free. The cost is that yClippy stops owning its own storage. Worth prototyping
after phase 2, decided after phase 4.

---

## 5. Bug inventory

47 findings from reading the whole app. Severity: **P0** data loss or silently
wrong data · **P1** broken or misleading behaviour · **P2** rough edges and dead
weight. `svelte-check` is clean and `cargo check` passes, so none of these are
compiler-visible.

### A. Data loss and sync

**1. [P0] Folder and clip identity is a local rowid.**
`src-tauri/src/db.rs:127`, `:155`, merged on that number at
`sync_engine.rs:189`. Two devices each create folders numbered 1, 2, 3…; the
merge treats device A's folder 2 and device B's folder 2 as the same entity, so
one name and parent silently overwrite the other and videos re-parent into the
wrong tree. Clips collide the same way, across unrelated videos. Nothing else in
the sync design can be trusted until this is fixed.

**2. [P0] Whole-row last-writer-wins on unsynchronised clocks.**
`sync_engine.rs:196` compares `updated_at` values that each device stamps from
its own wall clock (`db.rs:86`). A phone with a fast clock wins every conflict
forever, and editing a title on one device reverts a folder move made on the
other, because the whole row is the unit.

**3. [P0] The push is three unrelated writes.**
`sync_engine.rs:106`–`:138` PUTs `videos.json`, `folders.json`, `clips.json`
in sequence. A failure or a 409 between them leaves the remote internally
inconsistent — clips referencing videos that were never pushed. There is no
retry on a stale SHA, so a second device syncing at the same moment just shows
"Sync Error".

**4. [P0] Any non-404 read failure is treated as "the remote is empty".**
`sync_engine.rs:45`–`:67` collapses every error into `(Vec::new(), None)`, and
malformed JSON is swallowed by `unwrap_or_default()`. An expired token, a 500,
or a truncated file is indistinguishable from a fresh repository, and the local
DB is then treated as the whole truth.

**5. [P0] Re-sharing a video you already have wipes its state.**
`SharedVideoDialog.svelte:30` builds a fresh `Video` with `last_position: 0`,
`folder_id: null`, `start_time: 0`, and `save_video` is `INSERT OR REPLACE`
(`db.rs:516`). Share a video from YouTube that's already in your library and you
lose your watch position, its folder, and its trim range.

**6. [P0] Import rebuilds the folder tree wrong.**
`db.rs:779` inserts folders with `INSERT OR IGNORE` and *without* their ids, so
the `folder_id` values on the imported videos point at whatever rowids happen to
exist. Re-importing the same backup duplicates every folder and clip.

**7. [P1] Tombstones are immortal and backups resurrect the dead.**
Soft deletes are never purged, so deleted rows accumulate in the synced JSON
forever; `export_db` filters them out (`db.rs:689`) while `import_db` re-inserts
the survivors, so a backup → import round trip brings deleted items back on the
next sync.

**8. [P1] The device id is regenerated every launch.**
`db.rs:261` creates a fresh UUID on each start and `INSERT OR IGNORE`s it, while
`DbState.device_id` holds the *new* one rather than the stored one. Any
per-device logic built on this — including the mailbox design in §1.4 — starts
broken.

### B. Player and correctness

**9. [P1] The YouTube player is never destroyed.**
`VideoPlayer.svelte:125`, `NativePlayer.svelte:114` — no `player.destroy()`.
Every navigation leaks an iframe and its timers, and audio can continue after
you've gone back to the library.

**10. [P1] The iframe API callback is a global that outlives the component.**
`VideoPlayer.svelte:31` assigns `window.onYouTubeIframeAPIReady = initPlayer`.
Unmount before the script loads and the callback fires into a destroyed
component, calling `new YT.Player("player")` on an element that's gone. Both
players also hard-code their container ids, so two can never coexist.

**11. [P1] Exit writes a stale row over fresh data.**
`VideoPlayer.svelte:127` mutates the `video` prop and saves the entire object on
destroy. If a sync updated that video's title, folder, or trim while it was
playing, those changes are reverted.

**12. [P1] "Sync successful!" is shown when sync failed.**
`triggerSync` catches its own errors and never rethrows (`state.svelte.ts:166`),
and returns silently when the token or repo is missing (`:143`), while
`SettingsModal.svelte:213` chains `.then(() => alert("Sync successful!"))`. Every
outcome — success, failure, doing nothing at all — reports success.

**13. [P1] GitHub settings are live but unsaved.**
`SettingsModal.svelte:192` and `:207` bind the repo URL and token straight into
app state; only "Save Changes" writes to storage. Cancel and they stay active
for the session and vanish on restart.

**14. [P1] Two different metadata paths, one of them third-party.**
`AddVideoModal.svelte:37` fetches `noembed.com` from the webview, while
`fetch_video_oembed` (`db.rs:1060`) already does this in Rust and is what the
share dialog uses. The webview path depends on an unrelated service staying up
and on CORS.

**15. [P1] URL parsing rejects most YouTube URL shapes.**
`AddVideoModal.svelte:21` handles only `?v=` and `youtu.be/`. Shorts, `/embed/`,
`/live/`, `m.youtube.com`, and URLs with extra query parameters all fail — even
though `extract_video_id` in Rust (`db.rs:985`) already handles them.

**16. [P1] Thumbnails 404 with no fallback.**
`AddVideoModal.svelte:33` hard-codes `maxresdefault.jpg`, which doesn't exist for
many videos, and there's no `onerror` fallback to `hqdefault.jpg` — the row shows
a broken image forever.

**17. [P2] The number next to a video means nothing.**
`Dashboard.svelte:536` shows `last_position` if non-zero, else `duration` — which
is *never* populated (always 0, nothing fetches it). So the column is either your
watch position or blank, unlabelled, and reads like a runtime.

**18. [P2] Every new folder gets `sort_order: 0`.**
`Dashboard.svelte:74`. Ordering among new folders is arbitrary until you drag one.

**19. [P2] The delete confirmation describes the wrong behaviour.**
`Dashboard.svelte:95` says "move items to root"; `delete_folder` re-parents to
the *grandparent* (`db.rs:430`).

**20. [P2] Clip reordering is off by one and leaks duplicate orders.**
`ClipList.svelte:56` splices the dragged clip out *before* finding the target
index, so dropping above vs below an item differ by one position. `moveClipUp` /
`moveClipDown` (`:14`–`:38`) write index-derived `sort_order` values for two rows
only, leaving the rest of the list with stale or duplicate orders.

**21. [P2] The two players disagree about "ended".**
`VideoPlayer.svelte:103` treats the ended state as *not* paused, so no overlay
appears; `NativePlayer.svelte:72` treats anything that isn't "playing" as paused.
Same product, two behaviours — a symptom of the duplication §2.1 removes.

**22. [P2] Back does nothing if the library hasn't loaded.**
`state.svelte.ts:108` looks the video up in `this.videos` and silently skips when
it isn't found, leaving the player open while the path underneath it changed.

**23. [P1] There is no way to seek.**
No scrubber exists, and `VideoPlayer.svelte:44` sets `disablekb: 1` and `fs: 0`,
which removes YouTube's own keyboard control and fullscreen without replacing
them. Marking a clip at 42:00 means watching to 42:00.

**24. [P2] The clipboard template mangles text containing `/n`.**
`ClipList.svelte:120` runs `.replace(/\/n/g, "\n")` over the whole template
*before* substituting `{url}` and `{title}`.

**25. [P2] `alert` / `confirm` / `prompt` throughout.**
Blocking, unstyleable, and on Android they render as system dialogs over a
full-screen app. Used for every confirmation, every error, and folder creation.

### C. Android

**26. [P1] Safe areas are declared and never wired up.**
`app.css:17` defines `--safe-top` / `--safe-bottom` as `0px` and nothing ever
sets them, `index.html:6` has no `viewport-fit=cover` (without which
`env(safe-area-inset-*)` returns 0 anyway), and `App.svelte:38` uses `100vh`.
Content sits under the status bar and gesture bar, and the layout jumps whenever
the system bars resize.

**27. [P1] Six of eight declared native methods don't exist.**
`global.d.ts:7` declares `startAudioService`, `stopAudioService`, `keepScreenOn`,
`releaseScreenOn`, `showYouTubePlayer`, `showYouTubePlayerByUrl`,
`hideYouTubePlayer`, `playYouTubeVideo`, `pauseYouTubeVideo`, `seekYouTubeVideo`,
`getYouTubePosition`. `MainActivity.kt:132` implements `openInRevanced` and
`onAppReady`. Anything written against that type type-checks and silently
no-ops.

**28. [P1] The ReVanced handoff drops your position.**
`MainActivity.kt:74` builds `https://www.youtube.com/watch?v=$videoId` with no
`&t=`. Hand off at 12:30, restart at 0:00.

**29. [P1] The library's structure is read-only on a phone.**
Moving and reordering videos and folders is HTML5 drag-and-drop only
(`Dashboard.svelte:411`, `:500`), which touch does not fire. There is no
alternative path — no "move to folder", no reorder mode.

**30. [P1] The hierarchy is invisible on a phone.**
`Dashboard.svelte:391` hides every column but the last below `md`, and the only
breadcrumb lives in a footer that wraps.

**31. [P1] The clips panel traps you.**
`NativePlayer.svelte:263` positions the panel `absolute` against `main`: it
covers the transport bar, has no scrim, and tapping outside doesn't dismiss it.

**32. [P1] One capability file, desktop schema, desktop permissions.**
`src-tauri/capabilities/default.json` references `gen/schemas/desktop-schema.json`
and requests `core:window:allow-minimize` / `maximize` / `start-dragging` with no
`platforms` key — for the Android build as well.

**33. [P2] The notification plugin is initialised and unused.**
`lib.rs:16` registers `tauri-plugin-notification`; there's no JS package, no
capability permission, and no caller. On Android it pulls `POST_NOTIFICATIONS`
into the manifest for nothing.

**34. [P2] Share only accepts `ACTION_SEND` + `text/plain`.**
`AndroidManifest.xml` — no `ACTION_VIEW` filter for YouTube links, so yClippy
can't be offered as an opener, and apps that share as `text/uri-list` are
invisible to it.

**35. [P2] Shared videos always land at the root.**
`SharedVideoDialog.svelte:37` hard-codes `folder_id: null` with no picker.

### D. Dead and unreachable code

**36. [P1] Video trimming is unreachable.**
`EditVideoModal` renders when `isEditVideoModalOpen` is true — and nothing in the
codebase ever sets it, or `videoToEdit`. `update_video_metadata` works fine and
cannot be invoked.

**37. [P1] Nothing can be renamed except a clip.**
`renameVideo` and `renameFolder` are imported at `Dashboard.svelte:12`–`:13` and
never called. Both backend commands work.

**38. [P1] Eight sync helpers would throw if anything called them.**
`getDeviceId`, `getSyncStatus`, `getPendingChangesCount`, `exportChanges`,
`importChanges`, `markChangesSynced`, `recordChange`, `resetSync` are exported
from `db.ts` and are **not registered** in the `invoke_handler`
(`lib.rs:22`–`:48`). `mark_changes_synced` exists in Rust (`db.rs:818`) and is
likewise unregistered; the other seven have no Rust side at all.

**39. [P2] A change-notification system with no listener.**
`db.ts:10` defines `notifyChange`, every mutation calls it, and
`setChangeListener` (`:163`) is never invoked.

**40. [P2] `Sidebar.svelte` is an unused component, still branded "Clipper".**
Never imported. `isClipModalOpen` and `isSidebarOpen` in `state.svelte.ts` exist
for it.

**41. [P2] Unused import.** `getCurrentWebview` at `ClipSaveModal.svelte:4`.

**42. [P2] A half-removed sync design is still in the schema.**
The `changes` table (`db.rs:186`) and the `Change` / `ChangeSet` / `ChangeType`
structs (`:22`–`:56`) are dead, and `save_folder` still carries the commented-out
`record_change` call (`:362`). Every one of these types is `#[allow(dead_code)]`,
which is how they stayed.

### E. Security and robustness

**43. [P0] The GitHub token lives in `localStorage` and in the DOM.**
`state.svelte.ts:65` writes the classic PAT into web storage; the settings input
holds it in the page. With `csp: null` (`tauri.conf.json:22`) and a YouTube
iframe in the same webview, nothing stops a script from reading it. yReviewy
already has the fix: token in Rust app-private config, only `token_present`
crosses the bridge.

**44. [P1] No content security policy at all.** `tauri.conf.json:22`.

**45. [P2] Over-broad filesystem scope.**
`capabilities/default.json:23` grants `fs:allow-read-text-file` over `{"path":
"**"}`. It isn't needed — the dialog plugin already adds picked files to the
runtime scope (`tauri-plugin-dialog/src/commands.rs:196`).

**46. [P1] A poisoned mutex or a failed config write takes the app down.**
Every DB command does `state.conn.lock().unwrap()`, and `save_config`
(`db.rs:82`) `expect`s on both serialize and write — a read-only directory or a
full disk panics the process.

**47. [P1] Switching the database file validates nothing.**
`set_db_path` (`db.rs:289`) swaps the live connection to any path the dialog
returns without checking that it's writable, that it's a yClippy database, or
offering to migrate the existing data.

### Not bugs, but worth deciding

- **No tests anywhere.** The sync merge and `extract_video_id` are pure functions
  with obvious table tests; the id migration in §1.4 should not ship without them.
- **`fetch_video_oembed` builds its URL by string concatenation** (`db.rs:1062`)
  without percent-encoding the inner URL. It works because the video id lands
  before the first `&`, which is luck rather than design.
- **A video can only live in one folder** — `videos.id` is the YouTube id and
  `folder_id` is a column on it. See §1.4.
- **`$lib` is aliased in `vite.config.ts` but has no `paths` entry in
  `tsconfig.json`**, so the alias `AGENTS.md` recommends would break type
  checking. All current imports are relative.
