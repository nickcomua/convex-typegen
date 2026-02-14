use std::fs;

use convex_typegen::{generate, Configuration};
use tempdir::TempDir;

fn setup_test_dir() -> TempDir
{
    let temp_dir = TempDir::new("convex_typegen_test").expect("Failed to create temp directory");

    // Create _generated/ stubs so function files can import from "./_generated/server"
    let generated_dir = temp_dir.path().join("_generated");
    fs::create_dir_all(&generated_dir).expect("Failed to create _generated dir");
    fs::write(
        generated_dir.join("server.ts"),
        r#"export { query, mutation, action, internalQuery, internalMutation, internalAction, httpAction } from "convex/server";"#,
    )
    .expect("Failed to write _generated/server stub");

    temp_dir
}

#[test]
fn test_valid_function()
{
    let temp_dir = setup_test_dir();

    // Create an empty schema file first
    let schema_path = temp_dir.path().join("schema.ts");
    fs::write(
        &schema_path,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    test: defineTable({
        name: v.string(),
    }),
})
"#,
    )
    .unwrap();

    let function_path = temp_dir.path().join("valid_function.ts");
    fs::write(
        &function_path,
        r#"
import { query } from "./_generated/server";

export const testQuery = query({
    args: {},
    handler: async (ctx, args) => {},
});
    "#,
    )
    .unwrap();

    let config = Configuration {
        schema_path,
        function_paths: vec![function_path],
        out_file: temp_dir.path().join("types.rs").to_string_lossy().to_string(),
        helper_stubs: std::collections::HashMap::new(),
    };

    let result = generate(config);
    assert!(result.is_ok(), "Expected Ok result, got {:?}", result);
}

#[test]
fn test_plain_function_is_ignored()
{
    let temp_dir = setup_test_dir();

    let schema_path = temp_dir.path().join("schema.ts");
    fs::write(
        &schema_path,
        r#"
import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
    test: defineTable({ name: v.string() }),
})
"#,
    )
    .unwrap();

    // A plain function (not wrapped in query/mutation) should be silently ignored
    let function_path = temp_dir.path().join("plain_function.ts");
    fs::write(
        &function_path,
        r#"
        export default async function plainFunction() {
            // Not a Convex query/mutation — should be ignored
        }
    "#,
    )
    .unwrap();

    let config = Configuration {
        schema_path,
        function_paths: vec![function_path],
        out_file: temp_dir.path().join("types.rs").to_string_lossy().to_string(),
        helper_stubs: std::collections::HashMap::new(),
    };

    // Should succeed — plain exports without __type are silently skipped
    let result = generate(config);
    assert!(result.is_ok(), "Expected Ok result, got {:?}", result);
}
