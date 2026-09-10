// The benchmark command is a guard as well as an instrument. These cases are
// written from its documented command contract and the registry rule that a
// pending subject cannot have a runner. Every refusal in run.mjs must be
// reached directly so the census can count the file as watched.

import assert from "node:assert/strict";
import test from "node:test";

import * as command from "../run.mjs";

const ARGUMENT_REFUSALS = [
  [["--out"], /--out needs a value/],
  [["--tolerance", "zero"], /takes a fraction above 0 and at most 1/],
  [["--not-an-option"], /unknown argument --not-an-option/],
  [["--accept"], /--accept needs --story and --why/],
  [["--tolerance", "0.25"], /--tolerance needs --tolerance-why/],
  [["--tolerance-why", "measured"],
    /--tolerance-why only means something with --tolerance/],
  [["--accept", "--story", "F-000", "--why", "probe", "--no-build"],
    /--accept refuses --no-build/],
  [["--subject", "wasm.cold_start"],
    /--subject only means something with --accept/],
];

for (const [argv, expected] of ARGUMENT_REFUSALS) {
  test(`parseArgs refuses ${argv.join(" ")}`, () => {
    assert.throws(() => command.parseArgs(argv), expected);
  });
}

test("subject execution refuses a runner for a pending subject", async () => {
  assert.equal(
    typeof command.runSubjects,
    "function",
    "runSubjects must be the exported production seam, not a test wrapper",
  );
  await assert.rejects(
    command.runSubjects([
      {
        subject: {
          id: "wasm.cold_start",
          subject_story: "F-999",
        },
        subjectExists: false,
        storyStatus: "pending",
      },
    ], { out: "unused", build: false }),
    /subject story F-999 is pending rather than in-progress or done/,
  );
});
