import { describe, expect, it } from "vitest";

import { chipOf, toolOf } from "./tools";

describe("clean page chips and icons", () => {
  it("puts rules under the right chip", () => {
    expect(chipOf("xcode-derived-data", "xcode")).toBe("xcode");
    expect(chipOf("simulator-runtimes", "xcode")).toBe("xcode");
    expect(chipOf("npm-cache", "java_script")).toBe("javascript");
    expect(chipOf("pip", "languages")).toBe("python");
    expect(chipOf("cargo", "languages")).toBe("rust");
    expect(chipOf("jetbrains", "tools")).toBe("ide");
    expect(chipOf("user-logs", "system")).toBe("logs");
    expect(chipOf("docker", "tools")).toBe("other");
    expect(chipOf("go-mod", "languages")).toBe("other");
  });

  it("picks a brand logo when there is one", () => {
    expect(toolOf("xcode-archives")).toBe("xcode");
    expect(toolOf("android-avd")).toBe("android");
    expect(toolOf("gradle-caches")).toBe("gradle");
    expect(toolOf("yarn-cache")).toBe("yarn");
    expect(toolOf("user-caches")).toBeNull();
    expect(toolOf("trash")).toBeNull();
  });
});
