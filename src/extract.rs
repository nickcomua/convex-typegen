//! Bun-based type extraction — spawns `bun run` with the extractor script
//! and parses the JSON output into [`ConvexSchema`] + [`ConvexFunctions`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{thread, time::Duration};

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::bun_installer;
use crate::errors::ConvexTypeGeneratorError;
use crate::types::{ConvexColumn, ConvexFunction, ConvexFunctionParam, ConvexSchema, ConvexTable};

// ---------------------------------------------------------------------------
// Deserialization types for Bun's JSON output
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct BunOutput
{
    schema: SchemaOutput,
    functions: Vec<FunctionOutput>,
}

#[derive(Deserialize)]
struct SchemaOutput
{
    tables: Vec<TableOutput>,
}

#[derive(Deserialize)]
struct TableOutput
{
    name: String,
    columns: Vec<ColumnOutput>,
}

#[derive(Deserialize)]
struct ColumnOutput
{
    name: String,
    data_type: JsonValue,
}

#[derive(Deserialize)]
struct FunctionOutput
{
    name: String,
    #[serde(rename = "type")]
    type_: String,
    params: Vec<ParamOutput>,
    return_type: Option<JsonValue>,
    file_name: String,
}

#[derive(Deserialize)]
struct ParamOutput
{
    name: String,
    data_type: JsonValue,
}

// ---------------------------------------------------------------------------
// Public extraction function
// ---------------------------------------------------------------------------

/// Run the Bun extractor against the given schema and function files.
///
/// The extractor uses mock Convex packages so that `v.*` calls produce JSON
/// descriptors instead of actual validators. The result is parsed into the
/// same types that [`crate::codegen`] expects.
pub(crate) fn extract(
    schema_path: &Path,
    function_paths: &[PathBuf],
    helper_stubs: &HashMap<String, PathBuf>,
) -> Result<(ConvexSchema, Vec<ConvexFunction>), ConvexTypeGeneratorError>
{
    let js_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("js");
    let extractor = js_dir.join("extractor.ts");

    // Serialize helper stubs as JSON for the Bun plugin
    let stubs_json = serde_json::to_string(helper_stubs).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Failed to serialize helper stubs: {e}"),
    })?;

    let schema_abs = schema_path.canonicalize().map_err(|e| ConvexTypeGeneratorError::IOError {
        file: schema_path.display().to_string(),
        error: e,
    })?;

    // Get or download the bun binary
    let bun_path = bun_installer::get_bun_path()?;

    // The extractor registers its own plugin via Bun.plugin() — no --preload needed
    let mut cmd = Command::new(&bun_path);
    cmd.arg("run")
        .arg(&extractor)
        .arg(&schema_abs)
        .env("TYPEGEN_HELPER_STUBS", &stubs_json);

    for fp in function_paths {
        let abs = fp.canonicalize().map_err(|e| ConvexTypeGeneratorError::IOError {
            file: fp.display().to_string(),
            error: e,
        })?;
        cmd.arg(abs);
    }

    // Retry on ETXTBSY ("Text file busy") which can happen if another thread
    // just finished writing the bun binary.
    let output = {
        let mut last_err = None;
        let mut result = None;
        for attempt in 0..5u64 {
            match cmd.output() {
                Ok(out) => {
                    result = Some(out);
                    break;
                }
                Err(e) => {
                    let is_text_busy = e.raw_os_error() == Some(26);
                    if is_text_busy && attempt < 4 {
                        thread::sleep(Duration::from_millis(200 * (attempt + 1)));
                        last_err = Some(e);
                        continue;
                    }
                    return Err(ConvexTypeGeneratorError::ExtractionFailed {
                        details: format!("Failed to spawn bun ({}): {e}", bun_path.display()),
                    });
                }
            }
        }
        result.ok_or_else(|| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!(
                "Failed to spawn bun ({}) after retries: {}",
                bun_path.display(),
                last_err.map(|e| e.to_string()).unwrap_or_default()
            ),
        })?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("bun exited with {}: {stderr}", output.status),
        });
    }

    let bun_output: BunOutput =
        serde_json::from_slice(&output.stdout).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to parse bun output: {e}"),
        })?;

    // Convert to the shared types that codegen expects
    let schema = ConvexSchema {
        tables: bun_output
            .schema
            .tables
            .into_iter()
            .map(|t| ConvexTable {
                name: t.name,
                columns: t
                    .columns
                    .into_iter()
                    .map(|c| ConvexColumn {
                        name: c.name,
                        data_type: c.data_type,
                    })
                    .collect(),
            })
            .collect(),
    };

    let functions = bun_output
        .functions
        .into_iter()
        .map(|f| ConvexFunction {
            name: f.name,
            type_: f.type_,
            params: f
                .params
                .into_iter()
                .map(|p| ConvexFunctionParam {
                    name: p.name,
                    data_type: p.data_type,
                })
                .collect(),
            return_type: f.return_type,
            file_name: f.file_name,
        })
        .collect();

    Ok((schema, functions))
}
