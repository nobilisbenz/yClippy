# AGENTS.md - Developer Guide for yClippy

This is a Tauri + Svelte 5 + TypeScript desktop application for managing YouTube video clips.
Uses SQLite for local storage via `rusqlite` and supports GitHub-based sync.

## Build Commands

### Frontend (Bun)
```bash
bun run dev              # Start dev server
bun run build            # Build for production
bun run preview          # Preview production build
bun run check            # Run Svelte type checking (svelte-check)
bun run check:watch      # Watch mode for type checking
```

### Tauri/Rust Backend
```bash
bun run tauri dev        # Start Tauri in dev mode
bun run tauri build      # Build Tauri app
bun run tauri build -- --debug  # Build debug version
```

### Running Single Tests
This project currently has no test suite. If tests are added:
- **Vitest**: `bun run test -- <test-file>` or `bunx vitest run <test-file>`
- **Rust**: `cargo test <test-name>`

## Code Style Guidelines

### General
- **No comments** unless explicitly required by the user
- Use **4 spaces** for indentation
- Use **semicolons** at end of statements

### TypeScript
- Use **explicit types** for function parameters and return types
- Use **interfaces** over types for public APIs (see `src/lib/db.ts`)
- Use **PascalCase** for types, interfaces, and classes
- Use **camelCase** for variables, functions, and properties

### Svelte 5 (Runes)
- Use `$state()` rune for reactive state (not `let` or `$:`)
- Use `$derived()` for computed values
- Use `$effect()` for side effects
- Access rune state without `.value` suffix

```typescript
// Good
class AppState {
    videos = $state<Video[]>([]);
    activeVideo = $state<Video | null>(null);
}

// Bad (Svelte 4 style)
let videos = [];
$: activeVideos = videos.filter(v => v.id);
```

### Component Structure
```svelte
<script lang="ts">
  // 1. Imports (alphabetical within groups)
  import Component from "./Component.svelte";
  import { helper } from "./helpers";
  import type { Props } from "./types";

  // 2. Types/interfaces (if needed)

  // 3. Props with defaults using $props()
  let { title = "Default", onSave }: { title?: string; onSave: () => void } = $props();

  // 4. State (runes)

  // 5. Effects
</script>

<!-- Template -->
<Component prop={value} />
```

### Component Props
- Use `$props()` rune with explicit type annotation
- Provide default values for optional props
- Destructure props directly: `let { prop1, prop2 } = $props()`

### Tailwind CSS
- Use Tailwind CSS v4 syntax
- Use arbitrary values with square brackets when needed
- Prefer utility classes over custom CSS

### Imports
- **Double quotes** for external packages (e.g., `@tauri-apps/api/core`)
- **Single quotes** for local imports (e.g., `'./lib/db'`)
- Use **$lib/** alias** for imports from `src/lib` (e.g., `'@lib/db'`)

```typescript
// Good - using $lib alias
import { getVideos } from '$lib/db';
```

### Android build notes

`src-tauri/gen/android` is tracked in this repository. `MainActivity.kt` is hand-written and `tauri android init` will clobber it. If you regenerate the Android project, re-apply:

- `yclippy://` deep-link `intent-filter` and `ACTION_VIEW` for `youtube.com`/`youtu.be`/`m.youtube.com`
- `text/*` (not just `text/plain`) `ACTION_SEND` filter
- The `yclippy://play?v=…&t=…` parsing in `handleViewIntent`
- The timestamp-aware `openInRevanced(videoId, startSeconds)` overload

The split `src-tauri/capabilities/desktop.json` and `mobile.json` are also committed — both are referenced by `tauri android build` and `cargo tauri build` respectively.
import type { Video } from '$lib/db';

// Good - relative imports also work
import { getVideos } from './lib/db';
```

### Error Handling
- Use `try/catch` for async operations
- Log errors with `console.error`
- Use descriptive error messages

```typescript
try {
    await someAsyncOperation();
} catch (e) {
    console.error("Failed to operation", e);
}
```

### Rust Backend
- Follow standard Rust conventions (rustfmt)
- Use `Result<T, Error>` for fallible functions
- Prefer `?` operator over `match` for simple error propagation

### Naming Conventions
| Type | Convention | Example |
|------|------------|---------|
| Svelte Components | PascalCase | `Dashboard.svelte` |
| TypeScript Files | kebab-case | `state.svelte.ts` |
| Functions | camelCase | `getVideos()` |
| Variables | camelCase | `activeVideo` |
| Interfaces/Types | PascalCase | `Video`, `Clip` |
| Database Tables | snake_case | `videos`, `clips` |
| Rust Modules | snake_case | `db.rs`, `github_api.rs` |

### File Organization
```
src/
├── lib/                    # Shared code
│   ├── db.ts              # Database types and functions
│   ├── state.svelte.ts    # Global app state (runes class)
│   ├── *.svelte           # UI components
├── App.svelte             # Root component
├── main.ts                # Entry point
└── app.css                # Global styles

src-tauri/
├── src/
│   ├── lib.rs             # Tauri app setup
│   ├── main.rs            # Main entry point
│   ├── db.rs              # SQLite operations
│   ├── sync.rs            # GitHub sync logic
│   └── github_api.rs      # GitHub API client
└── Cargo.toml
```

### Working with Tauri
- Use `@tauri-apps/api/core` `invoke()` for backend calls
- All Tauri commands are defined in `src-tauri/src/lib.rs`
- Database operations go in `src-tauri/src/db.rs`
- Frontend types/interfaces for database entities are in `src/lib/db.ts`

### State Management
- Global app state uses Svelte 5 runes in a class (`src/lib/state.svelte.ts`)
- Database changes trigger notifications via `setChangeListener()` callback
- Components subscribe to state changes for reactivity

### Git Workflow
- Create feature branches for new functionality
- Commit messages should be concise (under 72 chars for subject)
- Never commit secrets, keys, or credentials
