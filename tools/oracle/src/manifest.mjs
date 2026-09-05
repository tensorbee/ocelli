// The corpus manifest, read the way `corpus/README.md` documents it.
//
// This is boundary one of the four the sprint names. "A headless page can
// start, load a test runner and exit successfully without decoding every
// corpus row." The run can only assert it attempted every row if the reader
// produces every row, so the reader refuses a malformed manifest rather than
// skipping the line it could not read. Every refusal below names a row that
// would otherwise disappear from a total the run then reports as complete.

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

import { repoPath } from "./paths.mjs";

/** corpus/README.md, "Columns:". The order is part of the format. */
export const MANIFEST_COLUMNS = [
  "path",
  "modality",
  "transfer_syntax",
  "category",
  "source",
  "licence",
  "licence_url",
  "sha256",
  "url",
];

export const MANIFEST_PATH = repoPath("corpus", "manifest.tsv");
export const CORPUS_DATA = repoPath("corpus", "data");

const SHA256 = /^[0-9a-f]{64}$/;

/**
 * Parse the manifest text into rows.
 *
 * @param {string} text the whole file
 * @returns {Array<{path: string, modality: string, transferSyntax: string,
 *   categories: string[], category: string, source: string, licence: string,
 *   licenceUrl: string, sha256: string, url: string, line: number}>}
 */
export function parseManifest(text) {
  const lines = text.split("\n");
  if (lines.length === 0 || lines[0].trim() === "") {
    throw new Error("corpus/manifest.tsv is empty, so it has no header");
  }
  const header = lines[0].split("\t");
  if (
    header.length !== MANIFEST_COLUMNS.length ||
    header.some((name, index) => name !== MANIFEST_COLUMNS[index])
  ) {
    throw new Error(
      `corpus/manifest.tsv header is ${JSON.stringify(header)}, expected ` +
        `${JSON.stringify(MANIFEST_COLUMNS)}. corpus/README.md documents the ` +
        `column list and it is part of the format.`,
    );
  }

  const rows = [];
  const seen = new Map();
  for (let index = 1; index < lines.length; index += 1) {
    const raw = lines[index];
    if (raw === "") {
      continue;
    }
    const lineNumber = index + 1;
    const cells = raw.split("\t");
    if (cells.length !== MANIFEST_COLUMNS.length) {
      throw new Error(
        `corpus/manifest.tsv line ${lineNumber} has ${cells.length} cells, ` +
          `expected ${MANIFEST_COLUMNS.length} columns. A short row is a row ` +
          `the oracle would not attempt.`,
      );
    }
    const [
      path,
      modality,
      transferSyntax,
      category,
      source,
      licence,
      licenceUrl,
      sha256,
      url,
    ] = cells;

    if (path === "") {
      throw new Error(`corpus/manifest.tsv line ${lineNumber} has no path`);
    }
    if (!SHA256.test(sha256)) {
      throw new Error(
        `corpus/manifest.tsv line ${lineNumber} (${path}) has sha256 ` +
          `${JSON.stringify(sha256)}, which is not 64 hex characters. The ` +
          `digest is what ties reference output to one corpus.`,
      );
    }
    // Keyed on the OUTPUT NAME, not on the path, because the output name is
    // what would collide. `a/b.dcm` and `a__b.dcm` are two paths and one id.
    let id;
    try {
      id = rowId(path);
    } catch (error) {
      // Every other refusal here names its line, and this one is reached from
      // the same loop, so it names one too.
      throw new Error(
        `corpus/manifest.tsv line ${lineNumber}: ` +
          `${String(error?.message ?? error)}`,
      );
    }
    if (seen.has(id)) {
      const [firstPath, firstLine] = seen.get(id);
      throw new Error(
        `corpus/manifest.tsv line ${lineNumber} (${path}) reduces to the ` +
          `output name ${id}, which line ${firstLine} (${firstPath}) already ` +
          `claims. Two rows would write one set of files and the second would ` +
          `silently overwrite the first.`,
      );
    }
    seen.set(id, [path, lineNumber]);

    rows.push({
      path,
      modality,
      transferSyntax,
      category,
      categories: category
        .split(",")
        .map((token) => token.trim())
        .filter((token) => token !== ""),
      source,
      licence,
      licenceUrl,
      sha256,
      url,
      line: lineNumber,
    });
  }
  return rows;
}

/** Read and parse the committed manifest. */
export async function readManifest() {
  return parseManifest(await readFile(MANIFEST_PATH, "utf8"));
}

/**
 * The output name for a row.
 *
 * Flat, unique and safe as a file name, because every row writes three files
 * into one directory. `/` becomes `__` rather than a nested directory so that
 * `<id>.png`, `<id>.raw` and `<id>.json` always sit beside each other.
 */
export function rowId(path) {
  // A manifest path is joined onto `corpus/data` to read the bytes, so a
  // segment that walks upward would read a file outside the corpus. The
  // manifest is tracked and every row is digest-checked, so this is not a
  // reachable attack, but `src/server.mjs` refuses exactly this for its own
  // directory and gives the reason, and one path derivation in this harness
  // without the check is one place a later reader has to reason about.
  if (path.split("/").some((segment) => segment === ".." || segment === ".")) {
    throw new Error(
      `corpus path ${path} contains a relative segment, so joining it onto ` +
        `corpus/data would not stay inside the corpus`,
    );
  }
  if (path.startsWith("/")) {
    throw new Error(`corpus path ${path} is absolute, and every row is relative`);
  }
  const withoutSuffix = path.replace(/\.dcm$/i, "");
  const id = withoutSuffix.replace(/\//g, "__");
  if (!/^[A-Za-z0-9_.-]+$/.test(id)) {
    throw new Error(
      `corpus path ${path} does not reduce to a safe output name (${id})`,
    );
  }
  return id;
}

/** The sha256 of a buffer, as lower-case hex. */
export function digestOf(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

/** The sha256 of the manifest file itself, recorded in run.json. */
export async function digestOfManifest() {
  return digestOf(await readFile(MANIFEST_PATH));
}
