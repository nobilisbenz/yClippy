# yClippy

still on work

A desktop application for managing YouTube video clips. Built with Tauri, Svelte 5, and TypeScript.

## Features

- Manage YouTube video clips locally
- SQLite database for persistent storage
- Desktop integration with Tauri

## Development

### Prerequisites

- [Bun](https://bun.sh/) (frontend package manager)
- [Rust](https://www.rust-lang.org/) (for Tauri)
- [Node.js](https://nodejs.org/) (for Tauri CLI)

### Setup

```bash
# Install frontend dependencies
bun install

# Run in development mode
bun run tauri dev
```

### Build

```bash
# Build for production
bun run tauri build
```

## Tech Stack

- **Frontend**: Svelte 5, TypeScript, Tailwind CSS v4
- **Backend**: Tauri (Rust)
- **Database**: SQLite

## License

MIT
