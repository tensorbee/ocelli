// Where the harness looks for a runner, and whether it is the script node was
// asked to run.
//
// Both questions decide whether `bin/ocelli.sh bench` measures anything, and
// both were answered by three exported functions that no test executed. The
// failure they produce is the one this harness exists to refuse: an exit code
// of 0 over a run that took no figure.
//
// `run.mjs` asks `existsSync(runnerPath(id))` for every subject. A
// `runnerBasename` that returned the id unchanged makes that false for the one
// subject with a runner, so every subject reports `unavailable / no_runner`,
// `failedRunners` is empty, `main` returns 0, and the gate is green having
// measured nothing. An `isEntryPoint` that is never true does the same thing
// one level up, and its own comment says so.
//
// The convention itself is the registry's. `tools/bench/subjects.json` states
// it in the `id` field's documentation, "The runner file, when one exists, is
// src/runners/<id with dots replaced by underscores>.mjs", and
// `scripts/bench_check.py` applies the same rule from Python so the gate and
// the harness look in one place. These assertions are written from that
// sentence.

import assert from "node:assert/strict";
import test from "node:test";

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  BENCH_ROOT,
  isEntryPoint,
  REPO_ROOT,
  benchPath,
  repoPath,
  runnerBasename,
  runnerPath,
} from "../src/paths.mjs";

test("a runner basename is the subject id with every dot an underscore", () => {
  assert.equal(runnerBasename("wasm.cold_start"), "wasm_cold_start");
  // Three segments, so a rule that replaced only the first would be caught.
  assert.equal(
    runnerBasename("decode.transfer_syntax.htj2k"),
    "decode_transfer_syntax_htj2k",
  );
  // An underscore already in the id survives, and an id with no dot is itself.
  assert.equal(runnerBasename("session.cpu_idle"), "session_cpu_idle");
  assert.equal(runnerBasename("nodots"), "nodots");
  assert.equal(runnerBasename(""), "");
});

test("a runner basename never keeps a dot, which a filename would read as an extension", () => {
  for (const id of [
    "wasm.cold_start",
    "decode.frame",
    "cine.frame_change_rate",
    "tier.startup_microbenchmark",
  ]) {
    assert.equal(runnerBasename(id).includes("."), false, id);
  }
});

test("a runner path is src/runners/<basename>.mjs under tools/bench", () => {
  assert.equal(
    runnerPath("decode.frame"),
    join(BENCH_ROOT, "src", "runners", "decode_frame.mjs"),
  );
});

// The consequence, not the convention. `run.mjs` decides whether a subject has
// an instrument by asking `existsSync` this exact question, so a basename rule
// that drifted from the file on disk turns the one measurable subject in this
// tree into `unavailable / no_runner` and the run into a green nothing.
test("the runner that exists today is found at the path run.mjs asks for", () => {
  assert.equal(
    existsSync(runnerPath("wasm.cold_start")),
    true,
    "run.mjs would report the only measurable subject as having no runner",
  );
});

test("every benchmark node suite is registered in both exact runner lists", () => {
  const suites = readdirSync(join(BENCH_ROOT, "tests"))
    .filter((name) => name.endsWith("_test.mjs"))
    .sort();
  const manifest = JSON.parse(
    readFileSync(join(BENCH_ROOT, "package.json"), "utf8"),
  );
  const packageSuites = [...manifest.scripts.test.matchAll(
    /tests\/([a-z_]+_test\.mjs)/g,
  )].map((match) => match[1]).sort();
  const gate = readFileSync(join(REPO_ROOT, "bin", "ocelli.sh"), "utf8")
    .match(/^ {4}bench\)[\s\S]*?;;$/m)?.[0] ?? "";
  const gateSuites = [...gate.matchAll(
    /tools\/bench\/tests\/([a-z_]+_test\.mjs)/g,
  )].map((match) => match[1]).sort();

  assert.deepEqual(packageSuites, suites, "tools/bench/package.json test list");
  assert.deepEqual(gateSuites, suites, "bin/ocelli.sh bench gate list");
});

test("the two roots are the harness and the repository, not the caller's cwd", () => {
  assert.equal(benchPath("subjects.json"), join(BENCH_ROOT, "subjects.json"));
  assert.equal(
    repoPath("ci", "bench-baseline.json"),
    join(REPO_ROOT, "ci", "bench-baseline.json"),
  );
  assert.equal(existsSync(benchPath("subjects.json")), true);
  assert.equal(existsSync(repoPath("ci")), true);
});

// `isEntryPoint` decides whether `run.mjs`'s main block runs at all, so the
// property under test is that it is true for the file node was asked to run
// and false for every other file. `process.argv[1]` is restored in a `finally`,
// because leaving it changed would alter what a later test in this process
// observes.
function withArgv(invoked, body) {
  const saved = process.argv[1];
  process.argv[1] = invoked;
  try {
    return body();
  } finally {
    process.argv[1] = saved;
  }
}

test("the module node was asked to run is the entry point, and no other is", () => {
  const self = fileURLToPath(import.meta.url);
  withArgv(self, () => {
    assert.equal(isEntryPoint(import.meta.url), true);
  });
  withArgv(join(BENCH_ROOT, "run.mjs"), () => {
    assert.equal(
      isEntryPoint(import.meta.url),
      false,
      "a module that was not invoked reported itself as the entry point",
    );
  });
});

test("an absent or unreadable argv[1] is not an entry point rather than a throw", () => {
  withArgv(undefined, () => {
    assert.equal(isEntryPoint(import.meta.url), false);
  });
  withArgv("", () => {
    assert.equal(isEntryPoint(import.meta.url), false);
  });
  withArgv(join(tmpdir(), "ocelli-bench-no-such-file.mjs"), () => {
    assert.equal(isEntryPoint(import.meta.url), false);
  });
  withArgv(fileURLToPath(import.meta.url), () => {
    assert.equal(isEntryPoint("not a file url at all"), false);
  });
});

// The two shapes that break the obvious idiom, and the reason this function is
// not `import.meta.url === pathToFileURL(process.argv[1]).href`. A URL
// percent-encodes bytes that a path does not, and it resolves no symlinks,
// while `argv[1]` does neither. A repository reached through either would make
// the two unequal, `run.mjs`'s main block would never run, node would exit 0,
// and the gate would report success having measured nothing. Sprint worktrees
// in this project already reach the repository through a symlinked corpus.
test("a path that percent-encodes, and one reached through a symlink, still match", () => {
  const root = mkdtempSync(join(tmpdir(), "ocelli-bench-paths-"));

  const awkward = join(root, "a dir with a space and a % in it");
  mkdirSync(awkward);
  const script = join(awkward, "run.mjs");
  writeFileSync(script, "// a stand-in for run.mjs\n", "utf8");
  withArgv(script, () => {
    assert.equal(
      isEntryPoint(pathToFileURL(script).href),
      true,
      "a path needing percent-encoding was not recognised as its own module",
    );
  });

  const link = join(root, "linked.mjs");
  symlinkSync(script, link);
  withArgv(link, () => {
    assert.equal(
      isEntryPoint(pathToFileURL(script).href),
      true,
      "a module reached through a symlink was not recognised as the entry point",
    );
  });
});
