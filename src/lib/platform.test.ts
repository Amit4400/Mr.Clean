import { describe, expect, it } from "vitest";

import { detectPlatform, wordsFor } from "./platform";

describe("platform wording", () => {
  it("detects the OS from the webview user agent", () => {
    expect(detectPlatform("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Edg/128.0")).toBe("windows");
    expect(detectPlatform("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15")).toBe("mac");
    expect(detectPlatform("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15")).toBe("linux");
  });
  it("uses each system's own words", () => {
    expect(wordsFor("windows")).toMatchObject({ computer: "PC", trash: "Recycle Bin", fileManager: "File Explorer" });
    expect(wordsFor("mac")).toMatchObject({ computer: "Mac", trash: "Trash", fileManager: "Finder" });
    expect(wordsFor("linux").os).toBe("Linux");
  });
});
