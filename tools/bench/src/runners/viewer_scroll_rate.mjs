import { measureScroll } from "../viewer_work.mjs";

export const id = "viewer.scroll_rate";

export async function run(options = {}) {
  return measureScroll(options);
}
