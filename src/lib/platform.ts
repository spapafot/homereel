/** WKWebView on macOS reports `Macintosh` in its user agent; WebView2 on Windows doesn't. */
export const IS_MAC = navigator.userAgent.includes('Mac');

/** The system's file browser, which "Show in folder" opens. */
export const FILE_MANAGER_NAME = IS_MAC ? 'Finder' : 'File Explorer';

/** Where the Rust side keeps secrets such as the Gemini API key. */
export const SECRET_STORE_NAME = IS_MAC ? 'your Mac’s Keychain' : 'Windows Credential Manager';
