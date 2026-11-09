// Types for the example's tests. The workspace has no @types/node dependency.
// F-228 reads the documents that quote the policy, the wasm crate's resolved
// dependency graph and the shipped sources.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
  export function existsSync(path: string): boolean;
  export function readdirSync(
    path: string,
    options: { recursive: true, encoding: "utf8" },
  ): string[];
  export interface Dirent {
    isDirectory(): boolean;
    readonly name: string;
  }
  export function readdirSync(
    path: string,
    options: { withFileTypes: true },
  ): Dirent[];
}

declare module "node:child_process" {
  export interface SpawnSyncResult {
    status: number | null;
    stdout: string;
    stderr: string;
    readonly error: Error | undefined;
  }
  export function spawnSync(
    command: string,
    args: readonly string[],
    options: { cwd: string, encoding: "utf8" },
  ): SpawnSyncResult;
}

declare module "node:path" {
  export function join(...parts: string[]): string;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
}
