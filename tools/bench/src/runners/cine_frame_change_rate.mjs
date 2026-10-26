import { runBrowserSession } from "../browser_session.mjs";

export const id = "cine.frame_change_rate";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
