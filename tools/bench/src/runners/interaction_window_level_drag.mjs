import { runBrowserSession } from "../browser_session.mjs";

export const id = "interaction.window_level_drag";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
