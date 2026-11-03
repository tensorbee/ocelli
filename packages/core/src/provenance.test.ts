import { describe, expect, it } from "vitest";

import { classifyProvenance } from "./provenance.js";

const RECORDED = [
  "F-118, classify a message",
  "",
  "The classifier reads one message.",
  "",
  "Ocelli-Verify: profile=feature gates=fmt corpus=pass tree=abc",
  "Ocelli-Generated-By: worker",
].join("\n");

describe("classifyProvenance", () => {
  it("returns recorded when both trailers carry the four fields", () => {
    expect(classifyProvenance(RECORDED)).toBe("recorded");
    expect(
      classifyProvenance(
        RECORDED.replace(
          "Ocelli-Verify: profile=feature",
          "Ocelli-Verify:profile=feature",
        ),
      ),
    ).toBe("recorded");
    expect(
      classifyProvenance(
        RECORDED.replace(
          "tree=abc",
          "tree=abc comparison=present",
        ),
      ),
    ).toBe("recorded");
  });

  it("returns missing when neither trailer is present", () => {
    expect(
      classifyProvenance("F-118, classify a message\n\nThe sprint is open.\n"),
    ).toBe("missing");
    expect(
      classifyProvenance(
        "subject\n\nCo-Authored-By: Ada Lovelace <ada@example.com>\n",
      ),
    ).toBe("missing");
  });

  it("returns forged when only one trailer is present", () => {
    expect(
      classifyProvenance(
        "subject\n\nOcelli-Verify: profile=feature gates=fmt corpus=pass tree=abc\n",
      ),
    ).toBe("forged");
    expect(
      classifyProvenance("subject\n\nOcelli-Generated-By: worker\n"),
    ).toBe("forged");
  });

  it("returns forged when a longer token only looks like a field", () => {
    expect(
      classifyProvenance(
        RECORDED.replace("profile=feature", "notprofile=feature"),
      ),
    ).toBe("forged");
  });

  it("returns forged when the verify line lacks tree", () => {
    expect(
      classifyProvenance(
        [
          "subject",
          "",
          "Ocelli-Verify: profile=feature gates=fmt corpus=pass",
          "Ocelli-Generated-By: worker",
        ].join("\n"),
      ),
    ).toBe("forged");
  });

  it("returns forged when a trailer line is duplicated", () => {
    expect(
      classifyProvenance(
        [
          "subject",
          "",
          "Ocelli-Verify: profile=feature gates=fmt corpus=pass tree=abc",
          "Ocelli-Verify: profile=feature gates=fmt corpus=pass tree=abc",
          "Ocelli-Generated-By: worker",
        ].join("\n"),
      ),
    ).toBe("forged");
    expect(
      classifyProvenance(
        [
          "subject",
          "",
          "Ocelli-Verify: profile=feature gates=fmt corpus=pass tree=abc",
          "Ocelli-Generated-By: worker",
          "Ocelli-Generated-By: worker",
        ].join("\n"),
      ),
    ).toBe("forged");
  });

  it("returns forged when an agent co-author line is present", () => {
    expect(
      classifyProvenance(`${RECORDED}\nCo-Authored-By: Claude\n`),
    ).toBe("forged");
    expect(classifyProvenance("subject\n\nCo-Authored-By: codex\n")).toBe(
      "forged",
    );
    expect(classifyProvenance("subject\n\nCo-Authored-By: GPT\n")).toBe(
      "forged",
    );
    expect(classifyProvenance("subject\n\nco-authored-by: Copilot\n")).toBe(
      "forged",
    );
  });
});
