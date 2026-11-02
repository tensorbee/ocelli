/**
 * In-memory DICOM SR content tree.
 *
 * Decision D13 says the annotation type is this tree. The HLD does not give
 * the node fields. The local rules are in `.claude/plans/F-094-design.md`.
 * The drawing layer does not own the tree. TID 1500 bytes stay with F-097.
 */

export type Concept = {
  scheme: string;
  value: string;
  meaning: string;
};

export type Relationship =
  | "CONTAINS"
  | "HAS_OBS_CONTEXT"
  | "HAS_PROPERTIES"
  | "INFERRED_FROM"
  | "SELECTED_FROM";

export type GraphicType = "POINT" | "POLYLINE" | "CIRCLE";

type ItemBase = {
  relationship: Relationship | null;
  concept: Concept;
  children: readonly ContentItem[];
};

export type ContentItem =
  | (ItemBase & { valueType: "CONTAINER" })
  | (ItemBase & { valueType: "TEXT"; text: string })
  | (ItemBase & { valueType: "NUM"; number: number; unit: string })
  | (ItemBase & { valueType: "CODE"; code: Concept })
  | (ItemBase & { valueType: "UIDREF"; uid: string })
  | (ItemBase & { valueType: "IMAGE"; sopClass: string; sopInstance: string })
  | (ItemBase & { valueType: "SCOORD"; graphic: GraphicType; points: readonly (readonly number[])[] })
  | (ItemBase & {
      valueType: "SCOORD3D";
      graphic: GraphicType;
      points: readonly (readonly number[])[];
      frameOfReference: string;
    });

const RELATIONSHIPS = new Set<string>([
  "CONTAINS",
  "HAS_OBS_CONTEXT",
  "HAS_PROPERTIES",
  "INFERRED_FROM",
  "SELECTED_FROM",
]);

const VALUE_TYPES = new Set<string>([
  "CONTAINER",
  "TEXT",
  "NUM",
  "CODE",
  "UIDREF",
  "IMAGE",
  "SCOORD",
  "SCOORD3D",
]);

function text(value: string): string | null {
  return value.length > 0 ? value : null;
}

function concept(value: Concept): Concept | null {
  const scheme = text(value.scheme);
  const code = text(value.value);
  const meaning = text(value.meaning);
  if (scheme === null || code === null || meaning === null) {
    return null;
  }
  return { scheme, value: code, meaning };
}

function graphicCount(graphic: GraphicType, count: number): boolean {
  if (graphic === "POINT") {
    return count === 1;
  }
  if (graphic === "POLYLINE") {
    return count >= 2;
  }
  return count === 2;
}

function points(
  values: readonly (readonly number[])[],
  width: number,
  graphic: GraphicType,
): readonly (readonly number[])[] | null {
  if (!graphicCount(graphic, values.length)) {
    return null;
  }
  const copied: number[][] = [];
  for (const point of values) {
    if (point.length !== width) {
      return null;
    }
    const next: number[] = [];
    for (const component of point) {
      if (!Number.isFinite(component)) {
        return null;
      }
      next.push(component);
    }
    copied.push(next);
  }
  return copied;
}

function children(items: readonly ContentItem[]): readonly ContentItem[] | null {
  const copied: ContentItem[] = [];
  for (const item of items) {
    const next = copyItem(item, false);
    if (next === null) {
      return null;
    }
    copied.push(next);
  }
  return copied;
}

function copyItem(item: ContentItem, root: boolean): ContentItem | null {
  if (!VALUE_TYPES.has(item.valueType)) {
    return null;
  }
  if (root) {
    if (item.relationship !== null) {
      return null;
    }
  } else if (item.relationship === null || !RELATIONSHIPS.has(item.relationship)) {
    return null;
  }
  const coded = concept(item.concept);
  if (coded === null) {
    return null;
  }
  const nested = children(item.children);
  if (nested === null) {
    return null;
  }
  const relationship = item.relationship;
  if (item.valueType === "CONTAINER") {
    return { relationship, valueType: "CONTAINER", concept: coded, children: nested };
  }
  if (item.valueType === "TEXT") {
    return { relationship, valueType: "TEXT", concept: coded, text: item.text, children: nested };
  }
  if (item.valueType === "NUM") {
    if (!Number.isFinite(item.number) || text(item.unit) === null) {
      return null;
    }
    return {
      relationship,
      valueType: "NUM",
      concept: coded,
      number: item.number,
      unit: item.unit,
      children: nested,
    };
  }
  if (item.valueType === "CODE") {
    const code = concept(item.code);
    if (code === null) {
      return null;
    }
    return { relationship, valueType: "CODE", concept: coded, code, children: nested };
  }
  if (item.valueType === "UIDREF") {
    const uid = text(item.uid);
    if (uid === null) {
      return null;
    }
    return { relationship, valueType: "UIDREF", concept: coded, uid, children: nested };
  }
  if (item.valueType === "IMAGE") {
    const sopClass = text(item.sopClass);
    const sopInstance = text(item.sopInstance);
    if (sopClass === null || sopInstance === null) {
      return null;
    }
    return {
      relationship,
      valueType: "IMAGE",
      concept: coded,
      sopClass,
      sopInstance,
      children: nested,
    };
  }
  if (item.valueType === "SCOORD") {
    const copied = points(item.points, 2, item.graphic);
    if (copied === null) {
      return null;
    }
    return {
      relationship,
      valueType: "SCOORD",
      concept: coded,
      graphic: item.graphic,
      points: copied,
      children: nested,
    };
  }
  if (item.valueType !== "SCOORD3D") {
    return null;
  }
  const copied = points(item.points, 3, item.graphic);
  const frame = text(item.frameOfReference);
  if (copied === null || frame === null) {
    return null;
  }
  return {
    relationship,
    valueType: "SCOORD3D",
    concept: coded,
    graphic: item.graphic,
    points: copied,
    frameOfReference: frame,
    children: nested,
  };
}

/** Copy one valid content tree. One bad node returns nothing. */
export function contentTree(root: ContentItem): ContentItem | null {
  return copyItem(root, true);
}

const PROVENANCE = {
  tool: { scheme: "OCELLI", value: "tool", meaning: "Tool" },
  engine: { scheme: "OCELLI", value: "engine", meaning: "Engine version" },
  algorithm: { scheme: "OCELLI", value: "algorithm", meaning: "Algorithm" },
  operator: { scheme: "OCELLI", value: "operator", meaning: "Operator" },
} as const;

function provenanceText(role: keyof typeof PROVENANCE, value: string): ContentItem {
  const source = PROVENANCE[role];
  return {
    relationship: "HAS_OBS_CONTEXT",
    valueType: "TEXT",
    concept: { scheme: source.scheme, value: source.value, meaning: source.meaning },
    text: value,
    children: [],
  };
}

/**
 * Provenance children in tool, engine, algorithm, operator order.
 * A null algorithm omits that child. The concept codes are local labels.
 */
export function provenanceItems(
  tool: string,
  engine: string,
  algorithm: string | null,
  operator: string,
): readonly ContentItem[] | null {
  if (text(tool) === null || text(engine) === null || text(operator) === null) {
    return null;
  }
  const items = [
    provenanceText("tool", tool),
    provenanceText("engine", engine),
  ];
  if (algorithm !== null) {
    if (text(algorithm) === null) {
      return null;
    }
    items.push(provenanceText("algorithm", algorithm));
  }
  items.push(provenanceText("operator", operator));
  return items;
}
