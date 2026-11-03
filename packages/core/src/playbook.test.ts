import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { nextMigrationKind } from "./playbook.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const LLD = readFileSync(
  join(HERE, "../../../docs/lld/playbook.md"),
  "utf8",
);

describe("nextMigrationKind", () => {
  it("starts at stack", () => {
    expect(nextMigrationKind([])).toBe("stack");
  });

  it("walks stack, then mpr, then volume", () => {
    expect(nextMigrationKind(["stack"])).toBe("mpr");
    expect(nextMigrationKind(["stack", "mpr"])).toBe("volume");
    expect(nextMigrationKind(["stack", "mpr", "volume"])).toBeNull();
  });

  it("returns null for a skip, a duplicate or an unknown name", () => {
    expect(nextMigrationKind(["mpr"])).toBeNull();
    expect(nextMigrationKind(["volume"])).toBeNull();
    expect(nextMigrationKind(["stack", "volume"])).toBeNull();
    expect(nextMigrationKind(["stack", "stack"])).toBeNull();
    expect(nextMigrationKind(["oblique"])).toBeNull();
  });

  it("names the same order in the playbook", () => {
    expect(LLD.includes("The order is stack, then mpr, then volume.")).toBe(
      true,
    );
  });
});
