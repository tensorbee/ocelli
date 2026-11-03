/**
 * Shadow divergence count. F-109, HLD section 12.
 *
 * The caller renders both and passes two byte buffers.
 * This function counts samples and does not render.
 * It does not import another viewer.
 */

export function shadowDivergence(
  ocelli: Uint8Array,
  other: Uint8Array,
  limit: number,
): number | null {
  if (ocelli.length !== other.length) {
    return null;
  }
  if (!Number.isInteger(limit) || limit < 0 || limit > 255) {
    return null;
  }
  let count = 0;
  for (let index = 0; index < ocelli.length; index += 1) {
    const left = ocelli[index];
    const right = other[index];
    if (left === undefined || right === undefined) {
      return null;
    }
    if (Math.abs(left - right) > limit) {
      count += 1;
    }
  }
  return count;
}
