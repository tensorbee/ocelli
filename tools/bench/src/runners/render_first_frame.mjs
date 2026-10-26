import { runBrowserSession } from "../browser_session.mjs";

export const id = "render.first_frame";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
