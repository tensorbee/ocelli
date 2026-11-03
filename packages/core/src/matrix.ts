/**
 * Browser matrix row. F-114, HLD section 7 and deviation D-07.
 *
 * The function recomputes the status. A caller-supplied status is ignored.
 * It does not launch a browser and it does not write the matrix file.
 */

export type MatrixTier = "A" | "B" | "C";

export type MatrixStatus = "certified" | "observed" | "not-run";

export interface MatrixObservation {
  renderer: string;
  resolvedTier: MatrixTier;
  exitCode: number;
}

export interface MatrixInput {
  id: string;
  expectedTier: MatrixTier;
  observation?: MatrixObservation;
  status?: string;
}

export function certifyRow(row: MatrixInput): MatrixStatus | null {
  const observation = row.observation;
  if (observation === undefined) {
    return "not-run";
  }
  if (
    row.expectedTier === "B" &&
    observation.renderer.includes("SwiftShader")
  ) {
    return null;
  }
  const finished =
    observation.exitCode === 0 &&
    observation.resolvedTier === row.expectedTier &&
    observation.renderer.length > 0;
  if (!finished) {
    return "observed";
  }
  if (
    row.expectedTier === "C" &&
    observation.renderer.includes("SwiftShader")
  ) {
    return "certified";
  }
  if (row.expectedTier === "B") {
    return "certified";
  }
  return "observed";
}
