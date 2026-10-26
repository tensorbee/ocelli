// Types for the package tests. The package has no @types/node dependency.
// F-100 reads a fixture with readFileSync. The wasm boundary test reads the
// built module and resolves its path.

declare module "node:fs" {
  export function readFileSync(path: string, encoding: "utf8"): string;
}

declare module "node:fs/promises" {
  export function readFile(path: string): Promise<Uint8Array>;
}

declare module "node:path" {
  export function dirname(path: string): string;
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
}

declare module "node:url" {
  export function fileURLToPath(url: string | URL): string;
  export function pathToFileURL(path: string): URL;
}

declare module "node:process" {
  export const env: Record<string, string | undefined>;
}
