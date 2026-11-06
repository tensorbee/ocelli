import { measureOpen } from "../viewer_work.mjs";

export const id = "viewer.open";

export async function run(options = {}) {
  return measureOpen(options);
}
