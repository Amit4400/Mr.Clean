import { describe, expect, it } from "vitest";

import { ago, baseName, bytes, duration, parentOf, percent, sepOf, tildify } from "./format";

describe("bytes", () => {
  it("uses decimal units like Finder", () => {
    expect(bytes(0)).toBe("0 B");
    expect(bytes(999)).toBe("999 B");
    expect(bytes(1_500)).toBe("1.5 KB");
    expect(bytes(38_000_000_000)).toBe("38.0 GB");
    expect(bytes(245_000_000_000)).toBe("245 GB");
  });
  it("handles bad input", () => {
    expect(bytes(-5)).toBe("0 B");
    expect(bytes(Number.NaN)).toBe("0 B");
  });
});

describe("ago", () => {
  const now = Date.now() / 1000;
  it("formats relative times", () => {
    expect(ago(null)).toBe("—");
    expect(ago(now - 10)).toBe("just now");
    expect(ago(now - 5 * 60)).toBe("5 min ago");
    expect(ago(now - 3 * 86400)).toBe("3 days ago");
    expect(ago(now - 800 * 86400)).toMatch(/years ago$/);
  });
});

describe("helpers", () => {
  it("duration", () => {
    expect(duration(90)).toBe("1m");
    expect(duration(3 * 3600 + 120)).toBe("3h 2m");
    expect(duration(2 * 86400 + 3600)).toBe("2d 1h");
  });
  it("percent", () => {
    expect(percent(1, 4)).toBe(25);
    expect(percent(5, 0)).toBe(0);
  });
  it("tildify", () => {
    expect(tildify("/Users/me/Library/Caches", "/Users/me")).toBe("~/Library/Caches");
    expect(tildify("/opt/x", "/Users/me")).toBe("/opt/x");
    expect(tildify("/Users/me/x", null)).toBe("/Users/me/x");
  });
});

describe("paths on every platform", () => {
  it("splits both separators", () => {
    expect(baseName("/Users/me/projects/shop")).toBe("shop");
    expect(baseName("C:\\Users\\me\\projects\\shop")).toBe("shop");
    expect(parentOf("/Users/me/projects/shop")).toBe("/Users/me/projects");
    expect(parentOf("C:\\Users\\me\\shop")).toBe("C:\\Users\\me");
    expect(sepOf("C:\\Users\\me")).toBe("\\");
    expect(sepOf("/home/me")).toBe("/");
  });
});
