# DriveTube

A Windows desktop app that shows the videos in your Google Drive in a YouTube-style interface.
Drive access is read-only. Favorites, Watch later, playlists, history and AI titles are stored
on this computer only.

## First-time setup

1. Install the toolchain (Node, Rust, Visual Studio Build Tools) and fetch ffmpeg. See
   [docs/setup.md → Toolchain](docs/setup.md#toolchain).
2. Put your Google OAuth client in `src-tauri/google-oauth-client.json`. See
   [docs/setup.md](docs/setup.md). Builds without it cannot connect to Drive.
3. Install the packages:

   ```bash
   npm install
   ```

## Run while developing

```bash
npm run dev
```

This opens the app with hot reload. Changes to the React code show up immediately; changes to
the Rust code rebuild and restart the window. Development builds also import `GEMINI_API_KEY`
from `.env` (see [docs/setup.md](docs/setup.md#gemini-api-key-ai-titles)).

## Build a new version to install

1. Close DriveTube and stop `npm run dev`. Only one DriveTube window can run at a time.
2. Optional: raise `version` in `src-tauri/tauri.conf.json` (for example `0.1.0` → `0.2.0`) so
   you can tell builds apart. The installer then upgrades the old version.
3. Check and build:

   ```bash
   npm run typecheck
   npm run lint
   npm test
   npm run build
   ```

   The build takes a few minutes. It produces the installer
   `src-tauri/target/release/bundle/nsis/DriveTube_<version>_x64-setup.exe` and the bare app
   `src-tauri/target/release/drivetube.exe` (with `ffmpeg.exe` next to it).

4. Run the setup file. It installs for your Windows user and needs no admin rights. If Windows
   shows "Windows protected your PC", choose **More info → Run anyway** (the installer is not
   code-signed).
5. Start **DriveTube** from the Start menu.

Installing a new version keeps your data. The Google sign-in and Gemini key stay in Windows
Credential Manager, so you don't need to sign in again.

| What                                  | Where                                                                  |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Favorites, playlists, history, titles | `%APPDATA%\com.drivetube.desktop\drivetube.sqlite3`                    |
| Thumbnail cache and logs              | `%LOCALAPPDATA%\com.drivetube.desktop\`                                |
| Google sign-in, Gemini key            | Windows Credential Manager, entries ending in `.com.drivetube.desktop` |

To back up your library, copy the `.sqlite3` file while DriveTube is closed.

## Commands

| Command             | What it does                              |
| ------------------- | ----------------------------------------- |
| `npm run dev`       | Run the app in development mode           |
| `npm run build`     | Build the release app and installer       |
| `npm test`          | Frontend tests (Vitest) and Rust tests    |
| `npm run typecheck` | TypeScript check                          |
| `npm run lint`      | oxlint and `cargo clippy` (warnings fail) |
| `npm run format`    | Prettier and `cargo fmt`                  |

## Troubleshooting

- **`failed to run 'cargo metadata' … program not found`**: the terminal was opened before Rust
  was installed. Open a new terminal, or restart VS Code.
- **Nothing happens when starting DriveTube**: another copy is already running, often
  `npm run dev`. It gets focus instead. Close it first.
- **The build fails because `binaries/ffmpeg-x86_64-pc-windows-msvc.exe` doesn't exist**, or
  in development frame thumbnails and AI titles fail: ffmpeg is missing. Run
  `powershell -ExecutionPolicy Bypass -File scripts/fetch-ffmpeg.ps1`, then build again.
- **Google asks you to sign in again every week**: the OAuth app is still in _Testing_. See
  [docs/setup.md](docs/setup.md).
