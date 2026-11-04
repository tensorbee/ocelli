import { useEffect, useRef, useState } from "react";

import {
  PAGE_LAYOUT,
  fullscreenWord,
  layoutReport,
  type LayoutReport,
} from "./layout-report.js";

function word(value: number | string | null): string {
  return value === null ? "refused" : String(value);
}

/** Layout grid, stack drag, full screen, reference lines and reference cursor. */
export function LayoutPanel() {
  const sectionRef = useRef<HTMLElement>(null);
  const [report, setReport] = useState<LayoutReport | null>(null);
  const [screenWord, setScreenWord] = useState(() =>
    fullscreenWord(document.fullscreenEnabled === true, false),
  );

  useEffect(() => {
    function onChange(): void {
      const section = sectionRef.current;
      const active = section !== null && document.fullscreenElement === section;
      setScreenWord(fullscreenWord(document.fullscreenEnabled === true, active));
    }
    document.addEventListener("fullscreenchange", onChange);
    return () => {
      document.removeEventListener("fullscreenchange", onChange);
    };
  }, []);

  function toggleFullscreen(): void {
    const section = sectionRef.current;
    if (section === null || document.fullscreenEnabled !== true) {
      setScreenWord("unavailable");
      return;
    }
    if (typeof section.requestFullscreen !== "function") {
      setScreenWord("unavailable");
      return;
    }
    if (document.fullscreenElement === section) {
      void document.exitFullscreen();
      return;
    }
    void section.requestFullscreen().catch(() => {
      setScreenWord("unavailable");
    });
  }

  return (
    <section ref={sectionRef}>
      <h2>Layout</h2>
      <p>The series panel above is unchanged.</p>
      <p>Full screen {screenWord}</p>
      <p>Prior studies unavailable</p>
      <p>Linked scroll unavailable</p>
      <p>Synced window unavailable</p>
      <p>MPR unavailable</p>
      <p>Slab unavailable</p>
      <p>Volume rendering unavailable</p>
      <p>Fusion unavailable</p>
      <p>
        <button type="button" onClick={() => { setReport(layoutReport(PAGE_LAYOUT)); }}>
          Show layout
        </button>
        <button type="button" onClick={toggleFullscreen}>
          Full screen
        </button>
      </p>
      {report !== null && (
        <div>
          <p>Grid lines {word(report.gridLines)}</p>
          <p>Stack drag {word(report.dragStep)}</p>
          <p>Horizontal reference {word(report.horizontal)}</p>
          <p>Vertical reference {word(report.vertical)}</p>
          <p>Reference cursor {word(report.cursor)}</p>
        </div>
      )}
    </section>
  );
}
