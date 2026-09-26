import { describe, expect, it } from "vitest";

import { api, inTauri } from "./api";

describe("api outside the desktop app", () => {
  it("uses the mock backend", async () => {
    expect(inTauri).toBe(false);
    const info = await api.systemInfo();
    expect(info.memory.total_bytes).toBeGreaterThan(0);
    const scan = await api.cleanerScan();
    expect(scan.rules.length).toBeGreaterThan(0);
    expect(scan.safe_bytes).toBeLessThanOrEqual(scan.total_bytes);
  });
});
