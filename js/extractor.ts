// Extractor entry point — run with: bun run ./extractor.ts <schema> <func1> <func2> ...
//
// 1. Registers mock modules via build.module() to intercept Convex imports
// 2. Dynamically imports the schema file (populates __schema via defineSchema mock)
// 3. Dynamically imports each function file (exports tagged with __type)
// 4. Prints the combined result as JSON to stdout

import { plugin } from "bun";
import type { FunctionDef } from "./mocks/convex_server.ts";
import * as convexServer from "./mocks/convex_server.ts";
import * as convexValues from "./mocks/convex_values.ts";
import * as convexApi from "./mocks/convex_api.ts";

type Descriptor = Record<string, unknown>;

// ---------------------------------------------------------------------------
// 1. Register mock modules — intercepts `import "convex/values"` etc.
//    Must happen before any dynamic import() of user code.
// ---------------------------------------------------------------------------

plugin({
  name: "convex-typegen-mock",
  setup(build) {
    // Core Convex packages → our mock module instances
    build.module("convex/values", () => ({
      exports: convexValues,
      loader: "object",
    }));

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

      // Extract params from args descriptor.
      // args can be either:
      //   1. A raw record: { paramName: v.string(), ... }
      //   2. A v.object() descriptor: { type: "object", properties: { paramName: ... } }
      const args = config.args ?? {};
      const argsProperties: Record<string, Descriptor> =
        (args as Descriptor).type === "object" && (args as Descriptor).properties
          ? ((args as Descriptor).properties as Record<string, Descriptor>)
          : (args as Record<string, Descriptor>);
      const params = Object.entries(argsProperties).map(
        ([paramName, dt]) => ({
          name: paramName,
          data_type: dt,
        }),
      );

      functions.push({
        name: exportName,
        type: def.__type,
        params,
        return_type: (config.returns as Descriptor) ?? null,
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
