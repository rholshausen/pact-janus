#!/usr/bin/env node
// `csv` — a Janus content component for `text/csv`, in JavaScript, with no dependencies.
//
// The same component as `third-party/janus-csv`, written again in another language and reached through
// the subprocess binding (component-interfaces spec §9.3, staged plan S5.2b, ADR 0108). It is written
// against the published component interfaces, and it is held to the Rust component's own conformance
// cases (`third-party/janus-csv/conformance/`), so the cases say what a CSV component does and not what
// one implementation of it happens to do.
//
// It implements the `content` interface (`decode`, `encode`, `compile`; it declines `detect`), and the
// `matcher` interface's `apply`, for the actions its handshake substitutes for the value operators whose
// meaning CSV changes, and `compare`, for its one operator, `csv:column`, which it declares `exact`.
//
// CSV is lossy, and this component says so (spec §6.4). Every field decodes as a string. A document with
// a header row decodes to an array of objects, one member per column in header order; a body declared
// `header=absent` decodes to an array of arrays.

import { failure, isObject, serve } from "./pipe.mjs";

const NAME = "csv";
const VERSION = "1.0.0";
const MEDIA_TYPE = "text/csv";
const PROTOCOL_VERSION = 1;
const OPERATOR = "csv:column";
const ACTIONS = ["csv:integer", "csv:number", "csv:boolean", "csv:equality", OPERATOR];

let greeted = false;

serve((op, body) => {
  if (op === "component/hello") return hello(body);
  if (!greeted) throw failure("handshake-required", "protocol", `'${op}' before component/hello`);
  switch (op) {
    case "component/shutdown":
      return {};
    case "content/decode":
      return decode(body);
    case "content/encode":
      return encode(body);
    case "content/compile":
      contentType(body);
      return {};
    case "matcher/apply":
      return apply(body);
    case "matcher/compare":
      return compare(body);
    default:
      throw failure("operation-unsupported", "protocol", `the csv component does not implement '${op}'`, { op });
  }
});

function hello(body) {
  const offered = Array.isArray(body["component-protocol-versions"]) ? body["component-protocol-versions"] : [];
  if (!offered.includes(PROTOCOL_VERSION)) {
    throw failure("protocol-version-unsupported", "protocol", "the csv component speaks component protocol 1 only", {
      supported: [PROTOCOL_VERSION],
    });
  }
  greeted = true;
  return {
    "component-protocol-version": PROTOCOL_VERSION,
    component: { name: NAME, version: VERSION },
    interfaces: ["content", "matcher"],
    contributes: {
      actions: ACTIONS.map((name) => ({ name })),
      operators: [{ name: OPERATOR, comparability: "exact" }],
      "content-types": [
        {
          "media-type": MEDIA_TYPE,
          substitutions: {
            integer: "csv:integer",
            number: "csv:number",
            decimal: "csv:number",
            boolean: "csv:boolean",
            equality: "csv:equality",
          },
          degradations: [
            {
              code: "string-only",
              message:
                "CSV carries no types: every field decodes as a string, so integers, decimals and booleans are the text that spells them.",
            },
            {
              code: "no-null",
              message: "An empty field decodes as the empty string; CSV cannot tell it from an absent or null value.",
            },
          ],
        },
      ],
    },
  };
}

// --- content ------------------------------------------------------------------------------------------

function decode(body) {
  const type = contentType(body);
  const header = headerMode(type, body.options);
  const octets = slotOctets(body.value);
  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(octets);
  } catch (err) {
    throw decodeFailed(`CSV is not UTF-8: ${err.message}`);
  }
  const rows = parse(text);
  const degradations = [];
  let document;
  if (header) {
    if (rows.length === 0) throw decodeFailed("header=present but the body has no header row");
    const [names, ...records] = rows;
    names.forEach((name, i) => {
      if (names.indexOf(name) !== i) throw decodeFailed(`column '${name}' appears twice in the header row`, 1);
    });
    names.forEach((name, i) => {
      degradations.push(...columnDegradations(records.map((record) => record[i]), `$[*]${memberPath(name)}`, name));
    });
    document = records.map((record) => new Map(names.map((name, i) => [name, record[i]])));
  } else {
    const width = rows.length > 0 ? rows[0].length : 0;
    for (let i = 0; i < width; i++) {
      degradations.push(...columnDegradations(rows.map((row) => row[i]), `$[*][${i}]`, `${i}`));
    }
    document = rows;
  }
  const result = { document };
  if (degradations.length > 0) result.degradations = degradations;
  return result;
}

/** RFC 4180: fields separated by commas, records by CRLF or LF, quoted with `"` and `""` inside. Every
 * record must have as many fields as the first, and a line with nothing on it is not a record. */
function parse(text) {
  const rows = [];
  let row = [];
  let field = "";
  let quoted = false;
  let line = 1;
  let recordLine = 1;
  let empty = true;
  const endRecord = () => {
    if (!(empty && row.length === 0 && field === "")) {
      row.push(field);
      if (rows.length > 0 && row.length !== rows[0].length) {
        throw decodeFailed(
          `record ${rows.length + 1} (line ${recordLine}) has ${row.length} fields, and the first has ${rows[0].length}`,
          recordLine,
        );
      }
      rows.push(row);
    }
    row = [];
    field = "";
    empty = true;
    recordLine = line;
  };
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          quoted = false;
        }
      } else {
        if (ch === "\n") line++;
        field += ch;
      }
      continue;
    }
    if (ch === '"' && field === "") {
      quoted = true;
      empty = false;
    } else if (ch === ",") {
      row.push(field);
      field = "";
      empty = false;
    } else if (ch === "\r" && text[i + 1] === "\n") {
      // CRLF: the LF ends the record.
    } else if (ch === "\n" || ch === "\r") {
      line++;
      endRecord();
    } else {
      field += ch;
      empty = false;
    }
  }
  if (quoted) throw decodeFailed("a quoted field is not closed", recordLine);
  endRecord();
  return rows;
}

/** Where a loss actually bit, per column (spec §6.4). */
function columnDegradations(column, path, name) {
  const out = [];
  const nonEmpty = column.filter((value) => value !== "");
  if (nonEmpty.length > 0 && nonEmpty.every(spellsAFloat)) {
    out.push({ code: "numeric-lexical", path, message: `column '${name}' decoded as strings; CSV has no numeric type` });
  } else if (nonEmpty.length > 0 && nonEmpty.every((value) => value === "true" || value === "false")) {
    out.push({ code: "no-boolean", path, message: `column '${name}' decoded as strings; CSV has no boolean type` });
  }
  if (column.some((value) => value === "")) {
    out.push({ code: "no-null", path, message: `column '${name}' has empty fields, decoded as the empty string` });
  }
  return out;
}

function encode(body) {
  const type = contentType(body);
  const header = headerMode(type, body.options);
  const encoded = body.encoded ?? "json";
  if (encoded !== "json") throw encodeFailed(`a CSV document is structure, not '${encoded}'-encoded octets`);
  const rows = body.document;
  if (!Array.isArray(rows)) throw encodeFailed("a CSV document is an array of rows", "$");
  const records = [];
  if (header) {
    let columns = [];
    rows.forEach((row, i) => {
      if (!isObject(row)) throw encodeFailed("with header=present every row is an object", `$[${i}]`);
      for (const name of Object.keys(row)) if (!columns.includes(name)) columns.push(name);
    });
    if (rows.length === 0 && body.shape !== undefined) columns = shapeColumns(body.shape);
    if (columns.length > 0) records.push(columns);
    rows.forEach((row, i) => {
      records.push(columns.map((name) => field(row[name] ?? null, `$[${i}]${memberPath(name)}`)));
    });
  } else {
    rows.forEach((row, i) => {
      if (!Array.isArray(row)) throw encodeFailed("with header=absent every row is an array", `$[${i}]`);
      records.push(row.map((value, j) => field(value, `$[${i}][${j}]`)));
    });
  }
  const text = records.map((record) => `${record.map((value) => quote(value, record.length)).join(",")}\r\n`).join("");
  return { value: { content: Buffer.from(text, "utf8").toString("base64"), encoded: "base64", "content-type": type } };
}

/** A field quoted only where it must be: a comma, a quote or a line break in it, or the one empty field
 * of a record, which would otherwise be an empty line. */
function quote(value, width) {
  if (/[",\r\n]/.test(value) || (value === "" && width === 1)) return `"${value.replaceAll('"', '""')}"`;
  return value;
}

/** The columns a body of `shape` has: its rows' object members, in declared order. */
function shapeColumns(shape) {
  let row;
  if (shape?.shape === "each-like") row = shape.items;
  else if (shape?.shape === "array") row = Array.isArray(shape.entries) ? shape.entries[0] : undefined;
  return rowMembers(row) ?? [];
}

function rowMembers(row) {
  if (!isObject(row)) return undefined;
  if (row.shape === "object" && isObject(row.members)) return Object.keys(row.members);
  if (row.shape === "one-of" && isObject(row.alternatives)) {
    for (const alternative of Object.values(row.alternatives)) {
      const members = rowMembers(alternative);
      if (members) return members;
    }
  }
  return undefined;
}

function field(value, path) {
  if (typeof value === "string") return value;
  if (value === null) return "";
  if (typeof value === "boolean" || typeof value === "number") return String(value);
  throw encodeFailed("CSV fields are flat: a field cannot hold an array or an object", path);
}

/** The request's content type, which must be ours. `charset` must be UTF-8; `header` is read by
 * `headerMode`; any other parameter is ignored. */
function contentType(body) {
  const type = typeof body["content-type"] === "string" ? body["content-type"] : "";
  const [essence, ...parameters] = type.split(";").map((part) => part.trim());
  if (essence.toLowerCase() !== MEDIA_TYPE) {
    throw failure("unsupported-content-type", "document", `the csv component handles text/csv, not '${type}'`, {
      "content-type": type,
    });
  }
  for (const parameter of parameters) {
    const [name, value = ""] = parameter.split("=");
    const charset = value.trim().replace(/^"|"$/g, "").toLowerCase();
    if (name.trim().toLowerCase() === "charset" && !["utf-8", "utf8", "us-ascii"].includes(charset)) {
      throw failure("unsupported-content-type", "document", `the csv component reads UTF-8 only, not charset '${value.trim()}'`, {
        "content-type": type,
      });
    }
  }
  return type;
}

/** RFC 4180's `header` parameter, then the `header` option, then `present`. */
function headerMode(type, options) {
  const fromType = type
    .split(";")
    .slice(1)
    .map((parameter) => parameter.split("="))
    .find(([name]) => name.trim().toLowerCase() === "header");
  const mode = fromType
    ? (fromType[1] ?? "").trim().replace(/^"|"$/g, "").toLowerCase()
    : typeof options?.header === "string"
      ? options.header.toLowerCase()
      : undefined;
  if (mode === undefined || mode === "present") return true;
  if (mode === "absent") return false;
  throw failure("invalid-config", "document", `header must be 'present' or 'absent', not '${mode}'`, { header: mode });
}

/** A slot value's octets (parts schema `SlotValue`): base64, text, or a JSON string. */
function slotOctets(value) {
  if (!isObject(value)) throw decodeFailed("'value' must be a slot value");
  const encoded = value.encoded ?? "json";
  const content = value.content;
  if (typeof content === "string") {
    if (encoded === "base64") {
      if (!/^[A-Za-z0-9+/]*={0,2}$/.test(content) || content.length % 4 === 1) {
        throw decodeFailed("slot content is not base64");
      }
      return Buffer.from(content, "base64");
    }
    if (encoded === "text" || encoded === "json") return Buffer.from(content, "utf8");
  }
  throw decodeFailed(`a CSV slot holds octets or text, not a '${encoded}'-encoded ${kind(content)}`);
}

function kind(value) {
  if (value === undefined || value === null) return "nothing";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

/** A member path segment in the plan grammar's path syntax. */
function memberPath(name) {
  return /^[A-Za-z_-][A-Za-z0-9_-]*$/.test(name) ? `.${name}` : `['${name.replaceAll("'", "\\'")}']`;
}

function decodeFailed(message, line) {
  const details = { "content-type": MEDIA_TYPE };
  if (line !== undefined) details.line = line;
  return failure("decode-failed", "document", message, details);
}

function encodeFailed(message, path) {
  const details = { "content-type": MEDIA_TYPE };
  if (path !== undefined) details.path = path;
  return failure("encode-failed", "document", message, details);
}

// --- matcher ------------------------------------------------------------------------------------------

/** `matcher/apply` (spec §7.2): each value through the named action, with the core action's arguments. */
function apply(body) {
  const action = typeof body.action === "string" ? body.action : "";
  if (!ACTIONS.includes(action)) {
    throw failure("unknown-action", "document", `the csv component has no action '${action}'`, { action });
  }
  const values = Array.isArray(body.values) ? body.values : [];
  const args = Array.isArray(body.config?.arguments) ? body.config.arguments : [];
  return { results: values.map((value) => applyOne(action, value?.content, args)) };
}

function applyOne(action, value, args) {
  const text = typeof value === "string" ? value : undefined;
  let holds;
  let what;
  switch (action) {
    case "csv:integer":
      holds = text !== undefined && /^[+-]?\d+$/.test(text);
      what = "text that spells an integer";
      break;
    case "csv:number":
      holds = text !== undefined && spellsAFloat(text) && Number.isFinite(Number(text));
      what = "text that spells a number";
      break;
    case "csv:boolean":
      holds = text === "true" || text === "false";
      what = "text that spells a boolean";
      break;
    case OPERATOR: {
      let column;
      try {
        column = Column.of(args[0] ?? null);
      } catch (message) {
        return { status: "error", message };
      }
      if (column.action === undefined) {
        if (text !== undefined) return { status: "ok" };
        holds = false;
        what = "a CSV field";
        break;
      }
      return applyOne(column.action, value, []);
    }
    case "csv:equality": {
      const example = asField(args[0]);
      if (example === undefined) return { status: "error", message: `a CSV field cannot hold ${JSON.stringify(args[0] ?? null)}` };
      holds = text === example;
      what = `'${example}'`;
      break;
    }
  }
  return holds ? { status: "ok" } : { status: "error", message: `expected ${what}, got ${JSON.stringify(value ?? null)}` };
}

/** Text Rust's `f64` parser reads: digits with an optional point and exponent, or infinity or NaN. */
function spellsAFloat(text) {
  return /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(text) || /^[+-]?(inf|infinity|nan)$/i.test(text);
}

function asField(value) {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return undefined;
}

/** What a `csv:column` admits, by its `type`: each admits the ones below it, and `text` every field. */
const Column = {
  of(config) {
    const type = isObject(config) && typeof config.type === "string" ? config.type : "text";
    const action = { text: undefined, number: "csv:number", integer: "csv:integer", boolean: "csv:boolean" };
    if (!(type in action)) throw `a csv:column's type is text, integer, number or boolean, not '${type}'`;
    return { name: type, action: action[type] };
  },
  within(narrow, wide) {
    return narrow.name === wide.name || wide.name === "text" || (narrow.name === "integer" && wide.name === "number");
  },
};

/** `matcher/compare` (spec §7.5): whether every field the provider's shape admits, the consumer's does. */
function compare(body) {
  const operator = typeof body.operator === "string" ? body.operator : "";
  if (operator !== OPERATOR) {
    throw failure("unknown-operator", "document", `the csv component has no operator '${operator}'`, { operator });
  }
  if (!("provider" in body) || !("consumer" in body)) {
    throw failure("malformed-frame", "protocol", "matcher/compare needs 'provider' and 'consumer'");
  }
  const shape = (side) => (isObject(side) && typeof side.shape === "string" ? side.shape : "");
  const [p, c] = [shape(body.provider), shape(body.consumer)];
  const answer = (verdict, reason) => ({ verdict, reason });
  const column = (side) => {
    try {
      return { ok: Column.of(side) };
    } catch (reason) {
      return { reason };
    }
  };
  if (p === OPERATOR && c === OPERATOR) {
    const [provider, consumer] = [column(body.provider), column(body.consumer)];
    if (provider.reason ?? consumer.reason) return answer("unknown", provider.reason ?? consumer.reason);
    return Column.within(provider.ok, consumer.ok)
      ? answer("yes", `every field that spells ${provider.ok.name} spells ${consumer.ok.name}`)
      : answer("no", `a field that spells ${provider.ok.name} need not spell ${consumer.ok.name}`);
  }
  if (p === OPERATOR && (c === "string" || c === "any")) return answer("yes", "every CSV field is a string");
  if (p === "string" && c === OPERATOR) {
    const consumer = column(body.consumer);
    if (consumer.reason) return answer("unknown", consumer.reason);
    return consumer.ok.name === "text"
      ? answer("yes", "a text column admits every string")
      : answer("no", `a string need not spell ${consumer.ok.name}`);
  }
  return answer("unknown", `'${p}' against '${c}' depends on the content type of the slot, which compare is not told`);
}
