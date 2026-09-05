// Run every declared fault and require every one to be caught.
//
// The catalogue itself is `src/faults.mjs`, because some entries mutate the
// bytes the driver sends and one changes the driver's own loop, so the
// declarations are production data rather than test data. This file is only
// the runner: it spawns the driver once per fault and checks that the run went
// red at the named boundary, for the named reason.
//
// Injected runs write no reference output. They are deliberately broken and
// `out/` is where F-011 will read from.

import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { FAULTS } from "../src/faults.mjs";
import { isEntryPoint, oraclePath } from "../src/paths.mjs";

function runDriver(args, cwd) {
  return new Promise((done) => {
    const child = spawn(process.execPath, ["run.mjs", ...args], {
      cwd,
      stdio: ["ignore", "pipe", "pipe"],
    });
    // Decoded as a stream, like `capture` in run.mjs. Every `expect` fragment
    // is ASCII today, so per-chunk coercion could not corrupt one, and this is
    // the output that decides whether the guard this fault aims at fired.
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (error) => done({ code: 127, stdout, stderr: String(error) }));
    child.on("close", (code) => done({ code: code ?? 1, stdout, stderr }));
  });
}

/**
 * @param {string|null} only run just this fault
 * @returns {Promise<{ok: boolean, problems: string[], observed: object[]}>}
 */
export async function runSelfTest(only = null) {
  if (only !== null && !Object.hasOwn(FAULTS, only)) {
    // A filter that selects nothing would otherwise leave `problems` empty and
    // report "every one caught" having caught none, which is the shape this
    // whole file exists to refuse. `run.mjs` refuses both of its own filters
    // for the same reason.
    return {
      ok: false,
      problems: [
        `unknown fault ${JSON.stringify(only)}. The named faults are ` +
          `${Object.keys(FAULTS).join(", ")}.`,
      ],
      observed: [],
    };
  }
  const cwd = oraclePath();
  const scratch = await mkdtemp(join(tmpdir(), "ocelli-oracle-fault-"));
  const problems = [];
  const observed = [];
  try {
    for (const [name, fault] of Object.entries(FAULTS)) {
      if (only && only !== name) {
        continue;
      }
      process.stdout.write(`fault ${name}: ${fault.what}\n`);
      const result = await runDriver(
        ["--inject", name, "--rows", fault.row, "--out", join(scratch, name)],
        cwd,
      );
      const output = `${result.stdout}\n${result.stderr}`;
      observed.push({ name, code: result.code, boundary: fault.boundary });
      if (result.code === 0) {
        problems.push(
          `fault ${name} (${fault.boundary}) exited 0. The guard it aims at ` +
            `did not fire, so it is a guard nobody has watched fail.`,
        );
        continue;
      }
      if (!output.includes(fault.expect)) {
        problems.push(
          `fault ${name} failed, but not at the ${fault.boundary} boundary: ` +
            `nothing in the output contains ${JSON.stringify(fault.expect)}. ` +
            `A run that failed for another reason proves nothing about this ` +
            `guard.\n    output tail: ${output.trim().split("\n").slice(-4).join(" | ")}`,
        );
        continue;
      }
      process.stdout.write(`  red at ${fault.boundary}, as required\n`);
    }
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
  return { ok: problems.length === 0, problems, observed };
}

if (isEntryPoint(import.meta.url)) {
  const only = process.argv[2] ?? null;
  const result = await runSelfTest(only);
  for (const problem of result.problems) {
    process.stderr.write(`  ${problem}\n`);
  }
  process.stdout.write(
    result.ok
      ? `OK: ${result.observed.length} injected fault(s), every one caught\n`
      : `FAIL: oracle self test\n`,
  );
  process.exitCode = result.ok ? 0 : 1;
}
