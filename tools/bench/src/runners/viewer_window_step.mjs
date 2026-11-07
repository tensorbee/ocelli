import { measureWindow } from "../viewer_work.mjs";

export const id = "viewer.window_step";

export async function run(options = {}) {
  return measureWindow(options);
}
