# Homereel

A Windows desktop app that shows your own videos in a YouTube-style interface: those in
Google Drive™ (all of it, or one folder and its subfolders), in folders on this computer, or both. Choose them
in **Settings → Video library**. Files are only read, never changed. Favorites, Watch later,
playlists, history and AI titles are stored on this computer only.

## First-time setup

1. Install the toolchain (Node, Rust, Visual Studio Build Tools) and fetch ffmpeg. See
   [docs/setup.md → Toolchain](docs/setup.md#toolchain).
2. Put your Google OAuth client in `src-tauri/google-oauth-client.json`. See
   [docs/setup.md](docs/setup.md). Builds without it cannot connect to Drive, but still play
   videos from folders on the computer.
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

1. Close Homereel and stop `npm run dev`. Only one Homereel window can run at a time.
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
   `src-tauri/target/release/bundle/nsis/Homereel_<version>_x64-setup.exe` and the bare app
   `src-tauri/target/release/homereel.exe` (with `ffmpeg.exe` next to it).

4. Run the setup file. It installs for your Windows user and needs no admin rights. If Windows
   shows "Windows protected your PC", choose **More info → Run anyway** (the installer is not
   code-signed).
5. Start **Homereel** from the Start menu.

Installing a new version keeps your data. The Google sign-in and Gemini key stay in Windows
Credential Manager, so you don't need to sign in again.

| What                                  | Where                                                                 |
| ------------------------------------- | --------------------------------------------------------------------- |
| Favorites, playlists, history, titles | `%APPDATA%\com.homereel.desktop\homereel.sqlite3`                     |
| Thumbnail cache and logs              | `%LOCALAPPDATA%\com.homereel.desktop\`                                |
| Google sign-in, Gemini key            | Windows Credential Manager, entries ending in `.com.homereel.desktop` |

To back up your library, copy the `.sqlite3` file while Homereel is closed.

### Coming from DriveTube

Homereel was called DriveTube in version 0.1.0. On its first start, Homereel moves DriveTube's
library, thumbnails, Google sign-in and Gemini key to its own names, so nothing is lost. Close
DriveTube first: while it is open, its files can't be moved, and Homereel says so and quits.

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
- **Nothing happens when starting Homereel**: another copy is already running, often
  `npm run dev`. It gets focus instead. Close it first.
- **The build fails because `binaries/ffmpeg-x86_64-pc-windows-msvc.exe` doesn't exist**, or
  in development frame thumbnails and AI titles fail: ffmpeg is missing. Run
  `powershell -ExecutionPolicy Bypass -File scripts/fetch-ffmpeg.ps1`, then build again.
- **Google asks you to sign in again every week**: the OAuth app is still in _Testing_. See
  [docs/setup.md](docs/setup.md).

## License

Homereel is released under the [MIT License](LICENSE).

It uses this third-party software under its own licenses:

| Software                                           | License    | Notes                                                                                  |
| -------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------- |
| [ffmpeg](https://ffmpeg.org) (gyan.dev build)      | GPL v3     | Not in this repository: `scripts/fetch-ffmpeg.ps1` downloads it; installers include it |
| [Roboto](https://fonts.google.com/specimen/Roboto) | OFL 1.1    | Through `@fontsource/roboto`                                                           |
| [Material Symbols](https://fonts.google.com/icons) | Apache 2.0 | Through `material-symbols`                                                             |

Homereel runs ffmpeg as its own process and does not link to it. If you share installers you
built, include ffmpeg's license and say where its source is: the exact version is in
`src-tauri/binaries/FFMPEG_VERSION.txt`.

## Trademarks

Homereel is an independent project. It is not affiliated with, endorsed by or sponsored by
Google or YouTube. Google Drive and YouTube are trademarks of Google LLC.
