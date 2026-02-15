// Extractor entry point — run with: bun run ./extractor.ts <schema> <func1> <func2> ...
//
// 1. Registers mock modules via build.module() to intercept Convex server imports
//    NOTE: convex/values is NOT mocked — real Convex validators are used so that
//    .omit(), .extend(), .pick(), .partial() etc. work natively.
// 2. Dynamically imports the schema file (populates __schema via defineSchema mock)
// 3. Dynamically imports each function file (exports tagged with __type)
// 4. Prints the combined result as JSON to stdout

import { plugin } from "bun";
import type { FunctionDef } from "./mocks/convex_server.ts";
import * as convexServer from "./mocks/convex_server.ts";
import * as convexApi from "./mocks/convex_api.ts";
import { normalize } from "./mocks/normalize.ts";

type Descriptor = Record<string, unknown>;

// ---------------------------------------------------------------------------
// 1. Register mock modules — intercepts `import "convex/server"` etc.
//    Must happen before any dynamic import() of user code.
//
//    convex/values is NOT mocked — real validators are used and normalized
//    at extraction time via normalize().
// ---------------------------------------------------------------------------

plugin({
  name: "convex-typegen-mock",
  setup(build) {
    // Mock convex/server (defineSchema, defineTable, query, mutation, etc.)
    build.module("convex/server", () => ({
      exports: convexServer,
      loader: "object",
    }));

    // _generated imports → stubs (need onResolve for pattern matching)
    const MOCK_DIR = import.meta.dir + "/mocks";

    build.onResolve({ filter: /\/_generated\/api/ }, () => ({
      path: MOCK_DIR + "/convex_api.ts",
    }));

    build.onResolve({ filter: /\/_generated\/server/ }, () => ({
      path: MOCK_DIR + "/convex_server.ts",
    }));

    build.onResolve({ filter: /\/_generated\/dataModel/ }, () => ({
      path: MOCK_DIR + "/convex_api.ts",
    }));

    // User-supplied helper stubs (passed via TYPEGEN_HELPER_STUBS env var)
    // Format: JSON object mapping regex patterns to absolute file paths
    const raw = process.env.TYPEGEN_HELPER_STUBS;
    if (raw) {
      const helperStubs: Record<string, string> = JSON.parse(raw);
      for (const [pattern, stubPath] of Object.entries(helperStubs)) {
        build.onResolve({ filter: new RegExp(pattern) }, () => ({
          path: stubPath,
        }));
      }
    }
  },
});

// ---------------------------------------------------------------------------
// 2. Import schema — side-effect: populates __schema via defineSchema()
// ---------------------------------------------------------------------------

const [schemaPath, ...functionPaths] = process.argv.slice(2);

if (!schemaPath) {
  console.error("Usage: bun run extractor.ts <schema.ts> [func1.ts ...]");
  process.exit(1);
}

await import(schemaPath);

// ---------------------------------------------------------------------------
// 3. Import each function file and extract registered functions
// ---------------------------------------------------------------------------

interface FunctionRecord {
  name: string;
  type: string;
  params: Array<{ name: string; data_type: Descriptor }>;
  return_type: Descriptor | null;
  file_name: string;
}

const functions: FunctionRecord[] = [];

for (const fp of functionPaths) {
  const parts = fp.split("/");
  const rawName = parts[parts.length - 1] ?? fp;
  const fileName = rawName.replace(/\.ts$/, "");

  const mod = await import(fp);

  for (const [exportName, value] of Object.entries(mod)) {
    if (
      value !== null &&
      typeof value === "object" &&
      "__type" in (value as object)
    ) {
      const def = value as FunctionDef;
      const config = def.__config ?? {};

      // Extract and normalize params from args.
      // Since we use real convex/values, args is a real Convex validator
      // (v.object({ ... })) — normalize() converts it to codegen format.
      const argsRaw = config.args;
      let params: Array<{ name: string; data_type: Descriptor }> = [];

      if (argsRaw !== undefined && argsRaw !== null) {
        const normalized = normalize(argsRaw);
        // After normalization, should be { type: "object", properties: { ... } }
        if (
          normalized.type === "object" &&
          normalized.properties &&
          typeof normalized.properties === "object"
        ) {
          params = Object.entries(
            normalized.properties as Record<string, Descriptor>,
          ).map(([paramName, dt]) => ({
            name: paramName,
            data_type: dt,
          }));
        }
      }

      // Normalize return type if present
      const returnsRaw = config.returns;
      const returnType =
        returnsRaw !== undefined && returnsRaw !== null
          ? normalize(returnsRaw)
          : null;

      functions.push({
        name: exportName,
        type: def.__type,
        params,
        return_type: returnType,
        file_name: fileName,
      });
    }
  }
}

// ---------------------------------------------------------------------------
// 4. Print JSON to stdout — Rust extract.rs reads this
// ---------------------------------------------------------------------------

const output = JSON.stringify({ schema: convexServer.__schema, functions });
console.log(output);
