import {
  annotationsForFrame,
  createAnnotationState,
  createPointerDispatch,
  createToolRegistry,
  keyAction,
  keyImageMark,
  loadMeasurements,
  persistMeasurements,
  putAnnotation,
  readTid1500,
  undoAnnotation,
  viewportId,
  writeTid1500,
} from "@ocelli/core";

/** One synthetic measurement and the shortcut the button should name. */
export interface MeasurementInput {
  readonly millimetres: number;
  readonly tidMillimetres: number;
  readonly key: string;
  readonly shift: boolean;
  readonly sopInstance: string;
}

/** The document, the list, the mark, the shortcut and the touch name. */
export interface MeasurementReport {
  readonly stored: number | null;
  readonly annotations: number | null;
  readonly undo: number | null;
  readonly tid: number | null;
  readonly keyImage: string | null;
  readonly shortcut: string | null;
  readonly touch: string | null;
}

const CONCEPT = {
  scheme: "SCT",
  value: "410668003",
  meaning: "Length",
} as const;

const SOURCE = {
  study: "2.25.4401",
  series: "2.25.4402",
  sopClass: "1.2.840.10008.5.1.4.1.1.2",
  sopInstance: "2.25.4403",
} as const;

/** The fixed inputs the example button uses. */
export const PAGE_MEASUREMENT: MeasurementInput = {
  millimetres: 12.5,
  tidMillimetres: 12.5,
  key: "ArrowUp",
  shift: true,
  sopInstance: SOURCE.sopInstance,
};

const FRAME = "2.25.4409";

function lengthItem(millimetres: number) {
  return {
    relationship: null,
    valueType: "NUM" as const,
    concept: CONCEPT,
    number: millimetres,
    unit: "mm",
    children: [],
  };
}

function firstNumber(items: readonly { readonly valueType: string; readonly number?: number }[] | null): number | null {
  if (items === null) {
    return null;
  }
  for (const item of items) {
    if (item.valueType === "NUM" && typeof item.number === "number") {
      return item.number;
    }
  }
  return null;
}

/** The first event name for one pointer down. A mouse down is not a touch start. */
export function pointerStartName(pointerType: "touch" | "mouse"): string | null {
  const dispatch = createPointerDispatch({
    registry: createToolRegistry(),
    groupId: "s44",
    viewport: viewportId(1),
  });
  dispatch.ingest({
    type: "pointerdown",
    pointerType,
    button: 0,
    clientX: 4,
    clientY: 6,
    pointerId: 3,
  });
  const frame = dispatch.takeFrame();
  for (const event of frame.events) {
    return event.name;
  }
  return null;
}

/** One touch pointer down. */
export function touchStartName(): string | null {
  return pointerStartName("touch");
}

/** Persist, annotate, undo, write TID 1500, mark, shortcut and touch. */
export function measurementReport(input: MeasurementInput): MeasurementReport {
  const item = lengthItem(input.millimetres);
  const tidItem = lengthItem(input.tidMillimetres);
  const document = persistMeasurements([item]);
  const loaded = document === null ? null : loadMeasurements(document);
  const put = putAnnotation(createAnnotationState(), "m1", FRAME, item);
  const listed = put === null ? null : annotationsForFrame(put, FRAME);
  const undone = put === null ? null : undoAnnotation(put);
  const bytes = writeTid1500([tidItem], {
    study: SOURCE.study,
    series: SOURCE.series,
    sopClass: SOURCE.sopClass,
    sopInstance: input.sopInstance,
  });
  const read = bytes === null ? null : readTid1500(bytes);
  const mark = keyImageMark("k1", { x: 1, y: 2 });
  return {
    stored: firstNumber(loaded),
    annotations: listed === null ? null : listed.length,
    undo: undone === null ? null : undone.records.length,
    tid: firstNumber(read),
    keyImage: mark.ok ? mark.mark.text : null,
    shortcut: keyAction(input.key, input.shift),
    touch: touchStartName(),
  };
}
