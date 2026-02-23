//! Generate Rust types from Convex schema and function definitions.
//!
//! This crate runs a Bun-based extractor that mocks Convex packages and
//! executes your actual `schema.ts` + function files. The mock `v.*` calls
//! produce JSON descriptors that are then converted into:
//!
//! - **Table structs** with serde attributes for each table in the schema
//! - **Function arg structs** for every query, mutation, and action
//! - **`ConvexApi` trait** on `ConvexClient` with typed `subscribe_*`, `query_*`, and mutation methods
//!
//! # Usage
//!
//! Add to `build.rs`:
//!
//! ```rust,no_run
//! use convex_typegen::{generate, Configuration};
//!
//! fn main()
//! {
//!     let config = Configuration {
//!         function_paths: vec![std::path::PathBuf::from("convex/myFunctions.ts")],
//!         ..Default::default()
//!     };
//!     generate(config).expect("convex-typegen failed");
//! }
//! ```

mod bun_installer;
mod codegen;
pub mod errors;
mod extract;
pub(crate) mod types;

use std::collections::HashMap;
use std::path::PathBuf;

use codegen::generate_code;
use errors::ConvexTypeGeneratorError;

/// Configuration options for the type generator.
#[derive(Debug, Clone)]
pub struct Configuration
{
    /// Path to the Convex schema file (default: "convex/schema.ts")
    pub schema_path: PathBuf,

    /// Output file path for generated Rust types (default: "src/convex_types.rs")
    pub out_file: PathBuf,

    /// Paths to Convex function files for generating function argument types
    pub function_paths: Vec<PathBuf>,

    /// Map of import pattern (regex) → stub file path.
    ///
    /// Used to redirect project-specific helper imports to no-op stubs during
    /// extraction. The Bun plugin intercepts matching imports and loads the
    /// stub file instead.
    ///
    /// Example: `{ "helpers/result" => PathBuf::from("convex/helpers/result_stub.ts") }`
    pub helper_stubs: HashMap<String, PathBuf>,
}

impl Default for Configuration
{
    fn default() -> Self
    {
        Self {
            schema_path: PathBuf::from("convex/schema.ts"),
            out_file: PathBuf::from("src/convex_types.rs"),
            function_paths: Vec::new(),
            helper_stubs: HashMap::new(),
        }
    }
}

/// Generates Rust types from Convex schema and function definitions.
///
/// # Arguments
/// * `config` - Configuration options for the type generation process
///
/// # Returns
/// * `Ok(())` if type generation succeeds
/// * `Err(ConvexTypeGeneratorError)` if an error occurs during generation
///
/// # Errors
/// This function can fail for several reasons:
/// * Schema file not found
/// * Bun extractor script fails
/// * IO errors when writing the output file
/// * Network errors when downloading bun (first run only)
pub fn generate(config: Configuration) -> Result<(), ConvexTypeGeneratorError>
{
    if !config.schema_path.exists() {
        return Err(ConvexTypeGeneratorError::MissingSchemaFile);
    }

    let (schema, functions) = extract::extract(&config.schema_path, &config.function_paths, &config.helper_stubs)?;

    generate_code(&config.out_file, (schema, functions))?;

    Ok(())
}
