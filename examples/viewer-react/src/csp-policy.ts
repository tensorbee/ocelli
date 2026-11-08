/**
 * The one Content Security Policy an embedding page needs for the Ocelli
 * worker and wasm path.
 *
 * The text exists exactly once, in `policyHeader`. The tests read the
 * documents that quote the policy and hold each marked block to its Markdown
 * text fence or HTML pre element around one header line, the
 * header line the expected header for that block. The four directives are
 * F-228's design plan item 1, sized by evidence E4, and the unit tests
 * hold the exact text.
 */

/**
 * Sources that would widen the policy the plan measured. An addition to
 * `connect-src` that names one of them is refused. `blob:` and `data:` are
 * refused because a worker constructed from a local scheme inherits the page
 * policy, measured as E2. A wildcard origin is a refusal too, so an
 * embedding page lists explicit origins.
 */
export const FORBIDDEN_SOURCES: readonly string[] = [
  "'unsafe-inline'",
  "'unsafe-eval'",
  "'unsafe-hashes'",
  "'strict-dynamic'",
  "blob:",
  "data:",
  "*",
];

/**
 * The four directives of plan item 1, in order. `worker-src 'self'` is an
 * explicit pin rather than a requirement: a worker is also allowed through
 * `worker-src`'s fallback to `script-src 'self'`, and the pin keeps a later
 * widening of `script-src` from widening worker construction with it. The
 * probe's worker-fallback case asserts that fallback rather than a failure.
 * `default-src 'none'` is in the tested header so the probe can prove Ocelli
 * needs nothing beyond the three named sources. Styles, images and fonts are
 * the embedding page's own business and outside this claim.
 */
export const EMBEDDING_POLICY: readonly (readonly [string, readonly string[]])[] =
  [
    ["default-src", ["'none'"]],
    ["script-src", ["'self'", "'wasm-unsafe-eval'"]],
    ["worker-src", ["'self'"]],
    ["connect-src", ["'self'"]],
  ];

/**
 * The header an embedding page sends on every response, the worker script
 * included.
 *
 * Each origin in `extraConnect` is appended to `connect-src`, for a DICOMweb
 * service on another origin. That is the only extension this function makes.
 * A module hosted on a CDN would need its origin in `script-src` too,
 * because the worker's `import()` of the glue is a script request, and this
 * function does not extend `script-src`. A refusal names its cause. An
 * addition is refused when it differs from its own trimmed form (leading or
 * trailing whitespace), when it names a forbidden source, when it carries
 * whitespace, a quote, `';'` or `','` (a directive or policy boundary), or
 * when it is a scheme-only source such as `https:`, which would allow any
 * origin on that scheme where the claim lists explicit origins. Only an
 * entry that has passed every check is appended.
 */
export function policyHeader(extraConnect: readonly string[] = []): string {
  const checked: string[] = [];
  for (const source of extraConnect) {
    const trimmed = source.trim();
    if (trimmed !== source) {
      throw new Error(
        `policyHeader refused ${JSON.stringify(source)}: a connect-src addition may not carry leading or trailing whitespace`,
      );
    }
    if (trimmed === "") {
      throw new Error("policyHeader refused an empty connect-src addition");
    }
    for (const forbidden of FORBIDDEN_SOURCES) {
      if (trimmed.includes(forbidden)) {
        throw new Error(`policyHeader refused the forbidden source ${source}`);
      }
    }
    if (trimmed.includes(";")) {
      throw new Error(
        `policyHeader refused ${source}: a connect-src addition may not contain ';'`,
      );
    }
    if (trimmed.includes(",")) {
      throw new Error(
        `policyHeader refused ${source}: a connect-src addition may not contain ','`,
      );
    }
    if (/\s/.test(trimmed)) {
      throw new Error(
        `policyHeader refused ${source}: a connect-src addition may not contain whitespace`,
      );
    }
    if (/\p{Cc}/u.test(trimmed)) {
      throw new Error(
        `policyHeader refused ${source}: a connect-src addition may not contain a control character`,
      );
    }
    if (trimmed.includes("'") || trimmed.includes('"')) {
      throw new Error(
        `policyHeader refused ${source}: a connect-src addition may not contain a quote`,
      );
    }
    if (/^[a-z][a-z0-9+.-]*:$/i.test(trimmed)) {
      throw new Error(
        `policyHeader refused the scheme-only source ${source}, which allows any origin on that scheme. List an explicit origin instead`,
      );
    }
    checked.push(trimmed);
  }
  return EMBEDDING_POLICY.map(([directive, sources]) =>
    directive === "connect-src"
      ? `${directive} ${[...sources, ...checked].join(" ")}`
      : `${directive} ${sources.join(" ")}`,
  ).join("; ");
}
