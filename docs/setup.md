# Homereel setup

## Google Cloud OAuth client (one time, about 10 minutes)

Homereel reads your Drive with your own Google Cloud OAuth client. Nothing is hosted.

1. Open <https://console.cloud.google.com> and create a project named **Homereel**.
2. **APIs & Services → Library**: search for **Google Drive API** and click **Enable**.
3. **Google Auth Platform → Branding**: app name `Homereel`, your email as support and
   developer contact. Save.
4. **Audience**: user type **External**. Under **Test users**, add your own Google account.
   This is enough to start. While the app is in _Testing_, Google expires the sign-in every
   7 days, so you will need to reconnect weekly.

   For daily use, click **Publish app** so the status reads **In production**. Google requires
   a homepage URL and a privacy policy URL on the **Branding** page first (for example two
   pages on GitHub Pages, with that domain under **Authorized domains**). Do not submit for
   verification; personal use (under 100 users) is exempt.

5. **Data Access → Add or remove scopes**: add
   `https://www.googleapis.com/auth/drive.readonly` and save. Google lists it as a restricted
   scope; that is expected.
6. **Clients → Create client**: application type **Desktop app**, name `Homereel desktop`.
   Click **Download JSON** right away. The secret is shown only once.
7. Copy the downloaded file to `src-tauri/google-oauth-client.json`. It is gitignored. Never
   commit it.
8. Restart `npm run dev`. The build embeds the client from that file.

The first time you connect, Google shows **"Google hasn't verified this app"**. Click
**Advanced → Go to Homereel (unsafe) → Continue**, then allow access to your Drive files.

### Things that end a connection

- You revoke access at <https://myaccount.google.com/permissions>.
- The app is not used for 6 months (the refresh token expires).
- Google deletes OAuth clients that have had no sign-ins for 6 months. You can restore one
  within 30 days from the Clients page.

In each case, use **Connect Google Drive** again.

## Gemini API key (AI titles)

1. Create a key at <https://aistudio.google.com> in a project with **billing enabled**. On the
   free tier, Google may use the submitted frames to improve its products.
2. In Homereel, open **Settings → AI titles**, paste the key and click **Save key**. It is
   checked with Gemini and stored in Windows Credential Manager or the macOS keychain, never in
   a file.
3. Click **Generate missing titles**. Each video is titled once (about $0.002 per video with
   `gemini-3.8-flash`); rerunning skips videos that already have a title. Drive files are never
   renamed.

Development builds also import `GEMINI_API_KEY` from the project's `.env` into the credential
store on first launch. Release builds never read `.env`.

## Toolchain

### Windows

- Node 24+, npm
- Rust stable (MSVC) through rustup, plus Visual Studio 2022 Build Tools with the C++ workload
- WebView2 Runtime (preinstalled on Windows 11)
- ffmpeg sidecar: `powershell -ExecutionPolicy Bypass -File scripts/fetch-ffmpeg.ps1`

### macOS (Apple Silicon)

- Node 24+, npm
- Xcode Command Line Tools: `xcode-select --install`
- Rust stable, through Homebrew (`brew install rust`) or rustup
- ffmpeg sidecar: `bash scripts/fetch-ffmpeg.sh`

The same Google OAuth client works on both systems. Copy `src-tauri/google-oauth-client.json`
to each computer you build on.
