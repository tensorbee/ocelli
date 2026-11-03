import { describe, expect, it } from "vitest";

import { rewriteListenerNames } from "./migrate.js";

describe("F-108 listener renames", () => {
  it("rewrites a mapped listener literal", () => {
    const source = 'element.addEventListener("ELEMENT_ENABLED", listener);';
    const rewritten = rewriteListenerNames(source);
    expect(rewritten).toBe('element.addEventListener("viewport.mounted", listener);');
    expect(rewritten).not.toContain("ELEMENT_ENABLED");
  });

  it("leaves an unavailable listener name", () => {
    const source = 'element.addEventListener("CAMERA_MODIFIED", listener);';
    expect(rewriteListenerNames(source)).toBe(source);
  });

  it("leaves enable and rewrites a removed listener", () => {
    const source = 'cornerstone.enable(element);\n'
      + 'element.removeEventListener("ELEMENT_DISABLED", listener);';
    const rewritten = rewriteListenerNames(source);
    expect(rewritten).toContain("cornerstone.enable(element)");
    expect(rewritten).toContain('removeEventListener("viewport.unmounted", listener)');
    expect(rewritten).not.toContain("ELEMENT_DISABLED");
  });

  it("keeps whitespace around a rewritten literal", () => {
    const spaced = 'addEventListener ( "ELEMENT_ENABLED" , listener);';
    expect(rewriteListenerNames(spaced)).toBe(
      'addEventListener ( "viewport.mounted" , listener);',
    );
    const broken = "removeEventListener(\n  'ELEMENT_DISABLED',\n  listener);";
    expect(rewriteListenerNames(broken)).toBe(
      "removeEventListener(\n  'viewport.unmounted',\n  listener);",
    );
    const stayed = 'addEventListener ( "CAMERA_MODIFIED" , listener);';
    expect(rewriteListenerNames(stayed)).toBe(stayed);
  });

  it("refuses a non-string and leaves a template literal", () => {
    expect(rewriteListenerNames(1 as unknown as string)).toBeNull();
    const source = "element.addEventListener(`ELEMENT_ENABLED`, listener);";
    expect(rewriteListenerNames(source)).toBe(source);
  });
});
