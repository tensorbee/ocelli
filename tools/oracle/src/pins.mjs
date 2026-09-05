// Finding the installed version of a package, whatever its exports map says.
//
// HLD 15.2's reason applied to the reference rather than to wgpu: **an oracle
// that drifts is not an oracle.** A version the run cannot read is a version
// the run cannot pin, so the failure this module exists to prevent is a
// dependency that is silently exempt from the check.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";

/** How far above a package's main entry its own manifest may sit. */
const MAX_WALK = 8;

/**
 * The version in the `package.json` that NAMES this package, walking up from
 * `start`.
 *
 * Matching the name is what makes the walk safe rather than lucky. Measured
 * today, every package that reaches this function has exactly one manifest, at
 * its own root, and the walk starts one level below that in `dist/` or `src/`,
 * so it is found on the second step. But nested `package.json` files
 * carrying nothing but `{"type": "module"}`, with no name and no version, do
 * exist in this tree, in five of the `@cornerstonejs/*` packages, and all five
 * of those are reached by the direct route in `installedVersion` instead. A walk
 * that took the first manifest it met would read `undefined` from one of those
 * and record it as an installed version the day an exports map moves a package
 * from one route to the other. That is worse than not checking, because it
 * reads as a check.
 *
 * @param {string} start a directory inside the package
 * @param {string} name the package's own name
 * @returns {string} the version
 */
export function versionFromPackageRoot(start, name) {
  let directory = start;
  for (let depth = 0; depth < MAX_WALK; depth += 1) {
    try {
      const manifest = JSON.parse(
        readFileSync(join(directory, "package.json"), "utf8"),
      );
      if (manifest.name === name && typeof manifest.version === "string") {
        return manifest.version;
      }
    } catch {
      // No package.json here, or not readable. Keep walking.
    }
    const parent = dirname(directory);
    if (parent === directory) {
      break;
    }
    directory = parent;
  }
  throw new Error(
    `no package.json naming ${name} within ${MAX_WALK} directories above ` +
      `${start}`,
  );
}

/**
 * The installed version of a package.
 *
 * `require("<name>/package.json")` is the direct route, and several of these
 * packages do not expose that subpath in their exports map. The fallback
 * resolves the package's main entry and walks up from it.
 *
 * @param {(specifier: string) => unknown} require a `createRequire` bound to
 *   the caller, so resolution happens from where the packages are installed
 */
export function installedVersion(require, name) {
  try {
    return require(`${name}/package.json`).version;
  } catch {
    // Not exported. Fall through to the walk.
  }
  return versionFromPackageRoot(dirname(require.resolve(name)), name);
}
