// Deterministic ~100 KB order-history document (the RFC's order payload, at scale).
// Usage: node generate.mjs > order-100kb.json
let seed = 42;
const rand = () => (seed = (seed * 48271) % 2147483647) / 2147483647;
const pick = (xs) => xs[Math.floor(rand() * xs.length)];

const statuses = ["fulfilled", "pending", "shipped", "cancelled"];
const items = ["widget", "gadget", "sprocket", "flange", "grommet"];

const orders = [];
while (JSON.stringify({ orders }).length < 100 * 1024) {
  orders.push({
    id: `ORD-${orders.length + 1}`,
    status: pick(statuses),
    customer: {
      id: Math.floor(rand() * 100000),
      name: `Customer ${Math.floor(rand() * 1000)}`,
      email: `customer${Math.floor(rand() * 1000)}@example.com`,
    },
    lines: Array.from({ length: 1 + Math.floor(rand() * 4) }, (_, i) => ({
      line: i + 1,
      sku: `${pick(items)}-${Math.floor(rand() * 100)}`,
      quantity: 1 + Math.floor(rand() * 9),
      price: Math.round(rand() * 10000) / 100,
    })),
    created: "2026-08-23T10:00:00Z",
    tags: [pick(items), pick(items)],
  });
}
process.stdout.write(JSON.stringify({ orders }));
