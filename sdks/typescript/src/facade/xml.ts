// pact-js's `XmlBuilder` (ADR 0066 decision 4): a document of elements, text and matchers, whose JSON
// the facade hands the engine as an XML body, as pact-js handed it to the FFI. The engine builds the
// XML and its rules (`consumer-session/add-pact-interaction`'s `with_body`).
//
// Ported from pact-js (MIT, Copyright (c) 2014 DiUS and the pact-js contributors) at 8a8f27a9:
// src/xml/xmlBuilder.ts, xmlElement.ts, xmlNode.ts and xmlText.ts. Attributes are a `Map`, as
// there, so they stringify as `{}` and an element's attributes are lost, as they are in the pact
// pact-js writes; an object of attributes is kept.

import { isMatcher, type Matcher } from "./matchers.js";

export type XmlAttributes = Map<string, string>;
export type XmlCallback = (n: XmlElement) => void;

export class XmlText {
  content: string;
  matcher?: Matcher<string>;

  constructor(content: string, matcher?: Matcher<string>) {
    this.content = content;
    this.matcher = matcher;
  }
}

interface EachLikeOptions {
  min?: number;
  max?: number;
  examples?: number;
}

/** What an element holds: elements, text, and matchers over elements. pact-js's is an empty class. */
export type XmlNode = XmlElement | XmlText;

export class XmlElement {
  name: string;
  children: (XmlNode | Matcher<unknown>)[] = [];
  attributes?: XmlAttributes;

  constructor(name: string) {
    this.name = name;
  }

  setName(name: string): XmlElement {
    this.name = name;
    return this;
  }

  setAttributes(attributes: XmlAttributes): XmlElement {
    this.attributes = attributes;
    return this;
  }

  appendElement(name: string, attributes: XmlAttributes, arg?: string | XmlCallback | Matcher<string>): XmlElement {
    const el = new XmlElement(name).setAttributes(attributes);
    if (arg) {
      if (typeof arg === "function") {
        arg(el);
      } else {
        el.appendText(arg);
      }
    }
    this.children.push(el);
    return this;
  }

  appendText(content: string | Matcher<string>): XmlElement {
    if (typeof content === "object" && content["pact:matcher:type"]) {
      const example = isMatcher(content) && typeof content.value === "string" ? content.value : "";
      this.children.push(new XmlText(example, content));
    } else {
      this.children.push(new XmlText(content.toString()));
    }
    return this;
  }

  eachLike(name: string, attributes: XmlAttributes, cb?: XmlCallback, options: EachLikeOptions = { examples: 1 }): XmlElement {
    const el = new XmlElement(name).setAttributes(attributes);
    cb?.(el);
    this.children.push({ "pact:matcher:type": "type", value: el, examples: options.examples } as Matcher<unknown>);
    return this;
  }
}

export class XmlBuilder {
  private root: XmlElement;

  constructor(_version: string, _charset: string, rootElement: string) {
    this.root = new XmlElement(rootElement);
  }

  /** The document's JSON, which `xmlBody` passes as the body. */
  build(callback: (doc: XmlElement) => void): string {
    callback(this.root);
    return JSON.stringify(this);
  }
}
