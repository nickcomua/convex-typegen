use std::fs;
use std::path::PathBuf;

use convex_typegen::{generate, Configuration};
use tempdir::TempDir;

/// Set up a test environment with a schema file and optional function files.
///
/// Returns the temp dir (keep alive), schema path, output path, and function paths.
fn setup_test_env(
    schema_content: &str,
    function_files: Option<Vec<(&str, &str)>>,
) -> (TempDir, PathBuf, PathBuf, Vec<PathBuf>)
{
    let temp_dir = TempDir::new("convex_codegen_test").expect("Failed to create temp directory");
    let schema_path = temp_dir.path().join("schema.ts");
    let output_path = temp_dir.path().join("types.rs");

    fs::write(&schema_path, schema_content).expect("Failed to write test schema");

    let mut function_paths = Vec::new();
    if let Some(files) = function_files {
        for (content, filename) in files {
            let fn_path = temp_dir.path().join(filename);
            fs::write(&fn_path, content).expect("Failed to write function file");
            function_paths.push(fn_path);
        }
    }

    (temp_dir, schema_path, output_path, function_paths)
}

/// Generate code and return the output string.
fn generate_and_read(schema_content: &str, function_files: Option<Vec<(&str, &str)>>) -> String
{
    let (_temp_dir, schema_path, output_path, function_paths) = setup_test_env(schema_content, function_files);
    let config = Configuration {
        schema_path,
        out_file: output_path.to_string_lossy().to_string(),
        function_paths,
    };
    generate(config).expect("Code generation failed");
    fs::read_to_string(output_path).expect("Failed to read generated code")
}

// =============================================================================
// Basic types
// =============================================================================

#[test]
fn test_basic_types()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            users: defineTable({
                name: v.string(),
                age: v.number(),
                isActive: v.boolean(),
                tags: v.array(v.string()),
                metadata: v.object({
                    createdAt: v.number(),
                    updatedAt: v.number(),
                }),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct UsersTable"), "missing UsersTable struct");
    assert!(code.contains("pub name: String"), "missing name field");
    assert!(code.contains("pub age: f64"), "missing age field");
    assert!(code.contains("pub is_active: bool"), "missing is_active field");
    assert!(code.contains("pub tags: Vec<String>"), "missing tags field");
    assert!(code.contains("pub metadata: UsersMetadata"), "missing metadata typed field");
    assert!(code.contains("pub struct UsersMetadata"), "missing UsersMetadata struct");
    assert!(code.contains("pub created_at: f64"), "missing created_at in UsersMetadata");
    assert!(code.contains("pub updated_at: f64"), "missing updated_at in UsersMetadata");
}

// =============================================================================
// Object types
// =============================================================================

#[test]
fn test_nested_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            users: defineTable({
                profile: v.object({
                    name: v.string(),
                    address: v.object({
                        city: v.string(),
                        zip: v.string(),
                    }),
                }),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct UsersProfile"), "missing UsersProfile struct");
    assert!(
        code.contains("pub struct UsersProfileAddress"),
        "missing nested UsersProfileAddress struct"
    );
    assert!(
        code.contains("pub address: UsersProfileAddress"),
        "profile should reference UsersProfileAddress"
    );
    assert!(code.contains("pub city: String"), "missing city field");
    assert!(code.contains("pub zip: String"), "missing zip field");
}

#[test]
fn test_mixed_type_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                meta: v.object({
                    name: v.string(),
                    count: v.number(),
                    active: v.boolean(),
                }),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct ItemsMeta"), "missing ItemsMeta struct");
    assert!(code.contains("pub name: String"), "missing name (String)");
    assert!(code.contains("pub count: f64"), "missing count (f64)");
    assert!(code.contains("pub active: bool"), "missing active (bool)");
    // Should NOT be a BTreeMap
    assert!(
        !code.contains("BTreeMap<String, f64>"),
        "should not use BTreeMap for mixed-type objects"
    );
}

#[test]
fn test_empty_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                data: v.object({}),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub data: serde_json::Value"),
        "empty object should be serde_json::Value"
    );
}

#[test]
fn test_array_of_objects()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            orders: defineTable({
                items: v.array(v.object({
                    name: v.string(),
                    qty: v.number(),
                })),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct OrdersItems"), "missing OrdersItems struct");
    assert!(code.contains("pub items: Vec<OrdersItems>"), "should be Vec<OrdersItems>");
}

#[test]
fn test_optional_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            users: defineTable({
                settings: v.optional(v.object({
                    theme: v.string(),
                    lang: v.string(),
                })),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct UsersSettings"), "missing UsersSettings struct");
    assert!(
        code.contains("pub settings: Option<UsersSettings>"),
        "should be Option<UsersSettings>"
    );
}

#[test]
fn test_deeply_nested_objects()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            docs: defineTable({
                level1: v.object({
                    level2: v.object({
                        level3: v.object({
                            value: v.string(),
                        }),
                    }),
                }),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub struct DocsLevel1"), "missing DocsLevel1");
    assert!(code.contains("pub struct DocsLevel1Level2"), "missing DocsLevel1Level2");
    assert!(
        code.contains("pub struct DocsLevel1Level2Level3"),
        "missing DocsLevel1Level2Level3"
    );
    assert!(
        code.contains("pub level2: DocsLevel1Level2"),
        "level1 should reference Level2"
    );
    assert!(
        code.contains("pub level3: DocsLevel1Level2Level3"),
        "level2 should reference Level3"
    );
}

#[test]
fn test_object_in_array_in_optional()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            events: defineTable({
                participants: v.optional(v.array(v.object({
                    name: v.string(),
                    role: v.string(),
                }))),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub struct EventsParticipants"),
        "missing EventsParticipants struct"
    );
    assert!(
        code.contains("pub participants: Option<Vec<EventsParticipants>>"),
        "should be Option<Vec<EventsParticipants>>"
    );
}

// =============================================================================
// Union types
// =============================================================================

#[test]
fn test_literal_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            posts: defineTable({
                status: v.union(
                    v.literal("draft"),
                    v.literal("published"),
                    v.literal("archived"),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum PostsStatus"), "missing PostsStatus enum");
    assert!(code.contains("Draft"), "missing Draft variant");
    assert!(code.contains("Published"), "missing Published variant");
    assert!(code.contains("Archived"), "missing Archived variant");
    // Literal unions are Copy
    assert!(code.contains("Copy"), "literal enum should derive Copy");
}

#[test]
fn test_tagged_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            events: defineTable({
                action: v.union(
                    v.object({ type: v.literal("click"), x: v.number(), y: v.number() }),
                    v.object({ type: v.literal("scroll"), delta: v.number() }),
                    v.object({ type: v.literal("keypress") }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum EventsAction"), "missing EventsAction enum");
    assert!(
        code.contains("#[serde(tag = \"type\")]"),
        "tagged union should have serde tag"
    );
    assert!(code.contains("Click {"), "missing Click variant");
    assert!(code.contains("Scroll {"), "missing Scroll variant");
    assert!(code.contains("Keypress"), "missing Keypress variant");
    assert!(code.contains("x: f64"), "missing x field in Click");
    assert!(code.contains("delta: f64"), "missing delta field in Scroll");
}

#[test]
fn test_nullable_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                description: v.union(v.string(), v.null()),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub description: Option<String>"),
        "union(string, null) should be Option<String>"
    );
}

// =============================================================================
// Record type
// =============================================================================

#[test]
fn test_record_type()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            scores: defineTable({
                playerScores: v.record(v.string(), v.number()),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub player_scores: std::collections::HashMap<String, f64>"),
        "record should be HashMap<String, f64>"
    );
}

// =============================================================================
// Special types
// =============================================================================

#[test]
fn test_int64_type()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            counters: defineTable({
                bigCount: v.int64(),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub big_count: i64"), "int64 should be i64");
}

#[test]
fn test_bytes_type()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            blobs: defineTable({
                data: v.bytes(),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub data: Vec<u8>"), "bytes should be Vec<u8>");
}

// =============================================================================
// Schema-level shared validators (cross-file references)
// =============================================================================

#[test]
fn test_shared_validator_reference()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export const chatType = v.union(
            v.literal("Dialog"),
            v.literal("Group"),
        );

        export default defineSchema({
            chats: defineTable({
                chatType: chatType,
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ChatsChatType"), "missing ChatsChatType enum");
    assert!(code.contains("Dialog"), "missing Dialog variant");
    assert!(code.contains("Group"), "missing Group variant");
}

// =============================================================================
// Function args with typed unions
// =============================================================================

#[test]
fn test_function_arg_tagged_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({ name: v.string() }),
        });
        "#,
        Some(vec![(
            r#"
            import { v } from "convex/values";
            import { mutation } from "./_generated/server";

            export const complete = mutation({
                args: {
                    itemId: v.id("items"),
                    result: v.union(
                        v.object({ type: v.literal("Success"), value: v.number() }),
                        v.object({ type: v.literal("Failed"), error: v.string() }),
                    ),
                },
                returns: v.null(),
                handler: async (ctx, args) => {},
            });
            "#,
            "tasks.ts",
        )]),
    );

    assert!(code.contains("pub struct TasksCompleteArgs"), "missing TasksCompleteArgs");
    assert!(
        code.contains("pub enum TasksCompleteResult"),
        "missing TasksCompleteResult tagged enum"
    );
    assert!(
        code.contains("#[serde(tag = \"type\")]"),
        "tagged union should have serde tag"
    );
    assert!(code.contains("Success {"), "missing Success variant");
    assert!(code.contains("Failed {"), "missing Failed variant");
    assert!(code.contains("value: f64"), "missing value field in Success");
    assert!(code.contains("error: String"), "missing error field in Failed");
}

// =============================================================================
// Function returns with typed subscriptions
// =============================================================================

#[test]
fn test_typed_query_return()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export const itemDoc = v.object({
            _id: v.id("items"),
            _creationTime: v.number(),
            name: v.string(),
        });

        export default defineSchema({
            items: defineTable({ name: v.string() }),
        });
        "#,
        Some(vec![(
            r#"
            import { v } from "convex/values";
            import { query } from "./_generated/server";
            import { itemDoc } from "./schema";

            export const list = query({
                args: {},
                returns: v.array(itemDoc),
                handler: async (ctx) => {
                    return await ctx.db.query("items").collect();
                },
            });
            "#,
            "items.ts",
        )]),
    );

    // TypedSubscription wrapper should be generated
    assert!(
        code.contains("pub struct TypedSubscription<T>"),
        "missing TypedSubscription struct"
    );
    assert!(
        code.contains("impl<T: serde::de::DeserializeOwned> futures_core::Stream for TypedSubscription<T>"),
        "missing Stream impl"
    );

    // Subscribe should return TypedSubscription<Vec<ItemsTable>>
    assert!(
        code.contains("TypedSubscription<Vec<ItemsTable>>"),
        "subscribe should return TypedSubscription<Vec<ItemsTable>>"
    );

    // Query should return Vec<ItemsTable>
    assert!(
        code.contains("-> anyhow::Result<Vec<ItemsTable>>"),
        "query should return anyhow::Result<Vec<ItemsTable>>"
    );
}

#[test]
fn test_mutation_null_return()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({ name: v.string() }),
        });
        "#,
        Some(vec![(
            r#"
            import { v } from "convex/values";
            import { mutation } from "./_generated/server";

            export const create = mutation({
                args: { name: v.string() },
                returns: v.null(),
                handler: async (ctx, { name }) => {
                    await ctx.db.insert("items", { name });
                },
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(
        code.contains("-> anyhow::Result<()>"),
        "mutation with v.null() return should be anyhow::Result<()>"
    );
}

#[test]
fn test_untyped_query_no_return()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({ name: v.string() }),
        });
        "#,
        Some(vec![(
            r#"
            import { v } from "convex/values";
            import { query } from "./_generated/server";

            export const list = query({
                args: {},
                handler: async (ctx) => {
                    return await ctx.db.query("items").collect();
                },
            });
            "#,
            "items.ts",
        )]),
    );

    // Without `returns`, subscribe falls back to raw QuerySubscription
    assert!(
        code.contains("-> anyhow::Result<convex::QuerySubscription>"),
        "untyped query subscribe should return raw QuerySubscription"
    );
    assert!(
        code.contains("-> anyhow::Result<convex::FunctionResult>"),
        "untyped query should return FunctionResult"
    );
}
