import { useState } from "react";

import {
  PAGE_MEASUREMENT,
  measurementReport,
  type MeasurementReport,
} from "./measurement-report.js";

function word(value: number | string | null): string {
  return value === null ? "refused" : String(value);
}

/** Measurement document, annotation list, undo, TID 1500, key image, shortcut and touch. */
export function MeasurementPanel() {
  const [keyName, setKeyName] = useState(PAGE_MEASUREMENT.key);
  const [shift, setShift] = useState(PAGE_MEASUREMENT.shift);
  const [report, setReport] = useState<MeasurementReport | null>(null);

  return (
    <section>
      <h2>Measurement</h2>
      <p>Metadata browser unavailable</p>
      <p>Preferences unavailable</p>
      <p>Secondary capture export stays on the document panel.</p>
      <p>
        <label>
          Shortcut key
          <input
            aria-label="Shortcut key"
            value={keyName}
            onChange={(event) => {
              setKeyName(event.target.value);
            }}
          />
        </label>
        <label>
          <input
            type="checkbox"
            aria-label="Shortcut shift"
            checked={shift}
            onChange={(event) => {
              setShift(event.target.checked);
            }}
          />
          Shift
        </label>
      </p>
      <p>
        <button
          type="button"
          onClick={() => {
            setReport(measurementReport({
              millimetres: PAGE_MEASUREMENT.millimetres,
              tidMillimetres: PAGE_MEASUREMENT.tidMillimetres,
              key: keyName,
              shift,
              sopInstance: PAGE_MEASUREMENT.sopInstance,
            }));
          }}
        >
          Show measurement
        </button>
      </p>
      {report !== null && (
        <div>
          <p>Measurement {word(report.stored)}</p>
          <p>Annotations {word(report.annotations)}</p>
          <p>Undo {word(report.undo)}</p>
          <p>TID 1500 {word(report.tid)}</p>
          <p>Key image {word(report.keyImage)}</p>
          <p>Shortcut {word(report.shortcut)}</p>
          <p>Touch {word(report.touch)}</p>
        </div>
      )}
    </section>
  );
}
