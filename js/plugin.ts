// Bun preload plugin — intercepts Convex package imports and redirects them
// to our mock implementations.

import { plugin } from "bun";

const MOCK_DIR = import.meta.dir + "/mocks";

plugin({
  name: "convex-typegen-mock",
  setup(build) {
    // Core Convex packages → our mocks
    build.onResolve({ filter: /^convex\/values$/ }, () => ({
      path: MOCK_DIR + "/convex_values.ts",
    }));

    build.onResolve({ filter: /^convex\/server$/ }, () => ({
      path: MOCK_DIR + "/convex_server.ts",
    }));

    // _generated imports → stubs
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
