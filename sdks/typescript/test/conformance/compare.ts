// How a case's expectation is compared with what the SDK did. Documents are compared by value:
// object members are a set, because ADR 0017 compares content and a language cannot always
// preserve the order its author wrote (6.3's report §2.1); arrays are compared in order, because
// a list of states or array entries is content.

/** Deep equality: object members order-insensitive, arrays order-sensitive, numbers by value. */
export function equal(a: unknown, b: unknown): boolean {
  if (a === b) {
    return true;
  }
  if (typeof a === "number" && typeof b === "number") {
    return a === b || (Number.isNaN(a) && Number.isNaN(b));
  }
  if (Array.isArray(a) || Array.isArray(b)) {
    return (
      Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((item, i) => equal(item, b[i]))
    );
  }
  if (isObject(a) && isObject(b)) {
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every((k) => k in b && equal(a[k], b[k]));
  }
  return false;
}

/** Every member `expected` names is in `actual` and equal; members it does not name are not checked. */
export function subset(expected: unknown, actual: unknown): boolean {
  if (isObject(expected) && isObject(actual)) {
    return Object.entries(expected).every(([k, v]) => k in actual && subset(v, actual[k]));
  }
  return equal(expected, actual);
}

/** The value at a JSON pointer (RFC 6901), or `undefined` when the path is not there. */
export function pointer(document: unknown, path: string): unknown {
  if (path === "") {
    return document;
  }
  let here: unknown = document;
  for (const raw of path.replace(/^\//, "").split("/")) {
    const token = raw.replace(/~1/g, "/").replace(/~0/g, "~");
    if (Array.isArray(here)) {
      here = here[Number(token)];
    } else if (isObject(here)) {
      here = here[token];
    } else {
      return undefined;
    }
  }
  return here;
}

/** One line per side, for a failure message. */
export function show(value: unknown): string {
  return JSON.stringify(value) ?? String(value);
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
