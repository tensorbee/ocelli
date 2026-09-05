// Unit tests for the manifest reader. No browser, no corpus bytes.
//
// The reader is the first of the four boundaries: a row the reader drops is a
// row the run never attempts, and the run would then report a smaller total
// and call itself complete. So the expectations below are written against
// `corpus/README.md`'s stated column list and category grammar, and against
// the committed manifest's own shape, not against what the reader happens to
// return.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  MANIFEST_COLUMNS,
  parseManifest,
  rowId,
  digestOfManifest,
} from "../src/manifest.mjs";
import { repoPath } from "../src/paths.mjs";

const MANIFEST_TEXT = readFileSync(repoPath("corpus/manifest.tsv"), "utf8");

// corpus/README.md: "Columns: path, modality, transfer_syntax, category,
// source, licence, licence_url, sha256, url".
test("the column list is the one corpus/README.md documents", () => {
  assert.deepEqual(MANIFEST_COLUMNS, [
    "path",
    "modality",
    "transfer_syntax",
    "category",
    "source",
    "licence",
    "licence_url",
    "sha256",
    "url",
  ]);
});

test("a header naming different columns is refused, not adapted to", () => {
  const text = "path\tmodality\n" + "a.dcm\tCT\n";
  assert.throws(() => parseManifest(text), /header/i);
});

// The empty file. `"".split("\n")` is `[""]`, so `lines.length` is 1 and only
// the blank-first-line half of the condition catches it. Reached by nothing
// until this test: replacing the refusal with `if (false)` left every suite
// green, and the run would have gone on to read a header that is one empty
// string.
test("an empty manifest is refused for having no header", () => {
  assert.throws(() => parseManifest(""), /has no header/);
  assert.throws(() => parseManifest("   \n"), /has no header/);
});

// A row whose first cell is empty still has nine cells, so the column count
// and the digest both pass and the row would parse into a path of "". Every
// output name and every corpus lookup derives from that path.
test("a row with an empty path is refused, naming the line", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const digest = "0".repeat(64);
  const line = `\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\t${digest}\t`;
  assert.throws(() => parseManifest(`${header}\n${line}\n`), /line 2/);
  assert.throws(() => parseManifest(`${header}\n${line}\n`), /has no path/);
});

test("every committed row parses, and none is dropped", () => {
  const dataLines = MANIFEST_TEXT.trimEnd().split("\n").length - 1;
  const rows = parseManifest(MANIFEST_TEXT);
  assert.equal(rows.length, dataLines);
  assert.ok(rows.length > 0);
});

test("a row is parsed into the fields the driver uses", () => {
  const rows = parseManifest(MANIFEST_TEXT);
  const row = rows.find((r) => r.path === "synthetic/ct_unsigned_16.dcm");
  assert.ok(row, "synthetic/ct_unsigned_16.dcm is in the committed manifest");
  assert.equal(row.modality, "CT");
  assert.equal(row.transferSyntax, "1.2.840.10008.1.2.1");
  // corpus/README.md: the category column is a comma separated token list.
  assert.deepEqual(row.categories, ["synthetic", "mono16", "unsigned-16"]);
  assert.match(row.sha256, /^[0-9a-f]{64}$/);
});

test("a row missing a field is refused rather than filled in", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const short = `${header}\na.dcm\tCT\t1.2.840.10008.1.2.1\n`;
  assert.throws(() => parseManifest(short), /9 columns/);
});

// Two ways to not be a sha256, and both have to be refused. A wrong alphabet
// is the obvious one. A HEX STRING OF THE WRONG LENGTH is the one that gets
// through a loose check: a sha1 is forty valid hex characters, and a truncated
// sha256 is any prefix of one. The digest is what ties reference output to one
// corpus, so a shorter digest is a weaker tie, not a different format.
test("a digest with the wrong alphabet is refused", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const bad = `${header}\na.dcm\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\tnotadigest\t\n`;
  assert.throws(() => parseManifest(bad), /sha256/);
});

test("a hex digest of the wrong length is refused", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const line = (digest) =>
    `${header}\na.dcm\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\t${digest}\t\n`;
  // A sha1, forty valid hex characters.
  assert.throws(() => parseManifest(line("a".repeat(40))), /sha256/);
  // A truncated sha256.
  assert.throws(() => parseManifest(line("abc")), /sha256/);
  // One character too many.
  assert.throws(() => parseManifest(line("a".repeat(65))), /sha256/);
  // And exactly sixty-four is accepted, so the test above is about the length
  // and not about the row being malformed some other way.
  assert.equal(parseManifest(line("a".repeat(64))).length, 1);
});

test("a duplicate path is refused, because one output name would win", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const digest = "0".repeat(64);
  const line = `a.dcm\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\t${digest}\t`;
  assert.throws(
    () => parseManifest(`${header}\n${line}\n${line}\n`),
    /already\s+claims/,
  );
});

// Two DIFFERENT paths that reduce to one output name. `rowId` replaces `/`
// with `__`, so `a/b.dcm` and `a__b.dcm` are two rows and one set of output
// files, and the second would silently overwrite the first. No committed row
// contains `__` today, which is exactly why this needs its own test.
test("two paths that reduce to one output name are refused", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const digest = "0".repeat(64);
  const row = (path) =>
    `${path}\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\t${digest}\t`;
  assert.throws(
    () => parseManifest(`${header}\n${row("a/b.dcm")}\n${row("a__b.dcm")}\n`),
    /reduces to the output name a__b/,
  );
});

// A manifest path is joined onto `corpus/data` to read the bytes, so a segment
// that walks upward would read a file outside the corpus. Not reachable, since
// the manifest is tracked and every row is digest-checked, but `src/server.mjs`
// refuses exactly this for its own directory and has four tests for it, and one
// path derivation in this harness without the check is one a later reader has
// to reason about.
//
// The safe-name regex below would NOT catch these on its own: `.` is inside its
// character class, so `..____..____etc` is a legal file name.
test("a path with a relative segment is refused", () => {
  for (const path of [
    "../etc/passwd.dcm",
    "a/../../etc.dcm",
    "./a.dcm",
    "a/./b.dcm",
    "..",
  ]) {
    assert.throws(
      () => rowId(path),
      /relative segment/,
      `${path} was accepted`,
    );
  }
});

test("an absolute path is refused, because every row is relative", () => {
  assert.throws(() => rowId("/etc/passwd.dcm"), /is absolute/);
});

// Every row writes `<id>.png`, `<id>.raw` and `<id>.json` into one flat
// directory, so the id has to be a legal file name and not merely a unique
// string. The refusal was watched by nothing: with `if (false)` in its place a
// path carrying a space or a shell metacharacter reduced to an id that would
// have been used as a filename anyway.
test("a path that reduces to an unsafe output name is refused", () => {
  for (const path of ["a b.dcm", "a/b c.dcm", "a$(x).dcm", "a;rm.dcm", "a\u00e9.dcm"]) {
    assert.throws(
      () => rowId(path),
      /does not reduce to a safe output name/,
      `${path} was accepted`,
    );
  }
});

// And the adjacent shapes that are legitimate are still accepted, so the
// refusal is about segments and not about the character.
test("a dot inside a segment is not a relative segment", () => {
  assert.equal(rowId("a/.hidden.dcm"), "a__.hidden");
  assert.equal(rowId("a/..b.dcm"), "a__..b");
  assert.equal(rowId("a/b.c.dcm"), "a__b.c");
});

// Every other refusal in `parseManifest` names its line, and this one is
// reached from the same loop.
test("a relative segment in the manifest is refused, naming the line", () => {
  const header = MANIFEST_COLUMNS.join("\t");
  const digest = "0".repeat(64);
  const line = `../x.dcm\tCT\t1.2.840.10008.1.2.1\tsynthetic, mono16\ts\tl\tu\t${digest}\t`;
  assert.throws(() => parseManifest(`${header}\n${line}\n`), /line 2/);
  assert.throws(() => parseManifest(`${header}\n${line}\n`), /relative segment/);
});

// The output name has to be unique per row and legal as a file name, because
// every row writes `<id>.png`, `<id>.raw` and `<id>.json` into one flat
// directory. Two rows collapsing onto one id would silently overwrite.
test("rowId is a flat, unique, filesystem-safe name", () => {
  assert.equal(rowId("synthetic/ct_unsigned_16.dcm"), "synthetic__ct_unsigned_16");
  assert.equal(
    rowId("synthetic/ct_series_uniform/slice_003.dcm"),
    "synthetic__ct_series_uniform__slice_003",
  );
  assert.equal(rowId("real/us_cmb_crc/00000001.dcm"), "real__us_cmb_crc__00000001");
});

test("rowId is unique across every committed row", () => {
  const rows = parseManifest(MANIFEST_TEXT);
  const ids = new Set(rows.map((r) => rowId(r.path)));
  assert.equal(ids.size, rows.length);
  for (const id of ids) {
    assert.match(id, /^[A-Za-z0-9_.-]+$/, `${id} is not a safe file name`);
  }
});

// run.json ties its output to the manifest. The digest is of the bytes on
// disk, so a row edited between two runs changes it.
test("the manifest digest is the sha256 of the file bytes", async () => {
  const { createHash } = await import("node:crypto");
  const expected = createHash("sha256")
    .update(readFileSync(repoPath("corpus/manifest.tsv")))
    .digest("hex");
  assert.equal(await digestOfManifest(), expected);
});
