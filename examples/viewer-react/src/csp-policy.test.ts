import { spawnSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import {
  EMBEDDING_POLICY,
  FORBIDDEN_SOURCES,
  policyHeader,
} from "./csp-policy.js";

/**
 * F-228, acceptance rows AC1, AC2, AC9 and AC10.
 *
 * Expected values are written from the design plan, never from the module's
 * output. AC1 spells the header and the forbidden sources out in full, so a
 * policy grown by a source the plan refuses, or a refusal list that shrank,
 * cannot pass by matching a helper.
 */

const ROOT = fileURLToPath(new URL("../../../", import.meta.url));

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8");
}

/** The four directives of plan item 1, in order, spelled out. */
const FOUR_DIRECTIVES = [
  "default-src 'none'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "worker-src 'self'",
  "connect-src 'self'",
];

/**
 * The forbidden sources of the design plan's scope section, spelled out here
 * rather than read from the module, so shrinking the module's list cannot
 * shrink the test with it.
 */
const PLAN_FORBIDDEN = [
  "'unsafe-inline'",
  "'unsafe-eval'",
  "'unsafe-hashes'",
  "'strict-dynamic'",
  "blob:",
  "data:",
  "*",
];

describe("the embedding policy", () => {
  it("is exactly the four directives", () => {
    expect(policyHeader()).toBe(FOUR_DIRECTIVES.join("; "));
    expect(EMBEDDING_POLICY.map(([directive]) => directive)).toEqual([
      "default-src",
      "script-src",
      "worker-src",
      "connect-src",
    ]);
  });

  it("forbids exactly the sources the plan refuses", () => {
    expect([...FORBIDDEN_SOURCES]).toEqual(PLAN_FORBIDDEN);
  });

  it("contains no forbidden source", () => {
    const header = policyHeader();
    for (const forbidden of PLAN_FORBIDDEN) {
      expect(header.includes(forbidden)).toBe(false);
    }
  });

  it("adds a DICOMweb origin to connect-src only", () => {
    expect(policyHeader(["https://pacs.example"])).toBe(
      `${FOUR_DIRECTIVES.join("; ")} https://pacs.example`,
    );
  });

  it("refuses a forbidden source", () => {
    for (const forbidden of PLAN_FORBIDDEN) {
      expect(() => policyHeader([forbidden])).toThrow(/forbidden/);
    }
    expect(() =>
      policyHeader(["https://pacs.example", "blob:https://example.invalid"]),
    ).toThrow(/forbidden/);
  });

  it("refuses a scheme-only source", () => {
    expect(() => policyHeader(["https:"])).toThrow(/scheme-only/);
    expect(() => policyHeader(["http:"])).toThrow(/scheme-only/);
  });

  it("refuses an entry that would widen the header structurally", () => {
    expect(() =>
      policyHeader(["https://pacs.example; script-src https://evil.example"]),
    ).toThrow(/';'/);
    expect(() =>
      policyHeader(["https://pacs.example, https://evil.example"]),
    ).toThrow(/','/);
    expect(() =>
      policyHeader(["https://pacs.example img-src https://evil.example"]),
    ).toThrow(/whitespace/);
    expect(() => policyHeader(["https://pacs.example'"])).toThrow(/quote/);
    expect(() => policyHeader(['https://pacs.example"'])).toThrow(/quote/);
  });

  it("refuses an entry that differs from its trimmed form", () => {
    expect(() => policyHeader(["https://pacs.example\n"])).toThrow(
      /leading or trailing whitespace/,
    );
    expect(() => policyHeader([" https://pacs.example"])).toThrow(
      /leading or trailing whitespace/,
    );
    expect(() => policyHeader(["\thttps://pacs.example"])).toThrow(
      /leading or trailing whitespace/,
    );
    expect(() => policyHeader(["https://pacs.example "])).toThrow(
      /leading or trailing whitespace/,
    );
  });

  it("refuses a control character inside an entry", () => {
    expect(() => policyHeader(["https://pacs.example\u0000"])).toThrow(
      /control character/,
    );
    expect(() => policyHeader(["https://pacs.example\u0085/path"])).toThrow(
      /control character/,
    );
    expect(() => policyHeader(["https://pacs.example\u0007x"])).toThrow(
      /control character/,
    );
  });
});

/**
 * The marker that puts a quoted policy under the AC2 check. A document
 * quotes the policy in a fenced block preceded, on its own line, by the
 * marker. A marked block is an opening ```text fence or HTML <pre>, one header
 * line and its corresponding closing delimiter.
 */
const CSP_MARKER = "<!-- ocelli:csp -->";

describe("every document quotes the one policy", () => {
  it("every marked block is the exact policy text", () => {
    // F-228's amendment to AC2, after review pass 4. Each document quotes
    // the policy in fenced blocks marked on their own line with
    // `<!-- ocelli:csp -->`. A marked block has its opening delimiter,
    // one header line and its closing delimiter, and the test holds the
    // k-th marked block's header line to the k-th entry of the document's
    // ordered list of expected headers, each document to exactly that
    // number of marked blocks.
    const expectations: readonly (readonly [string, readonly string[]])[] = [
      ["README.md", [policyHeader(), policyHeader(["https://pacs.example"])]],
      ["docs/lld/typescript-packaging.md", [policyHeader()]],
      ["examples/folder-viewer/src/use.html", [policyHeader()]],
    ];
    const findings: string[] = [];
    for (const [name, expected] of expectations) {
      const lines = read(name).split("\n");
      const expectedHeaders = expected.map(
        (policy) => `Content-Security-Policy: ${policy}`,
      );
      let marked = 0;
      for (let index = 0; index < lines.length; index += 1) {
        if (lines[index] !== CSP_MARKER) {
          continue;
        }
        const where = `${name}:${index + 1}`;
        const opening = lines[index + 1] ?? "";
        const expectedOpening = name.endsWith(".html") ? "<pre>" : "```text";
        const expectedClosing = name.endsWith(".html") ? "</pre>" : "```";
        if (opening !== expectedOpening) {
          findings.push(
            `${where}: the marker is not followed by ${expectedOpening}: ${opening}`,
          );
          continue;
        }
        marked += 1;
        const header = lines[index + 2];
        const expectedHeader = expectedHeaders[marked - 1];
        if (expectedHeader === undefined) {
          findings.push(
            `${where}: marked block ${marked} has no expected header, the document holds exactly ${expectedHeaders.length}`,
          );
        } else if (header !== expectedHeader) {
          findings.push(
            `${where}: marked block ${marked}'s header line is not the expected header: ${header ?? "(the document ends first)"}`,
          );
        }
        const closing = lines[index + 3];
        if (closing !== expectedClosing) {
          findings.push(
            `${where}: the marked block does not close after its one header line: ${closing ?? "(the document ends first)"}`,
          );
        }
      }
      if (marked !== expectedHeaders.length) {
        findings.push(
          `${name}: found ${marked} marked block${marked === 1 ? "" : "s"}, expected exactly ${expectedHeaders.length}`,
        );
      }
    }
    expect(
      findings,
      "F-228's amendment to AC2. Every marked block must have its matching opening and closing delimiters around one header line, the header line the expected header for that block, and each document must hold exactly its expected number of marked blocks.",
    ).toEqual([]);
  });
});

describe("the wasm module still decodes nothing", () => {
  it("no normal dependency edge of ocelli-wasm under every feature names ocelli-codec", () => {
    // The resolved graph, not the manifest text, so a dotted key, a renamed
    // dependency, a `[dependencies.ocelli-codec]` table and a transitive
    // edge through another crate all link the codec into the module all the
    // same. The scope is normal edges only, with every feature enabled:
    // `decode-worker` is the build `bin/ocelli.sh wasm-decode` names, so an
    // optional edge that feature enables is exactly the one that must fail
    // here, while a dev-dependency links nothing into the module and must
    // stay green. An empty result, no codec edge anywhere in the tree, is
    // clean.
    const probe = spawnSync(
      "cargo",
      [
        "tree",
        "-p",
        "ocelli-wasm",
        "--target",
        "wasm32-unknown-unknown",
        "-e",
        "normal",
        "--all-features",
        "--locked",
      ],
      { cwd: ROOT, encoding: "utf8" },
    );
    if (probe.error !== undefined) {
      throw new Error(
        `the AC9 guard could not run cargo tree: ${probe.error.message}`,
      );
    }
    if (probe.status === null) {
      throw new Error("the AC9 guard's cargo tree was killed by a signal");
    }
    if (probe.status !== 0) {
      throw new Error(
        `the AC9 guard could not resolve the wasm graph: ${(probe.stderr ?? "").trim()}`,
      );
    }
    const edges = (probe.stdout ?? "")
      .split("\n")
      .filter((line) => /(^|\s)ocelli-codec v/.test(line));
    if (edges.length === 0) {
      return;
    }
    throw new Error(
      "F-228's design plan item 5 and AC9. The wasm module decodes nothing today, so the csp probe has no worker decode case to measure. The resolved normal-edge graph of ocelli-wasm, with every feature enabled, names ocelli-codec, so the codec is linked into the module. Extend examples/viewer-react/csp-probe.mjs to decode one frame in the worker under the policy before that dependency lands:\n" +
        probe.stdout,
    );
  });
});

/**
 * The needles that would need a weaker policy to run, with their names.
 * `data:` is matched as a quoted prefix, so a `data:` key in an ambient type
 * is not a finding.
 */
const BANNED: readonly (readonly [string, string | RegExp])[] = [
  ["createObjectURL", "createObjectURL"],
  ["blob:", "blob:"],
  ["eval(", "eval("],
  ["new Function(", "new Function("],
  ["data:", /['"`]data:/],
];

function needsWeakerPolicy(text: string): string[] {
  const hits: string[] = [];
  for (const [name, needle] of BANNED) {
    const hit =
      typeof needle === "string" ? text.includes(needle) : needle.test(text);
    if (hit) {
      hits.push(name);
    }
  }
  return hits;
}

/** Every shipped source directory, derived rather than listed. */
function shippedSourceDirectories(): string[] {
  const directories: string[] = [];
  const packages = join(ROOT, "packages");
  for (const entry of readdirSync(packages, { withFileTypes: true })) {
    if (entry.isDirectory() && existsSync(join(packages, entry.name, "src"))) {
      directories.push(join("packages", entry.name, "src"));
    }
  }
  directories.push(join("examples", "viewer-react", "src"));
  return directories;
}

describe("no shipped source needs a weaker policy", () => {
  it("no shipped source constructs a blob or data worker or evaluates strings", () => {
    const files: string[] = [];
    for (const directory of shippedSourceDirectories()) {
      const found = readdirSync(join(ROOT, directory), {
        recursive: true,
        encoding: "utf8",
      });
      for (const name of found) {
        if (/\.test\.tsx?$/.test(name) || !/\.(ts|tsx)$/.test(name)) {
          continue;
        }
        files.push(join(ROOT, directory, name));
      }
    }
    // The wasm glue is generated and untracked, so it is scanned when a
    // build has left it behind, which is what a local run sees.
    const glue = join(ROOT, "crates/ocelli-wasm/pkg/ocelli_wasm.js");
    if (existsSync(glue)) {
      files.push(glue);
    }
    // The policy module is the one file that names the sources it refuses.
    const policyModule = join(ROOT, "examples/viewer-react/src/csp-policy.ts");
    const findings: string[] = [];
    for (const file of files) {
      if (file === policyModule) {
        continue;
      }
      const hits = needsWeakerPolicy(readFileSync(file, "utf8"));
      if (hits.length > 0) {
        findings.push(`${file}: ${hits.join(", ")}`);
      }
    }
    expect(
      findings,
      "F-228's amendment to AC10. No source Ocelli ships may construct a blob or data worker or evaluate strings, because each would need a weaker policy than the plan measured.",
    ).toEqual([]);
  });

  it("the scanner flags a string that needs a weaker policy", () => {
    expect(
      needsWeakerPolicy("const f = new Function('return 1')"),
    ).toContain("new Function(");
    expect(
      needsWeakerPolicy("const url = URL.createObjectURL(blob)"),
    ).toContain("createObjectURL");
    expect(
      needsWeakerPolicy('new Worker("data:text/javascript,1")'),
    ).toContain("data:");
    expect(
      needsWeakerPolicy('new Worker("blob:https://example.invalid/uuid")'),
    ).toContain("blob:");
    expect(needsWeakerPolicy("eval('return 1')")).toContain("eval(");
    // A `data:` key in an ambient type is not a data worker.
    expect(
      needsWeakerPolicy("type Entry = { data: { frames: number } }"),
    ).toEqual([]);
  });
});
