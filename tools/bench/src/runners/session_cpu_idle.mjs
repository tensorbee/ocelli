import { runBrowserSession } from "../browser_session.mjs";

export const id = "session.cpu_idle";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
