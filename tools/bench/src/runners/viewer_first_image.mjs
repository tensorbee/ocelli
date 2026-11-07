import { measureFirstImage } from "../viewer_work.mjs";

export const id = "viewer.first_image";

export async function run(options = {}) {
  return measureFirstImage(options);
}
