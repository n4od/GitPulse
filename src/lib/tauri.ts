/**
 * True when the page is running inside the Tauri webview.
 *
 * Outside it — `vite dev` in a plain browser, or a component test — there is no
 * IPC to talk to, so live-update bridges stay idle rather than logging a caught
 * error on every mount.
 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
