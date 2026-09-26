import type { Pressure } from "./types";

export type HealthLevel = "good" | "care" | "attention";

export interface Health {
  level: HealthLevel;
  /** Short pill text: "Healthy", "Needs care", "At risk". */
  label: string;
  /** Big headline on the Overview. */
  headline: string;
  /** One line explaining why (or the reassuring default). */
  reason: string;
}

export interface HealthInput {
  diskUsedPercent: number | null;
  pressure: Pressure | null;
  highThreats: number;
  mediumThreats: number;
  /** Bytes of safe-to-clean junk found by the last scan (null = not scanned). */
  junkBytes: number | null;
}

const GB = 1e9;

/** Decide how the Mac is doing, worst problem first. */
export function health(i: HealthInput): Health {
  const disk = i.diskUsedPercent ?? 0;
  if (i.highThreats > 0) {
    return { level: "attention", label: "At risk", headline: "Your Mac needs attention.", reason: `${i.highThreats} security issue${i.highThreats > 1 ? "s" : ""} found — review them in Security.` };
  }
  if (disk >= 95) {
    return { level: "attention", label: "At risk", headline: "Your Mac needs attention.", reason: `The disk is ${Math.round(disk)}% full, which slows everything down.` };
  }
  if (i.pressure === "critical") {
    return { level: "attention", label: "At risk", headline: "Your Mac needs attention.", reason: "Memory is under heavy pressure — close some apps." };
  }
  if (i.mediumThreats > 0) {
    return { level: "care", label: "Needs care", headline: "Your Mac needs a little care.", reason: `${i.mediumThreats} thing${i.mediumThreats > 1 ? "s" : ""} in Security worth a look.` };
  }
  if (disk >= 85) {
    return { level: "care", label: "Needs care", headline: "Your Mac needs a little care.", reason: `The disk is ${Math.round(disk)}% full.` };
  }
  if (i.pressure === "warning") {
    return { level: "care", label: "Needs care", headline: "Your Mac needs a little care.", reason: "Memory is getting tight." };
  }
  if ((i.junkBytes ?? 0) >= 10 * GB) {
    return { level: "care", label: "Needs care", headline: "Your Mac needs a little care.", reason: "There's a lot of developer junk you can clean." };
  }
  return { level: "good", label: "Healthy", headline: "Your Mac, in good shape.", reason: "Clean, secure and ready for what you build next." };
}

export function greeting(date = new Date()): string {
  const h = date.getHours();
  if (h < 5) return "Good night";
  if (h < 12) return "Good morning";
  if (h < 18) return "Good afternoon";
  return "Good evening";
}
