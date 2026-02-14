// Mock implementation of `convex/server` — captures schema and function
// definitions so the extractor can serialize them to JSON.

type Descriptor = Record<string, unknown>;

// ---------------------------------------------------------------------------
// Schema collection
// ---------------------------------------------------------------------------

export interface TableDef {
  name: string;
  columns: Array<{ name: string; data_type: Descriptor }>;
}

export const __schema: { tables: TableDef[] } = { tables: [] };

// Table builder — supports chainable .index() / .searchIndex() (ignored)
interface TableBuilder {
  _validator: Descriptor;
  index: (...args: unknown[]) => TableBuilder;
  searchIndex: (...args: unknown[]) => TableBuilder;
}

export function defineTable(validator: Descriptor): TableBuilder {
  const builder: TableBuilder = {
    _validator: validator,
    index: () => builder,
    searchIndex: () => builder,
  };
  return builder;
}

export function defineSchema(
  tables: Record<string, TableBuilder>,
  _options?: unknown,
): typeof __schema {
  for (const [name, table] of Object.entries(tables)) {
    // defineTable() can receive either:
    //   1. A raw record of fields: defineTable({ name: v.string(), ... })
    //      → _validator = { name: { type: "string" }, ... }
    //   2. A v.object() descriptor: defineTable(v.object({ name: v.string() }))
    //      → _validator = { type: "object", properties: { name: { type: "string" } } }
    const validator = table._validator ?? {};
    const properties: Record<string, Descriptor> =
      validator.type === "object" && validator.properties
        ? (validator.properties as Record<string, Descriptor>)
        : (validator as Record<string, Descriptor>);
    const columns = Object.entries(properties).map(([fieldName, dt]) => ({
      name: fieldName,
      data_type: dt,
    }));
    __schema.tables.push({ name, columns });
  }
  return __schema;
}

// ---------------------------------------------------------------------------
// Function registration — captures config (args, returns), ignores handler
// ---------------------------------------------------------------------------

export interface FunctionDef {
  __type: string;
  __config: {
    args?: Descriptor;
    returns?: Descriptor;
    handler?: unknown;
  };
}

function makeFunctionRegistrar(type: string) {
  return (config: Record<string, unknown>): FunctionDef => ({
    __type: type,
    __config: config as FunctionDef["__config"],
  });
}

export const query = makeFunctionRegistrar("query");
export const mutation = makeFunctionRegistrar("mutation");
export const action = makeFunctionRegistrar("action");
export const internalQuery = makeFunctionRegistrar("internalQuery");
export const internalMutation = makeFunctionRegistrar("internalMutation");
export const internalAction = makeFunctionRegistrar("internalAction");
export const httpAction = makeFunctionRegistrar("httpAction");

// _generated/server.js imports these *Generic variants and re-exports them.
// We alias them so the generated file works without modification.
export const queryGeneric = query;
export const mutationGeneric = mutation;
export const actionGeneric = action;
export const internalQueryGeneric = internalQuery;
export const internalMutationGeneric = internalMutation;
export const internalActionGeneric = internalAction;
export const httpActionGeneric = httpAction;

// _generated/server.js also imports builder classes — stub them as no-ops
export class QueryBuilder {}
export class MutationBuilder {}
export class ActionBuilder {}
export class HttpActionBuilder {}

// _generated/api.js imports these from convex/server
const proxyHandler: ProxyHandler<object> = {
  get: (_target, _prop) => new Proxy({}, proxyHandler),
};
export const anyApi = new Proxy({}, proxyHandler);
export const componentsGeneric = () => ({});

// messages.ts and other files may use paginationOptsValidator from convex/server
export const paginationOptsValidator = {
  type: "object",
  properties: {
    numItems: { type: "number" },
    cursor: { type: "union", variants: [{ type: "string" }, { type: "null" }] },
  },
};
