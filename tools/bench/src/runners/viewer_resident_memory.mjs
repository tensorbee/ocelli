import { measureResident } from "../viewer_work.mjs";

export const id = "viewer.resident_memory";

export async function run(options = {}) {
  return measureResident(options);
}
