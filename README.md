# convex-typegen

> Fork of [JamalLyons/convex-typegen](https://github.com/JamalLyons/convex-typegen) with extended function support.

Generate Rust types and a typed API trait from [Convex](https://www.convex.dev) schema and function files at build time using [oxc](https://oxc.rs).

## Fork changes

- **Cross-file identifier resolution** — exported `const` validators in `schema.ts` are resolved when used in function args
- **`ConvexApi` trait generation** — typed extension trait on `ConvexClient` with `subscribe_*` / `query_*` / `{file}_{fn}()` methods
- **Tagged union support** — objects with a `type` discriminator generate `#[serde(tag = "type")]` enums
- **Integration test suite** — end-to-end tests using testcontainers with a real Convex backend

## Installation

Add as a **build dependency** via git:

```toml
[build-dependencies]
convex-typegen = { git = "https://github.com/nickcomua/convex-typegen" }
```

You also need these runtime dependencies:

```toml
[dependencies]
convex = "0.10.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Usage

Create a `build.rs` that runs the generator:

```rust
use convex_typegen::{generate, Configuration};

fn main() {
    println!("cargo:rerun-if-changed=convex/schema.ts");

    // Auto-discover function files in convex/
    let mut function_paths: Vec<std::path::PathBuf> = std::fs::read_dir("convex")
        .expect("convex/ directory must exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if name.ends_with(".ts") && name != "schema.ts" && !name.starts_with('_') {
                println!("cargo:rerun-if-changed=convex/{}", name);
                Some(path)
            } else {
                None
            }
        })
        .collect();
    function_paths.sort();

    let config = Configuration {
        schema_path: std::path::PathBuf::from("convex/schema.ts"),
        out_file: format!("{}/convex_types.rs", std::env::var("OUT_DIR").unwrap()),
        function_paths,
    };

    generate(config).expect("convex-typegen failed");
}
```

Then include the generated types in your code:

```rust
include!(concat!(env!("OUT_DIR"), "/convex_types.rs"));
```

Run `cargo build` — types regenerate automatically when schema or function files change.

## What gets generated

| Convex type | Rust type |
|---|---|
| `v.string()` | `String` |
| `v.number()` | `f64` |
| `v.boolean()` | `bool` |
| `v.int64()` | `i64` |
| `v.bytes()` | `Vec<u8>` |
| `v.null()` | `()` |
| `v.id("table")` | `String` |
| `v.array(T)` | `Vec<T>` |
| `v.object({...})` | Named struct |
| `v.record(K, V)` | `HashMap<K, V>` |
| `v.union(T, v.null())` | `Option<T>` |
| `v.union(literals...)` | `enum` (Copy) |
| `v.union(tagged objects...)` | `#[serde(tag = "type")] enum` |
| `v.optional(T)` | `Option<T>` |
| `v.any()` | `serde_json::Value` |

For each query/mutation/action, the generator also produces:
- **Arg structs** (e.g. `ChatsGetArgs`) with `From<BTreeMap<String, JsonValue>>`
- **`ConvexApi` trait** on `ConvexClient` with typed methods

## Testing

Unit tests and codegen pipeline tests (no external dependencies):

```bash
cargo test
```

End-to-end tests against a real Convex backend (requires Docker + Node.js):

```bash
cargo test --test integration_test -- --nocapture
```

## Example

See [examples/basic/](examples/basic/) for a complete working project.

## License

MIT - see [LICENSE](LICENSE).
