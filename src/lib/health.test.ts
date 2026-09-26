import { describe, expect, it } from "vitest";

import { greeting, health, type HealthInput } from "./health";

const ok: HealthInput = { diskUsedPercent: 40, pressure: "normal", highThreats: 0, mediumThreats: 0, junkBytes: 1e9 };

describe("health", () => {
  it("is healthy when nothing is wrong", () => {
    expect(health(ok)).toMatchObject({ level: "good", label: "Healthy", headline: "Your Mac, in good shape.", page: null });
  });
  it("puts security threats first", () => {
    expect(health({ ...ok, highThreats: 2, diskUsedPercent: 99 }).reason).toMatch(/2 security issues/);
    expect(health({ ...ok, highThreats: 2 }).page).toBe("security");
    expect(health({ ...ok, diskUsedPercent: 97 }).page).toBe("files");
    expect(health({ ...ok, pressure: "warning" }).page).toBe("memory");
  });
  it("flags a nearly full disk", () => {
    expect(health({ ...ok, diskUsedPercent: 96 }).level).toBe("attention");
    expect(health({ ...ok, diskUsedPercent: 88 }).level).toBe("care");
  });
  it("flags memory pressure and lots of junk", () => {
    expect(health({ ...ok, pressure: "critical" }).level).toBe("attention");
    expect(health({ ...ok, pressure: "warning" }).level).toBe("care");
    expect(health({ ...ok, junkBytes: 20e9 }).level).toBe("care");
  });
  it("works before anything is loaded", () => {
    expect(health({ diskUsedPercent: null, pressure: null, highThreats: 0, mediumThreats: 0, junkBytes: null }).level).toBe("good");
  });
});

describe("greeting", () => {
  it("follows the time of day", () => {
    expect(greeting(new Date(2026, 0, 1, 9))).toBe("Good morning");
    expect(greeting(new Date(2026, 0, 1, 14))).toBe("Good afternoon");
    expect(greeting(new Date(2026, 0, 1, 20))).toBe("Good evening");
  });
});
