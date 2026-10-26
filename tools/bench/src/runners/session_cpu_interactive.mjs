import { runBrowserSession } from "../browser_session.mjs";

export const id = "session.cpu_interactive";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
