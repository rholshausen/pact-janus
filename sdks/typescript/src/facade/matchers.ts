// pact-js's `MatchersV3` (ADR 0066 decision 4): its matchers are data, the FFI's integration JSON,
// and they are returned here exactly as pact-js returns them, so `reify`, `extractPayload` and tests
// that read a matcher's `value` work unchanged. Nothing here matches anything: the engine converts
// the data when the interaction is added (`consumer-session/add-pact-interaction`).
//
// Ported from pact-js (MIT, Copyright (c) 2014 DiUS and the pact-js contributors) at 8a8f27a9:
// src/v3/matchers.ts, src/v3/types.ts and src/common/matcherFormats.ts. One difference: `url2` and
// `url` with a `RegExp` fragment are refused, since generating its example needs `randexp`; pass
// `regex(pattern, example)` for the fragment instead.

export const HTTPResponseStatusClass = {
  Information: "information",
  Success: "success",
  Redirect: "redirect",
  ClientError: "clientError",
  ServerError: "serverError",
  NonError: "nonError",
  Error: "error",
} as const;
export type HTTPResponseStatusClass = (typeof HTTPResponseStatusClass)[keyof typeof HTTPResponseStatusClass];

export interface Matcher<T> {
  "pact:matcher:type": string;
  "pact:generator:type"?: string;
  value?: T | Record<string, T>;
}

export interface V3RegexMatcher extends Matcher<string> {
  regex: string;
  example?: string;
}
export interface MinLikeMatcher<T> extends Matcher<T> {
  min: number;
}
export interface MaxLikeMatcher<T> extends Matcher<T> {
  max: number;
}
export interface DateTimeMatcher extends Matcher<string> {
  format: string;
}
export interface ArrayContainsMatcher extends Matcher<unknown[]> {
  variants: unknown[];
}
export interface ProviderStateInjectedValue<T> extends Matcher<T> {
  expression: string;
}
export interface StatusCodeMatcher<T> extends Matcher<T> {
  status: HTTPResponseStatusClass | number[];
}
export interface RulesMatcher<T> extends Matcher<T> {
  rules: Matcher<T>[];
}

export type AnyJson = boolean | number | string | null | AnyJson[] | { [key: string]: AnyJson };

const EMAIL_FORMAT = "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+.[a-zA-Z]{2,}$";
const ISO8601_DATE_FORMAT =
  "^([\\+-]?\\d{4}(?!\\d{2}\\b))((-?)((0[1-9]|1[0-2])(\\3([12]\\d|0[1-9]|3[01]))?|W([0-4]\\d|5[0-2])(-?[1-7])?|(00[1-9]|0[1-9]\\d|[12]\\d{2}|3([0-5]\\d|6[1-6])))?)$";
const ISO8601_DATETIME_FORMAT = "^\\d{4}-[01]\\d-[0-3]\\dT[0-2]\\d:[0-5]\\d:[0-5]\\d([+-][0-2]\\d:[0-5]\\d|Z)$";
const ISO8601_DATETIME_WITH_MILLIS_FORMAT =
  "^\\d{4}-[01]\\d-[0-3]\\dT[0-2]\\d:[0-5]\\d:[0-5]\\d\\.\\d+([+-][0-2]\\d(:?[0-5]\\d)?|Z)$";
const ISO8601_TIME_FORMAT = "^(T\\d\\d:\\d\\d(:\\d\\d)?(\\.\\d+)?(([+-]\\d\\d:\\d\\d)|Z)?)?$";
const RFC1123_TIMESTAMP_FORMAT =
  "^(Mon|Tue|Wed|Thu|Fri|Sat|Sun),\\s\\d{2}\\s(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\\s\\d{4}\\s\\d{2}:\\d{2}:\\d{2}\\s(\\+|-)\\d{4}$";
const IPV4_FORMAT = "^(\\d{1,3}\\.)+\\d{1,3}$";
const IPV6_FORMAT =
  "^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$";
const HEX_FORMAT = "^[0-9a-fA-F]+$";

const times = <T>(make: () => T, count: number): T[] => Array.from({ length: count }, make);

export function isMatcher(x: unknown): x is Matcher<unknown> {
  return x != null && (x as Matcher<unknown>)["pact:matcher:type"] !== undefined && (x as Matcher<unknown>).value !== undefined;
}

export const like = <T>(template: T): Matcher<T> => ({ "pact:matcher:type": "type", value: template });

export const eachKeyLike = <T>(keyTemplate: string, template: T): Matcher<T> => ({
  "pact:matcher:type": "values",
  value: { [keyTemplate]: template },
});

export const eachKeyMatches = (
  example: Record<string, unknown>,
  matchers: Matcher<string> | Matcher<string>[] = like("key"),
): RulesMatcher<unknown> => ({
  "pact:matcher:type": "eachKey",
  rules: Array.isArray(matchers) ? matchers : [matchers],
  value: example,
});

export const eachValueMatches = <T>(example: Record<string, T>, matchers: Matcher<T> | Matcher<T>[]): RulesMatcher<T> => ({
  "pact:matcher:type": "eachValue",
  rules: Array.isArray(matchers) ? matchers : [matchers],
  value: example,
});

export const matchStatus = (example: number, status: HTTPResponseStatusClass | number[]): StatusCodeMatcher<number> => ({
  "pact:matcher:type": "statusCode",
  status,
  value: example,
});

export const eachLike = <T>(template: T, min = 1): MinLikeMatcher<T[]> => ({
  min,
  "pact:matcher:type": "type",
  value: times(() => template, min),
});

export const atLeastOneLike = <T>(template: T, count = 1): MinLikeMatcher<T[]> => ({
  min: 1,
  "pact:matcher:type": "type",
  value: times(() => template, count),
});

export const atLeastLike = <T>(template: T, min: number, count?: number): MinLikeMatcher<T[]> => {
  if (count && count < min) {
    throw new Error(
      `atLeastLike has a minimum of ${min} but ${count} elements were requested. Make sure the count is greater than or equal to the min.`,
    );
  }
  return { min, "pact:matcher:type": "type", value: times(() => template, count || min) };
};

export const atMostLike = <T>(template: T, max: number, count?: number): MaxLikeMatcher<T[]> => {
  if (count && count > max) {
    throw new Error(
      `atMostLike has a maximum of ${max} but ${count} elements where requested. Make sure the count is less than or equal to the max.`,
    );
  }
  return { max, "pact:matcher:type": "type", value: times(() => template, count || 1) };
};

export const constrainedArrayLike = <T>(
  template: T,
  min: number,
  max: number,
  count?: number,
): MinLikeMatcher<T[]> & MaxLikeMatcher<T[]> => {
  if (count && count < min) {
    throw new Error(
      `constrainedArrayLike has a minimum of ${min} but ${count} elements where requested. Make sure the count is greater than or equal to the min.`,
    );
  }
  if (count && count > max) {
    throw new Error(
      `constrainedArrayLike has a maximum of ${max} but ${count} elements where requested. Make sure the count is less than or equal to the max.`,
    );
  }
  return { min, max, "pact:matcher:type": "type", value: times(() => template, count || min) };
};

export const boolean = (b = true): Matcher<boolean> => ({ "pact:matcher:type": "type", value: b });

export const integer = (int?: number): Matcher<number> => {
  if (Number.isInteger(int)) {
    return { "pact:matcher:type": "integer", value: int };
  }
  if (int) {
    throw new Error(`The integer matcher was passed '${int}' which is not an integer.`);
  }
  return { "pact:generator:type": "RandomInt", "pact:matcher:type": "integer", value: 101 };
};

export const decimal = (num?: number): Matcher<number> => {
  if (Number.isFinite(num)) {
    return { "pact:matcher:type": "decimal", value: num };
  }
  if (num) {
    throw new Error(`The decimal matcher was passed '${num}' which is not a number.`);
  }
  return { "pact:generator:type": "RandomDecimal", "pact:matcher:type": "decimal", value: 12.34 };
};

export function number(num?: number): Matcher<number> {
  if (typeof num === "number") {
    return { "pact:matcher:type": "number", value: num };
  }
  if (num) {
    throw new Error(`The number matcher was passed '${String(num)}' which is not a number.`);
  }
  return { "pact:generator:type": "RandomInt", "pact:matcher:type": "number", value: 1234 };
}

export function string(str = "some string"): Matcher<string> {
  return { "pact:matcher:type": "type", value: str };
}

export function regex(pattern: RegExp | string, str: string): V3RegexMatcher {
  return { "pact:matcher:type": "regex", regex: pattern instanceof RegExp ? pattern.source : pattern, value: str };
}

const validatedRegex = (pattern: string, defaultExample: string, example?: string): V3RegexMatcher => {
  const value = example || defaultExample;
  // Escaped backslashes are sent over the wire as JSON.
  if (!new RegExp(pattern.replace("\\\\", "\\")).test(value)) {
    throw new Error(`Example '${value}' does not match regular expression '${pattern}'`);
  }
  return regex(pattern, value);
};

export const email = (address?: string): V3RegexMatcher => validatedRegex(EMAIL_FORMAT, "hello@pact.io", address);
export const ipv4Address = (ip?: string): V3RegexMatcher => validatedRegex(IPV4_FORMAT, "127.0.0.13", ip);
export const ipv6Address = (ip?: string): V3RegexMatcher => validatedRegex(IPV6_FORMAT, "::ffff:192.0.2.128", ip);
export const iso8601DateTime = (value?: string): V3RegexMatcher =>
  validatedRegex(ISO8601_DATETIME_FORMAT, "2015-08-06T16:53:10+01:00", value);
export const iso8601DateTimeWithMillis = (value?: string): V3RegexMatcher =>
  validatedRegex(ISO8601_DATETIME_WITH_MILLIS_FORMAT, "2015-08-06T16:53:10.123+01:00", value);
export const iso8601Date = (value?: string): V3RegexMatcher => validatedRegex(ISO8601_DATE_FORMAT, "2013-02-01", value);
export const iso8601Time = (value?: string): V3RegexMatcher => validatedRegex(ISO8601_TIME_FORMAT, "T22:44:30.652Z", value);
export const rfc1123Timestamp = (value?: string): V3RegexMatcher =>
  validatedRegex(RFC1123_TIMESTAMP_FORMAT, "Mon, 31 Oct 2016 15:21:41 -0400", value);
export const hexadecimal = (value?: string): V3RegexMatcher => validatedRegex(HEX_FORMAT, "3F", value);

export const contentType = (contentTypeValue: string): Matcher<string> => ({
  "pact:matcher:type": "contentType",
  value: contentTypeValue,
});

export const equal = <T>(value: T): Matcher<T> => ({ "pact:matcher:type": "equality", value });

function dated(kind: string, format: string, example: string): DateTimeMatcher {
  if (!example) {
    throw new Error(`you must provide an example ${kind}`);
  }
  return { "pact:matcher:type": kind === "datetime" || kind === "timestamp" ? "timestamp" : kind, format, value: example };
}

export const datetime = (format: string, example: string): DateTimeMatcher => dated("datetime", format, example);
export const timestamp = (format: string, example: string): DateTimeMatcher => dated("timestamp", format, example);
export const time = (format: string, example: string): DateTimeMatcher => dated("time", format, example);
export const date = (format: string, example: string): DateTimeMatcher => dated("date", format, example);

export function includes(value: string): Matcher<string> {
  return { "pact:matcher:type": "include", value };
}

export function nullValue(): Matcher<null> {
  return { "pact:matcher:type": "null", value: null };
}

export function url2(basePath: string | null, pathFragments: (string | V3RegexMatcher | RegExp)[]): V3RegexMatcher {
  const fragments = pathFragments.map((p) => {
    if (p instanceof RegExp) {
      throw new TypeError(
        `url(): the fragment ${String(p)} needs an example, which pact-js generated with randexp; pass regex(${String(p)}, "<example>") instead`,
      );
    }
    return typeof p === "object" && p["pact:matcher:type"] === "regex" ? { pattern: p.regex, example: p.value ?? "" } : { pattern: String(p), example: String(p) };
  });
  const regexpr = [".*(", ...fragments.map((f) => `\\/${f.pattern}`)].join("");
  const example = [basePath || "http://localhost:8080", ...fragments.map((f) => `/${f.example}`)].join("");
  if (basePath == null) {
    return { "pact:matcher:type": "regex", "pact:generator:type": "MockServerURL", regex: `${regexpr})$`, value: example, example };
  }
  return { "pact:matcher:type": "regex", regex: `${regexpr})$`, value: example };
}

export const url = (pathFragments: (string | V3RegexMatcher | RegExp)[]): V3RegexMatcher => url2(null, pathFragments);

export function arrayContaining(...variants: unknown[]): ArrayContainsMatcher {
  return { "pact:matcher:type": "arrayContains", variants };
}

export function fromProviderState<V>(expression: string, exampleValue: V): ProviderStateInjectedValue<V> {
  return { "pact:matcher:type": "type", "pact:generator:type": "ProviderState", expression, value: exampleValue };
}

export function uuid(example?: string): V3RegexMatcher {
  const regexStr = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
  if (example) {
    if (!new RegExp(`^${regexStr}$`).test(example)) {
      throw new Error(`regex: Example value '${example}' does not match the UUID regular expression '${regexStr}'`);
    }
    return { "pact:matcher:type": "regex", regex: regexStr, value: example };
  }
  return { "pact:matcher:type": "regex", regex: regexStr, "pact:generator:type": "Uuid", value: "e2490de5-5bd3-43d5-b7c4-526e33f71304" };
}

export const matcherValueOrString = (obj: unknown): string => (typeof obj === "string" ? obj : JSON.stringify(obj));

export const isStatusCodeMatcher = (status: number | StatusCodeMatcher<number>): status is StatusCodeMatcher<number> =>
  isMatcher(status) && status["pact:matcher:type"] === "statusCode";

/** The example a template stands for: every matcher replaced by its `value`. */
export function reify<T extends AnyJson = AnyJson>(input: unknown): T {
  if (isMatcher(input)) {
    return reify<T>(input.value);
  }
  if (Array.isArray(input)) {
    return input.map((item) => reify(item)) as unknown as T;
  }
  if (input === null || typeof input === "number" || typeof input === "string" || typeof input === "boolean") {
    return input as unknown as T;
  }
  if (typeof input === "object") {
    return Object.fromEntries(Object.entries(input).map(([name, value]) => [name, reify(value)])) as T;
  }
  throw new Error(`Unable to strip matcher from a '${typeof input}', as it is not valid in a Pact description`);
}

export { reify as extractPayload };
