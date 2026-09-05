// A static file server for the built page, on loopback, on an ephemeral port.
//
// The page needs an HTTP origin rather than `file://` because the decode
// worker is a module worker, and a module worker from `file://` is refused by
// the browser's origin rules. It serves `page/dist` and nothing else. The
// corpus is NOT reachable through it: bytes reach the page as an argument to
// `page.evaluate`, so no server in this harness can read `corpus/data`.

import { createServer } from "node:http";
import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { extname, join, resolve, sep } from "node:path";

const CONTENT_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json; charset=utf-8",
};

/**
 * Where a request path lands on disk, or `null` if it lands outside `base`.
 *
 * Exported so `tests/server_test.mjs` can watch the refusal fire. A guard
 * reachable only through a browser is a guard nobody has watched fail.
 *
 * `join` normalises, so it is `join` that resolves `..` and therefore `join`
 * that can walk out of the served directory. The containment check is made
 * against its RESULT, and never against the request string, because a check
 * that inspects the request and then normalises it is checking something other
 * than the path that gets opened.
 */
export function resolveServedPath(base, pathname) {
  const root = resolve(base);
  let decoded;
  try {
    decoded = decodeURIComponent(pathname);
  } catch {
    // A malformed percent escape is not a path. Refusing here keeps the throw
    // out of the request handler, where it would take the process down.
    return null;
  }
  const candidate = resolve(join(root, decoded === "/" ? "/index.html" : decoded));
  if (candidate !== root && !candidate.startsWith(root + sep)) {
    return null;
  }
  return candidate;
}

/**
 * Serve `root` on 127.0.0.1.
 *
 * @returns {Promise<{origin: string, close: () => Promise<void>}>}
 */
export async function serveDirectory(root) {
  const base = resolve(root);

  const server = createServer((request, response) => {
    const requested = new URL(request.url, "http://127.0.0.1");
    const candidate = resolveServedPath(base, requested.pathname);
    if (candidate === null) {
      response.writeHead(403).end("outside the served directory");
      return;
    }
    stat(candidate)
      .then((info) => {
        if (!info.isFile()) {
          response.writeHead(404).end("not a file");
          return;
        }
        response.writeHead(200, {
          "content-type":
            CONTENT_TYPES[extname(candidate)] ?? "application/octet-stream",
          "content-length": info.size,
          "cache-control": "no-store",
        });
        createReadStream(candidate).pipe(response);
      })
      .catch(() => {
        response.writeHead(404).end("not found");
      });
  });

  await new Promise((resolveListen, rejectListen) => {
    server.once("error", rejectListen);
    server.listen(0, "127.0.0.1", resolveListen);
  });

  const address = server.address();
  return {
    origin: `http://127.0.0.1:${address.port}`,
    close: () => new Promise((done) => server.close(() => done())),
  };
}
