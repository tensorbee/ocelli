/**
 * Cold-start path choice. F-111, HLD section 26.
 *
 * The functions do not read the benchmark baseline and do not fetch a file.
 * Gate A4 stays open. The recorded baseline is not an input.
 */

export type StartupRole = "render" | "decode";

export interface ColdPhases {
  glue: number;
  fetch: number;
  compile: number;
  instantiate: number;
  firstCall: number;
}

const RENDER_ARTIFACT = "pkg/ocelli_wasm_bg.wasm";
const DECODE_ARTIFACT = "pkg-decode/ocelli_wasm_bg.wasm";

export function startupArtifact(role: string): string | null {
  if (role === "render") {
    return RENDER_ARTIFACT;
  }
  if (role === "decode") {
    return DECODE_ARTIFACT;
  }
  return null;
}

function finitePhase(value: number): boolean {
  return Number.isFinite(value) && value >= 0;
}

export function summariseColdStart(
  profile: string,
  phases: ColdPhases,
): number | null {
  if (profile !== "release") {
    return null;
  }
  const values = [
    phases.glue,
    phases.fetch,
    phases.compile,
    phases.instantiate,
    phases.firstCall,
  ];
  if (!values.every(finitePhase)) {
    return null;
  }
  return values.reduce((sum, value) => sum + value, 0);
}

export function summariseFirstFrame(
  profile: string,
  commandToPresentMs: number,
): number | null {
  if (profile !== "release" || !finitePhase(commandToPresentMs)) {
    return null;
  }
  return commandToPresentMs;
}
