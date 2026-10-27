/**
 * F-103 catalogue and payload fixtures.
 *
 * Kind numbers come from the generated module. The expected bytes below are
 * written out by hand from the encoder comments, not by calling the decoder.
 */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { RING_EVENT_KIND } from "./generated.js";
import {
  CORNERSTONE_EVENT_COUNT,
  CORNERSTONE_EVENT_MAP,
  EVENT_DELIVERY,
  OCELLI_EVENTS,
  UNMAPPED_EVENTS,
  decodeNamedEvent,
} from "./events.js";
import type { OcelliEvent } from "./ring.js";

const HERE = dirname(fileURLToPath(import.meta.url));

function event(kind: number, payload: Uint8Array): OcelliEvent {
  return { kind, viewport: 9, seq: 11n, payload };
}

describe("F-103 event catalogue", () => {
  it("mapping_is_complete", () => {
    expect(CORNERSTONE_EVENT_COUNT).toBe(103);
    expect(CORNERSTONE_EVENT_MAP).toHaveLength(103);
    const core = CORNERSTONE_EVENT_MAP.filter((row) => row.source === "core");
    const tool = CORNERSTONE_EVENT_MAP.filter((row) => row.source === "tool");
    expect(core).toHaveLength(50);
    expect(tool).toHaveLength(53);
    const names = CORNERSTONE_EVENT_MAP.map((row) => row.cornerstone);
    expect(new Set(names).size).toBe(names.length);
    for (const row of CORNERSTONE_EVENT_MAP) {
      expect(EVENT_DELIVERY).toContain(row.delivery);
      if (row.delivery === "unavailable") {
        expect(row.ocelli).toBeNull();
        expect(row.producer).toBeNull();
        expect(row.reason).toBeTruthy();
      } else {
        expect(row.ocelli).toBeTruthy();
        expect(row.producer).toBeTruthy();
      }
    }
  });

  it("names_are_unique", () => {
    expect(new Set(OCELLI_EVENTS).size).toBe(OCELLI_EVENTS.length);
    const mappedNames = CORNERSTONE_EVENT_MAP.flatMap((row) =>
      row.ocelli === null ? [] : [row.ocelli],
    );
    expect(new Set(mappedNames).size).toBe(mappedNames.length);
    const mapped = new Set(mappedNames);
    const unmapped = new Set(UNMAPPED_EVENTS.map((row) => row.name));
    for (const name of OCELLI_EVENTS) {
      const inMap = mapped.has(name);
      const inUnmapped = unmapped.has(name);
      expect(inMap === inUnmapped).toBe(false);
    }
    for (const row of UNMAPPED_EVENTS) {
      expect(row.producer.length).toBeGreaterThan(0);
      expect(row.reason.length).toBeGreaterThan(0);
    }
  });

  it("ring_kinds_match_generated", () => {
    const expected = new Map<number, string>([
      [RING_EVENT_KIND.evicted, "cache.evicted"],
      [RING_EVENT_KIND.displaced, "cache.displaced"],
      [RING_EVENT_KIND.refused, "cache.refused"],
      [RING_EVENT_KIND.pressure, "cache.pressure"],
      [RING_EVENT_KIND.budget_changed, "cache.budgetChanged"],
      [RING_EVENT_KIND.presented, "frame.presented"],
    ]);
    expect(expected.size).toBe(6);
    const ringNames = [
      ...CORNERSTONE_EVENT_MAP.filter((row) => row.delivery === "ring").map(
        (row) => row.ocelli,
      ),
      ...UNMAPPED_EVENTS.filter((row) => row.delivery === "ring").map((row) => row.name),
    ];
    expect(new Set(ringNames)).toEqual(new Set(expected.values()));
    expect(ringNames).toHaveLength(expected.size);
    expect([...expected.keys()].sort((left, right) => left - right)).toEqual(
      Object.values(RING_EVENT_KIND).sort((left, right) => left - right),
    );
  });

  it("decodes_every_kind", () => {
    const payload = new Uint8Array(32);
    const decoded = [
      decodeNamedEvent(event(RING_EVENT_KIND.evicted, payload)),
      decodeNamedEvent(event(RING_EVENT_KIND.displaced, payload)),
      decodeNamedEvent(event(RING_EVENT_KIND.refused, payload)),
      decodeNamedEvent(event(RING_EVENT_KIND.pressure, payload)),
      decodeNamedEvent(event(RING_EVENT_KIND.budget_changed, payload)),
      decodeNamedEvent(event(RING_EVENT_KIND.presented, payload)),
    ];
    expect(decoded.map((row) => row?.name)).toEqual([
      "cache.evicted",
      "cache.displaced",
      "cache.refused",
      "cache.pressure",
      "cache.budgetChanged",
      "frame.presented",
    ]);
    for (const row of decoded) {
      expect(row?.viewport).toBe(9);
      expect(row?.seq).toBe(11n);
    }
  });

  it("refuses_unknown_kind", () => {
    const payload = new Uint8Array(32);
    expect(decodeNamedEvent(event(0, payload))).toBeNull();
    expect(decodeNamedEvent(event(7, payload))).toBeNull();
  });

  it("decodes_fixture_payloads", () => {
    // CacheEvent::encode. series 7, instance 3, frame 5, bytes 4096.
    // 4096 is 0x1000. Offsets 0, 8, 12 and 16. HLD section 17.3 envelope.
    const key = new Uint8Array(32);
    key.set([
      0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x00,
      0x05, 0x00, 0x00, 0x00,
      0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    expect(decodeNamedEvent(event(RING_EVENT_KIND.evicted, key))?.payload).toEqual({
      series: 7n,
      instance: 3,
      frame: 5,
      bytes: 4096n,
    });

    // TelemetryEvent::encode pressure. Tier 2 is GPU. used 262144 is one
    // 512-row allocation written as 0x00040000. budget 524288 is 0x00080000.
    const pressure = new Uint8Array(32);
    pressure.set([
      2, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x04, 0x00, 0x00, 0x00,
      0x01, 0x00, 0x00, 0x00,
      0x02, 0x00, 0x00, 0x00,
    ]);
    expect(decodeNamedEvent(event(RING_EVENT_KIND.pressure, pressure))?.payload).toEqual({
      tier: 2,
      used: 262144n,
      budget: 524288n,
      entries: 4,
      evicted: 1,
      refused: 2,
    });

    // Budget change. Tier 1, budget 100, evicted bytes 40, evicted 2, dropped 3.
    const budget = new Uint8Array(32);
    budget.set([
      1, 0x00, 0x00, 0x00,
      0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x02, 0x00, 0x00, 0x00,
      0x03, 0x00, 0x00, 0x00,
    ]);
    expect(
      decodeNamedEvent(event(RING_EVENT_KIND.budget_changed, budget))?.payload,
    ).toEqual({
      tier: 1,
      budget: 100n,
      evictedBytes: 40n,
      evicted: 2,
      dropped: 3,
    });

    // push_presented. generation 9, drawn 80 by 40, FLAG_INVERT.
    const presented = new Uint8Array(32);
    presented.set([
      0x09, 0x00, 0x00, 0x00,
      0x50, 0x00, 0x00, 0x00,
      0x28, 0x00, 0x00, 0x00,
      0x01, 0x00, 0x00, 0x00,
    ]);
    expect(decodeNamedEvent(event(RING_EVENT_KIND.presented, presented))?.payload).toEqual({
      generation: 9,
      drawnWidth: 80,
      drawnHeight: 40,
      flags: 1,
    });
  });

  it("refuses_a_short_or_unknown_payload", () => {
    expect(decodeNamedEvent(event(RING_EVENT_KIND.evicted, new Uint8Array(23)))).toBeNull();
    expect(decodeNamedEvent(event(RING_EVENT_KIND.pressure, new Uint8Array(31)))).toBeNull();
    expect(
      decodeNamedEvent(event(RING_EVENT_KIND.budget_changed, new Uint8Array(27))),
    ).toBeNull();
    expect(decodeNamedEvent(event(RING_EVENT_KIND.presented, new Uint8Array(15)))).toBeNull();
    const badTier = new Uint8Array(32);
    badTier[0] = 3;
    expect(decodeNamedEvent(event(RING_EVENT_KIND.pressure, badTier))).toBeNull();
  });

  it("does_not_listen_and_does_not_declare_a_second_catalogue", () => {
    const marker = "export const " + "OCELLI_EVENTS =";
    const dirs = [HERE, join(HERE, "..", "..", "react", "src")];
    const listeners: string[] = [];
    const declarations: string[] = [];
    for (const dir of dirs) {
      for (const name of readdirSync(dir)) {
        const sourceFile = name.endsWith(".ts") || name.endsWith(".tsx");
        const testFile = name.endsWith(".test.ts") || name.endsWith(".test.tsx");
        if (!sourceFile || testFile) {
          continue;
        }
        const text = readFileSync(join(dir, name), "utf8");
        if (text.includes("addEventListener")) {
          listeners.push(name);
        }
        if (text.includes(marker)) {
          declarations.push(name);
        }
      }
    }
    expect(listeners).toEqual([]);
    expect(declarations).toEqual(["events.ts"]);
  });
});
