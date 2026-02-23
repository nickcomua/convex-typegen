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

    // Create _generated/ stubs so function files can import from "./_generated/server"
    let generated_dir = temp_dir.path().join("_generated");
    fs::create_dir_all(&generated_dir).expect("Failed to create _generated dir");
    fs::write(
        generated_dir.join("server.ts"),
        r#"export { query, mutation, action, internalQuery, internalMutation, internalAction, httpAction } from "convex/server";"#,
    )
    .expect("Failed to write _generated/server stub");
    fs::write(
        generated_dir.join("api.ts"),
        r#"export { anyApi as api, anyApi as internal } from "convex/server";"#,
    )
    .expect("Failed to write _generated/api stub");

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
        out_file: output_path.clone(),
        function_paths,
        helper_stubs: std::collections::HashMap::new(),
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

// -----------------------------------------------------------------------------
// Untagged / mixed unions
// -----------------------------------------------------------------------------

#[test]
fn test_untagged_primitive_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                value: v.union(v.string(), v.number()),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsValue"), "missing ItemsValue enum");
    assert!(code.contains("#[serde(untagged)]"), "mixed union should be untagged");
    assert!(code.contains("String(String)"), "missing String variant");
    assert!(code.contains("Number(f64)"), "missing Number variant");
    assert!(!code.contains("Copy"), "mixed union should NOT derive Copy");
}

#[test]
fn test_untagged_three_primitives()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                val: v.union(v.string(), v.number(), v.boolean()),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsVal"), "missing ItemsVal enum");
    assert!(code.contains("#[serde(untagged)]"), "should be untagged");
    assert!(code.contains("String(String)"), "missing String variant");
    assert!(code.contains("Number(f64)"), "missing Number variant");
    assert!(code.contains("Boolean(bool)"), "missing Boolean variant");
}

#[test]
fn test_untagged_string_and_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                meta: v.union(
                    v.string(),
                    v.object({ key: v.string(), value: v.string() }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsMeta"), "missing ItemsMeta enum");
    assert!(code.contains("#[serde(untagged)]"), "should be untagged");
    assert!(code.contains("String(String)"), "missing String variant");
    assert!(code.contains("Object("), "missing Object variant");
    // The nested struct should be generated for the object variant
    assert!(code.contains("pub key: String"), "missing key field in nested struct");
    assert!(code.contains("pub value: String"), "missing value field in nested struct");
}

#[test]
fn test_result_pattern_schema_field()
{
    // {Ok: T} | {Err: E} → Result<T, E>
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                result: v.union(
                    v.object({ Ok: v.string() }),
                    v.object({ Err: v.string() }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("Result<String, String>"),
        "result pattern should use Result<T, E>, got:\n{code}"
    );
    assert!(!code.contains("Object2("), "should NOT fall through to Object/Object2");
}

#[test]
fn test_result_pattern_null_value()
{
    // result(v.null()) → Result<(), String> (most common — unit mutations)
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                result: v.union(
                    v.object({ Ok: v.null() }),
                    v.object({ Err: v.string() }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("Result<(), String>"),
        "result(v.null()) should produce Result<(), String>, got:\n{code}"
    );
}

#[test]
fn test_result_pattern_non_matching_keys()
{
    // {Foo: T} | {Bar: E} should NOT match the Result pattern
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                result: v.union(
                    v.object({ Foo: v.string() }),
                    v.object({ Bar: v.string() }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(
        !code.contains("Result<"),
        "non-matching keys should NOT produce Result<T, E>, got:\n{code}"
    );
    assert!(code.contains("#[serde(untagged)]"), "should fall through to untagged enum");
}

#[test]
fn test_untagged_three_objects_deduplication()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                shape: v.union(
                    v.object({ kind: v.literal("circle"), radius: v.number() }),
                    v.object({ kind: v.literal("rect"), width: v.number(), height: v.number() }),
                    v.object({ kind: v.literal("point") }),
                ),
            }),
        });
        "#,
        None,
    );

    // These are NOT tagged (discriminator is "kind", not "type")
    assert!(code.contains("pub enum ItemsShape"), "missing ItemsShape enum");
    assert!(
        code.contains("#[serde(untagged)]"),
        "should be untagged (discriminant is 'kind' not 'type')"
    );
    assert!(code.contains("Object("), "missing Object variant");
    assert!(code.contains("Object2("), "missing Object2 variant");
    assert!(code.contains("Object3("), "missing Object3 variant");
}

// -----------------------------------------------------------------------------
// Nullable union edge cases
// -----------------------------------------------------------------------------

#[test]
fn test_nullable_object_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                settings: v.union(
                    v.object({ theme: v.string(), lang: v.string() }),
                    v.null(),
                ),
            }),
        });
        "#,
        None,
    );

    // Should collapse to Option<T> with a generated struct for the object
    assert!(
        code.contains("pub settings: Option<ItemsSettings>"),
        "union(object, null) should be Option<ItemsSettings>"
    );
    assert!(code.contains("pub struct ItemsSettings"), "missing ItemsSettings struct");
    assert!(code.contains("pub theme: String"), "missing theme field");
    assert!(code.contains("pub lang: String"), "missing lang field");
}

#[test]
fn test_nullable_array_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                tags: v.union(v.array(v.string()), v.null()),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub tags: Option<Vec<String>>"),
        "union(array(string), null) should be Option<Vec<String>>"
    );
}

#[test]
fn test_multi_type_with_null_not_nullable()
{
    // union(string, number, null) has 2 non-null variants → NOT a nullable pattern
    // Falls through to mixed untagged enum
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                val: v.union(v.string(), v.number(), v.null()),
            }),
        });
        "#,
        None,
    );

    // Should be an enum, NOT Option<T>
    assert!(
        code.contains("pub enum ItemsVal"),
        "should generate an enum for 3-variant union"
    );
    assert!(code.contains("#[serde(untagged)]"), "should be untagged");
    assert!(!code.contains("Option<"), "multi-type + null should NOT collapse to Option");
}

// -----------------------------------------------------------------------------
// Tagged union edge cases
// -----------------------------------------------------------------------------

#[test]
fn test_tagged_union_unit_variants_only()
{
    // All variants are objects with type discriminator but no extra fields
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                status: v.union(
                    v.object({ type: v.literal("pending") }),
                    v.object({ type: v.literal("active") }),
                    v.object({ type: v.literal("closed") }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsStatus"), "missing ItemsStatus enum");
    assert!(code.contains("#[serde(tag = \"type\")]"), "should be tagged");
    // All unit variants (no braces)
    assert!(code.contains("    Pending,"), "missing Pending unit variant");
    assert!(code.contains("    Active,"), "missing Active unit variant");
    assert!(code.contains("    Closed,"), "missing Closed unit variant");
    assert!(!code.contains("Pending {"), "unit variant should NOT have braces");
}

#[test]
fn test_tagged_union_with_nested_object()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            events: defineTable({
                event: v.union(
                    v.object({
                        type: v.literal("message"),
                        payload: v.object({ text: v.string(), sender: v.string() }),
                    }),
                    v.object({
                        type: v.literal("ping"),
                    }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("#[serde(tag = \"type\")]"), "should be tagged");
    assert!(code.contains("Message {"), "missing Message variant");
    assert!(code.contains("Ping"), "missing Ping variant");
    // Nested struct for the payload object
    assert!(
        code.contains("pub struct EventsEventMessagePayload") || code.contains("payload: EventsEventMessagePayload"),
        "should generate nested struct for Message payload"
    );
}

#[test]
fn test_tagged_union_with_nested_union()
{
    // A tagged union where one variant has a field that is itself a literal union
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            events: defineTable({
                action: v.union(
                    v.object({
                        type: v.literal("reaction"),
                        emoji: v.union(v.literal("thumbsUp"), v.literal("heart"), v.literal("fire")),
                    }),
                    v.object({ type: v.literal("view") }),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("#[serde(tag = \"type\")]"), "outer should be tagged");
    assert!(code.contains("Reaction {"), "missing Reaction variant");
    assert!(code.contains("View"), "missing View variant");
    // Inner literal union should be a Copy enum
    assert!(code.contains("ThumbsUp"), "missing ThumbsUp literal variant");
    assert!(code.contains("Heart"), "missing Heart literal variant");
    assert!(code.contains("Fire"), "missing Fire literal variant");
}

// -----------------------------------------------------------------------------
// Literal union edge cases
// -----------------------------------------------------------------------------

#[test]
fn test_literal_union_serde_rename()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                kind: v.union(
                    v.literal("my_item"),
                    v.literal("yourItem"),
                    v.literal("OurItem"),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsKind"), "missing ItemsKind enum");
    assert!(code.contains("Copy"), "literal union should derive Copy");
    // snake_case → PascalCase needs rename
    assert!(
        code.contains("#[serde(rename = \"my_item\")]"),
        "should rename my_item to PascalCase"
    );
    assert!(code.contains("MyItem"), "missing MyItem variant");
    // camelCase → PascalCase needs rename
    assert!(code.contains("#[serde(rename = \"yourItem\")]"), "should rename yourItem");
    assert!(code.contains("YourItem"), "missing YourItem variant");
    // Already PascalCase → no rename needed
    assert!(code.contains("OurItem"), "missing OurItem variant");
}

#[test]
fn test_literal_union_two_variants()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            flags: defineTable({
                toggle: v.union(v.literal("on"), v.literal("off")),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum FlagsToggle"), "missing FlagsToggle enum");
    assert!(code.contains("Copy"), "literal enum should derive Copy");
    assert!(code.contains("On"), "missing On variant");
    assert!(code.contains("Off"), "missing Off variant");
}

// -----------------------------------------------------------------------------
// Nested / compound union patterns
// -----------------------------------------------------------------------------

#[test]
fn test_array_of_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            logs: defineTable({
                entries: v.array(v.union(
                    v.literal("info"),
                    v.literal("warn"),
                    v.literal("error"),
                )),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub entries: Vec<LogsEntries>"), "should be Vec<LogsEntries>");
    assert!(code.contains("pub enum LogsEntries"), "missing LogsEntries enum");
    assert!(code.contains("Copy"), "literal union should derive Copy");
    assert!(code.contains("Info"), "missing Info variant");
    assert!(code.contains("Warn"), "missing Warn variant");
}

#[test]
fn test_optional_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                priority: v.optional(v.union(
                    v.literal("low"),
                    v.literal("medium"),
                    v.literal("high"),
                )),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub priority: Option<ItemsPriority>"),
        "optional union should be Option<EnumType>"
    );
    assert!(code.contains("pub enum ItemsPriority"), "missing ItemsPriority enum");
    assert!(code.contains("Low"), "missing Low variant");
    assert!(code.contains("Medium"), "missing Medium variant");
    assert!(code.contains("High"), "missing High variant");
}

#[test]
fn test_array_of_tagged_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            docs: defineTable({
                blocks: v.array(v.union(
                    v.object({ type: v.literal("text"), content: v.string() }),
                    v.object({ type: v.literal("image"), url: v.string(), alt: v.string() }),
                    v.object({ type: v.literal("divider") }),
                )),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub blocks: Vec<DocsBlocks>"), "should be Vec<DocsBlocks>");
    assert!(code.contains("pub enum DocsBlocks"), "missing DocsBlocks enum");
    assert!(code.contains("#[serde(tag = \"type\")]"), "should be tagged");
    assert!(code.contains("Text {"), "missing Text variant");
    assert!(code.contains("Image {"), "missing Image variant");
    assert!(code.contains("Divider"), "missing Divider variant");
    assert!(code.contains("content: String"), "missing content field");
    assert!(code.contains("url: String"), "missing url field");
    assert!(code.contains("alt: String"), "missing alt field");
}

#[test]
fn test_optional_tagged_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            tasks: defineTable({
                outcome: v.optional(v.union(
                    v.object({ type: v.literal("success"), value: v.number() }),
                    v.object({ type: v.literal("failure"), reason: v.string() }),
                )),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub outcome: Option<TasksOutcome>"),
        "optional tagged union should be Option<TasksOutcome>"
    );
    assert!(code.contains("pub enum TasksOutcome"), "missing TasksOutcome enum");
    assert!(code.contains("#[serde(tag = \"type\")]"), "should be tagged");
    assert!(code.contains("Success {"), "missing Success variant");
    assert!(code.contains("Failure {"), "missing Failure variant");
}

#[test]
fn test_union_of_arrays()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            items: defineTable({
                data: v.union(
                    v.array(v.string()),
                    v.array(v.number()),
                ),
            }),
        });
        "#,
        None,
    );

    assert!(code.contains("pub enum ItemsData"), "missing ItemsData enum");
    assert!(code.contains("#[serde(untagged)]"), "should be untagged");
    // Both variants wrap an array type
    assert!(code.contains("Vec<String>"), "missing Vec<String> variant");
    assert!(code.contains("Vec<f64>"), "missing Vec<f64> variant");
}

#[test]
fn test_union_in_record_value()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            configs: defineTable({
                settings: v.record(v.string(), v.union(v.string(), v.number())),
            }),
        });
        "#,
        None,
    );

    assert!(
        code.contains("pub settings: std::collections::HashMap<String,"),
        "should be a HashMap"
    );
    // The value type should be a generated enum
    assert!(
        code.contains("pub enum ConfigsSettings") || code.contains("pub enum ConfigsSettingsValue"),
        "should generate an enum for the record value union"
    );
}

// -----------------------------------------------------------------------------
// Union types in function args and returns
// -----------------------------------------------------------------------------

#[test]
fn test_function_arg_literal_union()
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

            export const update = mutation({
                args: {
                    status: v.union(
                        v.literal("active"),
                        v.literal("paused"),
                        v.literal("stopped"),
                    ),
                },
                returns: v.null(),
                handler: async (ctx, args) => {},
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(code.contains("pub enum ItemsUpdateStatus"), "missing ItemsUpdateStatus enum");
    assert!(code.contains("Copy"), "literal union in args should derive Copy");
    assert!(code.contains("Active"), "missing Active variant");
    assert!(code.contains("Paused"), "missing Paused variant");
    assert!(code.contains("Stopped"), "missing Stopped variant");
}

#[test]
fn test_function_arg_untagged_union()
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

            export const update = mutation({
                args: {
                    value: v.union(v.string(), v.number()),
                },
                returns: v.null(),
                handler: async (ctx, args) => {},
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(code.contains("pub enum ItemsUpdateValue"), "missing ItemsUpdateValue enum");
    assert!(code.contains("#[serde(untagged)]"), "should be untagged");
    assert!(code.contains("String(String)"), "missing String variant");
    assert!(code.contains("Number(f64)"), "missing Number variant");
}

#[test]
fn test_function_return_tagged_union()
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

            export const getStatus = query({
                args: {},
                returns: v.union(
                    v.object({ type: v.literal("online"), since: v.number() }),
                    v.object({ type: v.literal("offline") }),
                ),
                handler: async (ctx) => {
                    return { type: "online", since: Date.now() };
                },
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(
        code.contains("pub enum ItemsGetStatusReturn"),
        "missing ItemsGetStatusReturn enum"
    );
    assert!(code.contains("#[serde(tag = \"type\")]"), "should be tagged");
    assert!(code.contains("Online {"), "missing Online variant");
    assert!(code.contains("Offline"), "missing Offline variant");
    assert!(code.contains("since: f64"), "missing since field");
}

#[test]
fn test_function_return_literal_union()
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

            export const process = mutation({
                args: { name: v.string() },
                returns: v.union(
                    v.literal("created"),
                    v.literal("updated"),
                    v.literal("skipped"),
                ),
                handler: async (ctx, args) => { return "created"; },
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(
        code.contains("pub enum ItemsProcessReturn"),
        "missing ItemsProcessReturn enum"
    );
    assert!(code.contains("Copy"), "literal return union should derive Copy");
    assert!(code.contains("Created"), "missing Created variant");
    assert!(code.contains("Updated"), "missing Updated variant");
    assert!(code.contains("Skipped"), "missing Skipped variant");
}

// -----------------------------------------------------------------------------
// Multiple tables with similar unions (verify distinct enum names)
// -----------------------------------------------------------------------------

#[test]
fn test_multiple_tables_same_field_different_union()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            posts: defineTable({
                status: v.union(v.literal("draft"), v.literal("published")),
            }),
            comments: defineTable({
                status: v.union(v.literal("visible"), v.literal("hidden"), v.literal("flagged")),
            }),
        });
        "#,
        None,
    );

    // Each table should get its own enum
    assert!(code.contains("pub enum PostsStatus"), "missing PostsStatus enum");
    assert!(code.contains("pub enum CommentsStatus"), "missing CommentsStatus enum");
    // PostsStatus variants
    assert!(code.contains("Draft"), "missing Draft in PostsStatus");
    assert!(code.contains("Published"), "missing Published in PostsStatus");
    // CommentsStatus variants
    assert!(code.contains("Visible"), "missing Visible in CommentsStatus");
    assert!(code.contains("Hidden"), "missing Hidden in CommentsStatus");
    assert!(code.contains("Flagged"), "missing Flagged in CommentsStatus");
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
        code.contains("Result<Vec<ItemsTable>, ConvexError>"),
        "query should return Result<Vec<ItemsTable>, ConvexError>"
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
        code.contains("Result<(), ConvexError>"),
        "mutation with v.null() return should be Result<(), ConvexError>"
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
        code.contains("Result<convex::QuerySubscription, ConvexError>"),
        "untyped query subscribe should return raw QuerySubscription"
    );
    assert!(
        code.contains("Result<convex::FunctionResult, ConvexError>"),
        "untyped query should return FunctionResult"
    );
}

// =============================================================================
// Optional args: BTreeMap From impl skips None fields
// =============================================================================

#[test]
fn test_optional_args_skip_none_in_btreemap()
{
    let code = generate_and_read(
        r#"
        import { defineSchema, defineTable } from "convex/server";
        import { v } from "convex/values";

        export default defineSchema({
            messages: defineTable({
                text: v.optional(v.string()),
                mediaId: v.optional(v.string()),
            }),
        });
        "#,
        Some(vec![(
            r#"
            import { v } from "convex/values";
            import { mutation } from "./_generated/server";

            export const upsert = mutation({
                args: {
                    chatId: v.string(),
                    text: v.optional(v.string()),
                    mediaId: v.optional(v.string()),
                },
                returns: v.null(),
                handler: async (ctx, args) => {},
            });
            "#,
            "messages.ts",
        )]),
    );

    // Required field should use unconditional map.insert
    assert!(
        code.contains(r#"map.insert("chatId".to_string()"#),
        "required field should use unconditional insert"
    );

    // Optional fields should use `if let Some(val)` to skip None
    assert!(
        code.contains(r#"if let Some(val) = _args.text {"#),
        "optional text field should use if let Some(val)"
    );
    assert!(
        code.contains(r#"if let Some(val) = _args.mediaId {"#),
        "optional mediaId field should use if let Some(val)"
    );

    // The unconditional pattern should NOT appear for optional fields
    assert!(
        !code.contains(r#"map.insert("text".to_string(), serde_json::to_value(_args.text)"#),
        "optional text should NOT use unconditional insert"
    );
    assert!(
        !code.contains(r#"map.insert("mediaId".to_string(), serde_json::to_value(_args.mediaId)"#),
        "optional mediaId should NOT use unconditional insert"
    );
}

#[test]
fn test_nullable_union_args_skip_none_in_btreemap()
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

            export const update = mutation({
                args: {
                    name: v.string(),
                    description: v.union(v.string(), v.null()),
                },
                returns: v.null(),
                handler: async (ctx, args) => {},
            });
            "#,
            "items.ts",
        )]),
    );

    // v.union(v.string(), v.null()) maps to Option<String> and should skip None
    assert!(
        code.contains("pub description: Option<String>"),
        "union(string, null) should be Option<String>"
    );
    assert!(
        code.contains(r#"if let Some(val) = _args.description {"#),
        "nullable union field should use if let Some(val)"
    );
}

// -----------------------------------------------------------------------------
// Result pattern as function return type
// -----------------------------------------------------------------------------

#[test]
fn test_mutation_result_return_null()
{
    // result(v.null()) as mutation return type → Result<Result<(), String>, ConvexError>
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
                returns: v.union(
                    v.object({ Ok: v.null() }),
                    v.object({ Err: v.string() }),
                ),
                handler: async (ctx, { name }) => {
                    await ctx.db.insert("items", { name });
                    return { Ok: null };
                },
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(
        code.contains("Result<Result<(), String>, ConvexError>"),
        "result(v.null()) return should be Result<Result<(), String>, ConvexError>, got:\n{code}"
    );
}

#[test]
fn test_mutation_result_return_id()
{
    // result(v.id("items")) as mutation return type → Result<Result<String, String>, ConvexError>
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
                returns: v.union(
                    v.object({ Ok: v.id("items") }),
                    v.object({ Err: v.string() }),
                ),
                handler: async (ctx, { name }) => {
                    const id = await ctx.db.insert("items", { name });
                    return { Ok: id };
                },
            });
            "#,
            "items.ts",
        )]),
    );

    assert!(
        code.contains("Result<Result<String, String>, ConvexError>"),
        "result(v.id()) return should be Result<Result<String, String>, ConvexError>, got:\n{code}"
    );
}
