/** localStorage key: user chose "I'll do this later" for Full Disk Access. */
export const FDA_DISMISS_KEY = "fdaDismissed";

export type FdaUiState = "loading" | "granted" | "ask" | "dismissed";

/** Which Full Disk Access UI to show in Settings. */
export function fdaUiState(granted: boolean | null, dismissed: boolean): FdaUiState {
  if (granted === null) return "loading";
  if (granted) return "granted";
  if (dismissed) return "dismissed";
  return "ask";
}

/** Quiet Overview hint — only when access is missing and not dismissed. */
export function showOverviewFdaHint(granted: boolean | null, dismissed: boolean): boolean {
  return granted === false && !dismissed;
}

export function loadFdaDismissed(): boolean {
  try {
    return localStorage.getItem(FDA_DISMISS_KEY) === "1";
  } catch {
    return false;
  }
}

export function saveFdaDismissed(dismissed: boolean): void {
  try {
    if (dismissed) localStorage.setItem(FDA_DISMISS_KEY, "1");
    else localStorage.removeItem(FDA_DISMISS_KEY);
  } catch {
    /* private mode */
  }
}
