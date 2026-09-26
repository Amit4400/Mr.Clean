import { describe, expect, it } from "vitest";

import { isProtected } from "./paths";

// Mirrors crates/core/src/safety.rs `user_file_policy` test: the UI must never
// offer a checkbox the Rust guard would refuse.
const HOME = "/Users/me";

describe("isProtected", () => {
  it("allows ordinary user files", () => {
    expect(isProtected(`${HOME}/Downloads/big.dmg`, false, HOME)).toBe(false);
    expect(isProtected(`${HOME}/Documents/work/report.pdf`, false, HOME)).toBe(false);
    expect(isProtected(`${HOME}/Library/Caches/Foo`, true, HOME)).toBe(false);
    expect(isProtected(`${HOME}/Library/Developer/Xcode/DerivedData`, true, HOME)).toBe(false);
  });
  it("refuses top-level home folders", () => {
    expect(isProtected(`${HOME}/Documents`, true, HOME)).toBe(true);
    expect(isProtected(`${HOME}/Library`, true, HOME)).toBe(true);
  });
  it("refuses credentials, app data and git internals", () => {
    expect(isProtected(`${HOME}/.ssh/id_ed25519`, false, HOME)).toBe(true);
    expect(isProtected(`${HOME}/.aws/credentials`, false, HOME)).toBe(true);
    expect(isProtected(`${HOME}/Library/Preferences/x.plist`, false, HOME)).toBe(true);
    expect(isProtected(`${HOME}/Library/Application Support/App/db`, false, HOME)).toBe(true);
    expect(isProtected(`${HOME}/code/app/.git/objects/pack/p.pack`, false, HOME)).toBe(true);
  });
  it("refuses anything outside home, and look-alike prefixes", () => {
    expect(isProtected("/System/Library/x", false, HOME)).toBe(true);
    expect(isProtected("/Users/meother/file", false, HOME)).toBe(true);
  });
  it("handles Windows paths the same way", () => {
    const WH = "C:\\Users\\me";
    expect(isProtected(`${WH}\\Downloads\\setup.exe`, false, WH)).toBe(false);
    expect(isProtected(`${WH}\\AppData\\Local\\Temp\\x`, true, WH)).toBe(false);
    expect(isProtected(`${WH}\\AppData\\Roaming\\Slack`, true, WH)).toBe(true);
    expect(isProtected(`${WH}\\NTUSER.DAT`, false, WH)).toBe(true);
    expect(isProtected(`${WH}\\Documents`, true, WH)).toBe(true);
    expect(isProtected(`c:\\users\\ME\\Downloads\\a.zip`, false, WH)).toBe(false);
    expect(isProtected("C:\\Windows\\System32\\x.dll", false, WH)).toBe(true);
  });
});
