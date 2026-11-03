/**
 * Stable-release claim. F-117.
 *
 * The full-matrix rule and the deprecation window are the approved local
 * plan. They are not an HLD formula. The functions do not publish and do
 * not change a version.
 */

export interface StableClaim {
  d22Open: boolean;
  matrixRows: number;
  certifiedRows: number;
}

export function mayClaimStable(input: StableClaim): boolean {
  if (input.d22Open !== false) {
    return false;
  }
  if (!Number.isInteger(input.matrixRows) || input.matrixRows < 1) {
    return false;
  }
  if (!Number.isInteger(input.certifiedRows)) {
    return false;
  }
  return input.certifiedRows === input.matrixRows;
}

export function removalMinorsRemaining(channel: string): number | null {
  if (channel === "minor") {
    return 1;
  }
  if (channel === "major") {
    return 0;
  }
  return null;
}
