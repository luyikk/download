/**
 * DURL Download Manager – browser extension background service worker
 *
 * Flow:
 *   1. User clicks a download link in the browser.
 *   2. chrome.downloads.onCreated fires.
 *   3. We immediately cancel the browser download.
 *   4. GET http://127.0.0.1:19283/ping — check if DURL is running.
 *      a. If DURL is running:
 *           POST /download with {url, cookies, filename}
 *           → DURL shows its "New Download" dialog pre-filled.
 *      b. If DURL is NOT running:
 *           Re-start the download via chrome.downloads.download()
 *           so the browser handles it as normal.
 */

const APP_BASE = "http://127.0.0.1:19283";
const PING_TIMEOUT_MS = 100;

/**
 * Storage key used in chrome.storage.session to persist the fallback-URL set
 * across service-worker restarts within the same browser session.
 *
 * Background: MV3 service workers are terminated when idle and restarted on
 * demand.  An in-memory Set is wiped on every restart, so URLs we re-initiated
 * as browser-fallback downloads would be forgotten and intercepted again —
 * causing all previously-downloaded files to be re-sent when the browser wakes
 * up the service worker (e.g. while closing).  Storing the set in
 * chrome.storage.session keeps it alive for the entire browser session.
 */
const FALLBACK_URLS_KEY = "fallbackUrls";

// ── Helpers for persistent fallback-URL tracking ──────────────────────────────

async function getFallbackUrls() {
  const data = await chrome.storage.session.get(FALLBACK_URLS_KEY);
  return new Set(data[FALLBACK_URLS_KEY] ?? []);
}

async function hasFallbackUrl(url) {
  const set = await getFallbackUrls();
  return set.has(url);
}

async function addFallbackUrl(url) {
  const set = await getFallbackUrls();
  set.add(url);
  await chrome.storage.session.set({ [FALLBACK_URLS_KEY]: [...set] });
}

async function deleteFallbackUrl(url) {
  const set = await getFallbackUrls();
  set.delete(url);
  await chrome.storage.session.set({ [FALLBACK_URLS_KEY]: [...set] });
}

// ── Main download intercept ───────────────────────────────────────────────────

chrome.downloads.onCreated.addListener(async (downloadItem) => {
  const url = downloadItem.url;

  // Skip: data URIs, blob URLs, extension-internal URLs
  if (url.startsWith("data:") || url.startsWith("blob:") || url.startsWith("chrome-extension:")) {
    return;
  }

  // Skip downloads that are not freshly created.
  // When the browser closes (or restarts), Chrome can replay onCreated for
  // downloads that are in an "interrupted" or otherwise non-active state.
  // These are restorations of old items, not new user-initiated downloads.
  if (downloadItem.state !== "in_progress") {
    return;
  }

  // Skip fallback downloads that we re-initiated ourselves.
  // Use chrome.storage.session so the set survives service-worker restarts.
  if (await hasFallbackUrl(url)) {
    await deleteFallbackUrl(url);
    return;
  }

  // Cancel the browser's own download immediately so nothing saves to disk yet
  chrome.downloads.cancel(downloadItem.id, () => {
    // Ignore any "download not cancellable" errors (e.g. already finished)
    void chrome.runtime.lastError;
  });

  const filename = downloadItem.filename || guessFilename(url);
  const cookies  = await getCookieString(url);

  const durlRunning = await isDurlRunning();

  if (durlRunning) {
    await sendToDurl(url, cookies, filename);
  } else {
    // Fallback: let the browser download it normally.
    // Persist the URL so the next onCreated (for our own re-initiated download)
    // is recognised even if the service worker has been restarted in between.
    await addFallbackUrl(url);
    chrome.downloads.download({ url, filename: filename || undefined });
  }
});

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * Returns true if DURL is running (i.e. /ping returns 200 within timeout).
 */
async function isDurlRunning() {
  try {
    const res = await fetch(`${APP_BASE}/ping`, {
      method: "GET",
      signal: AbortSignal.timeout(PING_TIMEOUT_MS),
    });
    return res.ok;
  } catch {
    return false;
  }
}

/**
 * POST the download request to DURL.
 */
async function sendToDurl(url, cookies, filename) {
  try {
    await fetch(`${APP_BASE}/download`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ url, cookies, filename: filename || null }),
      signal: AbortSignal.timeout(3000),
    });
  } catch (e) {
    console.error("[DURL] Failed to send download request:", e);
  }
}

/**
 * Collect all cookies for the given URL and format them as a
 * "name=value; name=value" string.
 */
async function getCookieString(url) {
  try {
    const cookies = await chrome.cookies.getAll({ url });
    return cookies.map((c) => `${c.name}=${c.value}`).join("; ");
  } catch {
    return "";
  }
}

/**
 * Best-effort filename from URL.
 */
function guessFilename(url) {
  try {
    const u = new URL(url);
    const parts = u.pathname.split("/");
    return decodeURIComponent(parts[parts.length - 1] || "");
  } catch {
    return "";
  }
}
