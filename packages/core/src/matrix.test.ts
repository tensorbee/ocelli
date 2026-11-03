import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { certifyRow } from "./matrix.js";
import type { MatrixInput, MatrixTier } from "./matrix.js";

const MATRIX = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../../ci/browser-matrix.json",
);

const NAMED: readonly { id: string; expectedTier: MatrixTier }[] = [
  { id: "chromium-webgl2", expectedTier: "B" },
  { id: "chromium-swiftshader", expectedTier: "C" },
  { id: "firefox", expectedTier: "B" },
  { id: "safari", expectedTier: "B" },
  { id: "enterprise-locked", expectedTier: "B" },
];

function isTier(value: unknown): value is MatrixTier {
  return value === "A" || value === "B" || value === "C";
}

function rowsFrom(text: string): MatrixInput[] {
  const parsed: unknown = JSON.parse(text);
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("rows" in parsed) ||
    !Array.isArray(parsed.rows)
  ) {
    return [];
  }
  const rows: MatrixInput[] = [];
  for (const row of parsed.rows) {
    if (typeof row !== "object" || row === null) {
      return [];
    }
    if (!("id" in row) || typeof row.id !== "string") {
      return [];
    }
    if (!("expectedTier" in row) || !isTier(row.expectedTier)) {
      return [];
    }
    const input: MatrixInput = { id: row.id, expectedTier: row.expectedTier };
    if ("status" in row && typeof row.status === "string") {
      input.status = row.status;
    }
    if ("observation" in row && row.observation !== undefined) {
      const observation = row.observation;
      if (typeof observation !== "object" || observation === null) {
        return [];
      }
      if (!("renderer" in observation) || typeof observation.renderer !== "string") {
        return [];
      }
      if (!("resolvedTier" in observation) || !isTier(observation.resolvedTier)) {
        return [];
      }
      if (!("exitCode" in observation) || typeof observation.exitCode !== "number") {
        return [];
      }
      input.observation = {
        renderer: observation.renderer,
        resolvedTier: observation.resolvedTier,
        exitCode: observation.exitCode,
      };
    }
    rows.push(input);
  }
  return rows;
}

describe("browser matrix", () => {
  it("certifies SwiftShader only as tier C", () => {
    expect(
      certifyRow({
        id: "chromium-swiftshader",
        expectedTier: "C",
        observation: {
          renderer: "Google SwiftShader",
          resolvedTier: "C",
          exitCode: 0,
        },
      }),
    ).toBe("certified");
  });

  it("certifies a hardware WebGL2 row as tier B", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        observation: {
          renderer: "ANGLE (Apple, Metal)",
          resolvedTier: "B",
          exitCode: 0,
        },
      }),
    ).toBe("certified");
  });

  it("returns null when SwiftShader is recorded as tier B", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        observation: {
          renderer: "Google SwiftShader",
          resolvedTier: "B",
          exitCode: 0,
        },
      }),
    ).toBeNull();
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        status: "certified",
        observation: {
          renderer: "Google SwiftShader",
          resolvedTier: "B",
          exitCode: 1,
        },
      }),
    ).toBeNull();
  });

  it("stays not-run when the observation is missing", () => {
    expect(
      certifyRow({
        id: "firefox",
        expectedTier: "B",
        status: "certified",
      }),
    ).toBe("not-run");
  });

  it("records a failed probe as observed", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        status: "certified",
        observation: {
          renderer: "ANGLE (Apple, Metal)",
          resolvedTier: "B",
          exitCode: 1,
        },
      }),
    ).toBe("observed");
  });

  it("does not certify tier C without SwiftShader", () => {
    expect(
      certifyRow({
        id: "chromium-swiftshader",
        expectedTier: "C",
        observation: {
          renderer: "llvmpipe",
          resolvedTier: "C",
          exitCode: 0,
        },
      }),
    ).toBe("observed");
  });

  it("returns null for SwiftShader on expected tier B when the resolved tier differs", () => {
    for (const resolvedTier of ["A", "C"] as const) {
      expect(
        certifyRow({
          id: "chromium-webgl2",
          expectedTier: "B",
          observation: {
            renderer: "Google SwiftShader",
            resolvedTier,
            exitCode: 0,
          },
        }),
      ).toBeNull();
    }
  });

  it("does not certify a resolved tier that differs from the expected tier", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        observation: {
          renderer: "ANGLE (Apple, Metal)",
          resolvedTier: "C",
          exitCode: 0,
        },
      }),
    ).toBe("observed");
    expect(
      certifyRow({
        id: "chromium-swiftshader",
        expectedTier: "C",
        observation: {
          renderer: "Google SwiftShader",
          resolvedTier: "B",
          exitCode: 0,
        },
      }),
    ).toBe("observed");
  });

  it("keeps a certifying observation when the caller status says not-run", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        status: "not-run",
        observation: {
          renderer: "ANGLE (Apple, Metal)",
          resolvedTier: "B",
          exitCode: 0,
        },
      }),
    ).toBe("certified");
  });

  it("does not certify an empty renderer or a tier A row", () => {
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "B",
        observation: {
          renderer: "",
          resolvedTier: "B",
          exitCode: 0,
        },
      }),
    ).toBe("observed");
    expect(
      certifyRow({
        id: "chromium-webgl2",
        expectedTier: "A",
        observation: {
          renderer: "wgpu",
          resolvedTier: "A",
          exitCode: 0,
        },
      }),
    ).toBe("observed");
  });

  it("keeps the committed matrix not-run", () => {
    const text = readFileSync(MATRIX, "utf8");
    expect(text.includes("certified")).toBe(false);
    expect(text.includes("SwiftShader")).toBe(false);
    const rows = rowsFrom(text);
    expect(rows.map((row) => ({ id: row.id, expectedTier: row.expectedTier }))).toEqual([
      ...NAMED,
    ]);
    expect(rows).toHaveLength(5);
    for (const row of rows) {
      expect(row.observation).toBeUndefined();
      expect(row.status).toBe("not-run");
      expect(certifyRow(row)).toBe("not-run");
    }
  });
});
