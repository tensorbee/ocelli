/**
 * Provenance classification. F-118.
 *
 * The three words are the approved local plan. They are not an HLD
 * formula. The function does not write a trailer and does not call the
 * commit-msg hook.
 */

const VERIFY = "Ocelli-Verify:";
const GENERATED = "Ocelli-Generated-By:";
const AGENT =
  /^Co-Authored-By:.*(?:Claude|Codex|GPT|Copilot)/i;
const FIELDS = ["profile", "gates", "corpus", "tree"] as const;

function hasField(line: string, name: string): boolean {
  const needle = `${name}=`;
  let from = 0;
  while (from < line.length) {
    const at = line.indexOf(needle, from);
    if (at < 0) {
      return false;
    }
    const before = at === 0 ? "" : line.charAt(at - 1);
    if (before === "" || !/[A-Za-z0-9_]/.test(before)) {
      return true;
    }
    from = at + needle.length;
  }
  return false;
}

export function classifyProvenance(
  message: string,
): "recorded" | "missing" | "forged" {
  const lines = message.split("\n").map((line) => line.replace(/\r$/, ""));
  const verify = lines.filter((line) => line.startsWith(VERIFY));
  const generated = lines.filter((line) => line.startsWith(GENERATED));
  if (lines.some((line) => AGENT.test(line))) {
    return "forged";
  }
  if (verify.length === 0 && generated.length === 0) {
    return "missing";
  }
  if (verify.length !== 1 || generated.length !== 1) {
    return "forged";
  }
  const line = verify[0];
  if (line === undefined) {
    return "forged";
  }
  for (const name of FIELDS) {
    if (!hasField(line, name)) {
      return "forged";
    }
  }
  return "recorded";
}
