// Mock implementation of `convex/values` — every v.* function returns a JSON
// descriptor that matches what codegen.rs expects in data_type fields.

type Descriptor = Record<string, unknown>;

const v = {
  // Primitives
  string: (): Descriptor => ({ type: "string" }),
  number: (): Descriptor => ({ type: "number" }),
  boolean: (): Descriptor => ({ type: "boolean" }),
  int64: (): Descriptor => ({ type: "int64" }),
  float64: (): Descriptor => ({ type: "float64" }),
  null: (): Descriptor => ({ type: "null" }),
  null_: (): Descriptor => ({ type: "null" }),  // alias for compatibility
  any: (): Descriptor => ({ type: "any" }),
  bytes: (): Descriptor => ({ type: "bytes" }),

  // Reference to another table
  id: (tableName: string): Descriptor => ({ type: "id", tableName }),

  // Container types — key names must match what codegen.rs reads:
  //   optional → "inner"
  //   array    → "elements"
  //   object   → "properties" (map of fieldName → type descriptor)
  //   record   → "keyType" + "valueType"
  //   union    → "variants" (array of type descriptors)
  optional: (inner: Descriptor): Descriptor => ({ type: "optional", inner }),

  array: (elements: Descriptor): Descriptor => ({ type: "array", elements }),

  object: (fields: Record<string, Descriptor>): Descriptor => ({
    type: "object",
    properties: fields,
  }),

  record: (keyType: Descriptor, valueType: Descriptor): Descriptor => ({
    type: "record",
    keyType,
    valueType,
  }),

  union: (...variants: Descriptor[]): Descriptor => ({
    type: "union",
    variants,
  }),

  literal: (value: string | number | boolean): Descriptor => ({
    type: "literal",
    value,
  }),
};

export { v };
export default v;
