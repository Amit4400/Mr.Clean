import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { FDA_DISMISS_KEY, fdaUiState, loadFdaDismissed, saveFdaDismissed, showOverviewFdaHint } from "./fda";

describe("fdaUiState", () => {
  it("stays loading until the check returns", () => {
    expect(fdaUiState(null, false)).toBe("loading");
    expect(fdaUiState(null, true)).toBe("loading");
  });

  it("shows granted and never asks when access is on", () => {
    expect(fdaUiState(true, false)).toBe("granted");
    expect(fdaUiState(true, true)).toBe("granted");
  });

  it("asks only when denied and not dismissed", () => {
    expect(fdaUiState(false, false)).toBe("ask");
    expect(fdaUiState(false, true)).toBe("dismissed");
  });
});

describe("showOverviewFdaHint", () => {
  it("only hints when access is missing and the user has not dismissed", () => {
    expect(showOverviewFdaHint(false, false)).toBe(true);
    expect(showOverviewFdaHint(false, true)).toBe(false);
    expect(showOverviewFdaHint(true, false)).toBe(false);
    expect(showOverviewFdaHint(null, false)).toBe(false);
  });
});

describe("fda dismiss persistence", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: (k: string) => store.get(k) ?? null,
        setItem: (k: string, v: string) => {
          store.set(k, v);
        },
        removeItem: (k: string) => {
          store.delete(k);
        },
      },
    });
  });

  afterEach(() => {
    // @ts-expect-error cleanup node polyfill
    delete globalThis.localStorage;
  });

  it("round-trips through localStorage", () => {
    expect(loadFdaDismissed()).toBe(false);
    saveFdaDismissed(true);
    expect(loadFdaDismissed()).toBe(true);
    saveFdaDismissed(false);
    expect(loadFdaDismissed()).toBe(false);
    expect(localStorage.getItem(FDA_DISMISS_KEY)).toBeNull();
  });
});
