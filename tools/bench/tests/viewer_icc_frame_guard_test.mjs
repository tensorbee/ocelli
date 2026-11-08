// F-240 AC15 fixture refusal controls. Every profile and data file is copied
// into a disposable tiny fixture root. Production files are never modified.
import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const fixtureUrl = new URL("../src/viewer_icc_frame_fixture.mjs", import.meta.url);
const assetsUrl = new URL("./fixtures/f240-icc/", import.meta.url);

// Defensive leaf guards in a fixed, checksum-bound asset cannot receive bad
// public inputs. This test-owned copy exports those two leaves and points only
// its own benchPath at tiny disposable assets, leaving the actual body intact.
async function withFixture(options, observe) {
  const root = await mkdtemp(join(tmpdir(), "ocelli-icc-guards-"));
  try {
    const dir = join(root, "tests", "fixtures", "f240-icc");
    await mkdir(dir, { recursive: true });
    const provenance = JSON.parse(await readFile(new URL("profile-provenance.json", assetsUrl), "utf8"));
    const hand = JSON.parse(await readFile(new URL("hand-vectors.json", assetsUrl), "utf8"));
    const profiles = {};
    for (const name of ["icc_gamma1.icc", "icc_gamma22.icc"]) {
      profiles[name] = await readFile(new URL(name, assetsUrl));
    }
    if (options.profile) {
      options.profile(profiles["icc_gamma1.icc"]);
      provenance.bench_assets["icc_gamma1.icc"].sha256 = createHash("sha256")
        .update(profiles["icc_gamma1.icc"]).digest("hex");
    }
    options.provenance?.(provenance);
    options.hand?.(hand);
    if (options.corruptGamma22) profiles["icc_gamma22.icc"][200] ^= 1;
    await writeFile(join(dir, "profile-provenance.json"), JSON.stringify(provenance));
    await writeFile(join(dir, "hand-vectors.json"), JSON.stringify(hand));
    for (const [name, bytes] of Object.entries(profiles)) await writeFile(join(dir, name), bytes);
    let source = await readFile(fixtureUrl, "utf8");
    const pathImport = 'import { benchPath } from "./paths.mjs";';
    assert.equal(source.split(pathImport).length, 2);
    source = source.replace(pathImport,
      `const benchPath = (...parts) => ${JSON.stringify(root)} + "/" + parts.join("/");`);
    source = source.replace(/from "(\.\/[^"\n]+)"/g,
      (_, path) => `from ${JSON.stringify(new URL(path, fixtureUrl).href)}`);
    if (options.source) source = options.source(source);
    source += "\nexport { longHeader, writeAll };\n";
    const load = () => import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
    await observe(load, root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function refuses(action, text) {
  assert.throws(action, error => error instanceof Error && error.message.includes(text));
}

function tagEntry(profile, signature) {
  for (let i = 0; i < profile.readUInt32BE(128); i += 1) {
    const entry = 132 + 12 * i;
    if (profile.toString("latin1", entry, entry + 4) === signature) return entry;
  }
  throw new Error(`test profile has no ${signature} entry`);
}

test("ICC asset provenance must exist and agree with the actual bytes", async () => {
  await withFixture({}, async (load, root) => {
    const module = await load();
    const fixtures = await module.writeIccFrameFixtures(join(root, "built"), 4);
    assert.ok(fixtures.icc8.fileBytes > 48);
    assert.ok(fixtures.icc16.fileBytes > 96);
  });
  await withFixture({ provenance(p) { delete p.bench_assets["icc_gamma22.icc"]; } }, async (load, root) => {
    const module = await load();
    await assert.rejects(() => module.writeIccFrameFixtures(join(root, "built"), 4), /no provenance record/);
  });
  await withFixture({ corruptGamma22: true }, async (load, root) => {
    const module = await load();
    await assert.rejects(() => module.writeIccFrameFixtures(join(root, "built"), 4), /not its recorded/);
  });
});

for (const [name, mutate, text] of [
  ["curv type", p => p.write("para", 300, "latin1"), "does not hold a curv payload"],
  ["count-one gamma", p => p.writeUInt32BE(2, 308), "does not hold a count-1 gamma"],
  ["aligned TRC room", p => p.writeUInt32BE(315, tagEntry(p, "gTRC") + 4), "no aligned room"],
  ["all three TRCs", p => p.write("xxxx", tagEntry(p, "bTRC"), "latin1"), "rewrote 2 TRC tags"],
]) {
  test(`ICC table derivation requires ${name}`, async () => {
    await withFixture({ profile: mutate }, async load => {
      const module = await load();
      refuses(() => module.linear16TableProfile(), text);
    });
  });
}

test("ICC hand-vector and marker shapes cannot silently lose a sample", async () => {
  await withFixture({ hand(h) { h.rows.splice(h.rows.findIndex(row => row.bits === 8), 1); } }, async load => {
    await assert.rejects(load, /the hand vectors carry 2 8-bit/);
  });
  await withFixture({ source(s) {
    const marker = "  [2, 2],\n];";
    assert.equal(s.split(marker).length, 2);
    return s.replace(marker, "];");
  } }, async load => {
    await assert.rejects(load, /marker positions do not cover/);
  });
});

test("ICC helpers refuse unknown workloads, rows, windows and reply variants", async () => {
  await withFixture({}, async load => {
    const m = await load();
    for (const f of [m.iccMarkers, m.iccSampleBytes, m.iccTrcTableBytes]) {
      refuses(() => f("absent"), "unknown ICC frame workload");
    }
    refuses(() => m.iccExpectedDisplay("icc8", 63), "no hand vector row");
    refuses(() => m.iccWindowBytes("icc8", 64, { center: 999, width: 200 }), "no hand vector window");
    refuses(() => m.iccReplyByte("icc8", 0, 0, 0, { center: 128, width: 256 }, "indexed8"), "no 16-bit indexing");
    refuses(() => m.iccReplyByte("icc8", 0, 0, 0, { center: 128, width: 256 }, "absent"), "unknown marker byte variant");
  });
});

test("ICC fixture leaf controls require even VL and progressing writes", async () => {
  await withFixture({}, async load => {
    const { longHeader, writeAll } = await load();
    assert.equal(longHeader(0x7fe0, 0x0010, "OW", 2).length, 12);
    refuses(() => longHeader(0x7fe0, 0x0010, "OW", 1), "bad length");
    const positions = [];
    await writeAll({ async write(buffer, offset, count) {
      positions.push([offset, count]); return { bytesWritten: Math.min(2, count) };
    } }, Buffer.from([1, 2, 3]));
    assert.deepEqual(positions, [[0, 3], [2, 1]]);
    for (const bytesWritten of [0, -1]) {
      await assert.rejects(() => writeAll({ async write() { return { bytesWritten }; } }, Buffer.from([1])), /made no progress/);
    }
  });
});
