// facade/pact-js-matchers-patterns: pact-js's pattern matchers, anchored since pact-js's patterns match
// the whole value, and its date and time matchers with their formats.
import assert from "node:assert/strict";
import { Matchers, Pact } from "pact-janus/pact";

const m = Matchers;

async function served(dir: string, description: string, body: Record<string, unknown>): Promise<Record<string, unknown>> {
  return new Pact({ consumer: "web-app", provider: "orders-api", dir })
    .addInteraction()
    .uponReceiving(description)
    .withRequest("GET", "/orders/1")
    .willRespondWith(200, (builder) => {
      builder.jsonBody(body);
    })
    .executeTest(async (mock) => (await fetch(`${mock.url}/orders/1`)).json() as Promise<Record<string, unknown>>);
}

export async function patterns({ dir }: { dir: string }): Promise<void> {
  const order = await served(dir, "an order of patterned values", {
    code: m.regex(/\d{3}-\d{2}/, "123-45"),
    email: m.email("ada@example.com"),
    ipv4: m.ipv4Address("10.0.0.1"),
    ipv6: m.ipv6Address("::1"),
    date: m.iso8601Date("2026-11-09"),
    dateTime: m.iso8601DateTime("2026-11-09T10:00:00Z"),
    dateTimeMillis: m.iso8601DateTimeWithMillis("2026-11-09T10:00:00.123Z"),
    time: m.iso8601Time("T10:00:00"),
    rfc1123: m.rfc1123Timestamp("Mon, 09 Nov 2026 10:00:00 +0000"),
    hex: m.hexadecimal("3F"),
    id: m.uuid("ce118b6e-d8e1-11e7-9296-cec278b6b50a"),
  });
  assert.equal(order.code, "123-45");
  assert.equal(order.id, "ce118b6e-d8e1-11e7-9296-cec278b6b50a");
}

export async function datesAndTimes({ dir }: { dir: string }): Promise<void> {
  const order = await served(dir, "an order of dates and times", {
    placed: m.datetime("yyyy-MM-dd'T'HH:mm:ss", "2026-11-09T10:00:00"),
    stamped: m.timestamp("yyyy-MM-dd HH:mm:ss", "2026-11-09 10:00:00"),
    due: m.date("yyyy-MM-dd", "2026-11-20"),
    slot: m.time("HH:mm", "09:30"),
  });
  assert.deepEqual(order, { placed: "2026-11-09T10:00:00", stamped: "2026-11-09 10:00:00", due: "2026-11-20", slot: "09:30" });
}
