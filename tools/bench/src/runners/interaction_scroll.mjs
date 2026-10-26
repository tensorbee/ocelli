import { runBrowserSession } from "../browser_session.mjs";

export const id = "interaction.scroll";

export async function run(options = {}) {
  return runBrowserSession(id, options);
}
