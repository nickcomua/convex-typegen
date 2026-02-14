//! Shared type definitions for the Convex schema and function descriptors.
//!
//! These types are the stable interface between the extraction layer (which
//! produces them) and the codegen layer (which consumes them).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// The convex schema.
///
/// A schema can contain many tables. <https://docs.convex.dev/database/schemas>
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexSchema
{
    pub(crate) tables: Vec<ConvexTable>,
}

/// A table in the convex schema.
///
/// A table can contain many columns.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexTable
{
    /// The name of the table.
    pub(crate) name: String,
    /// The columns in the table.
    pub(crate) columns: Vec<ConvexColumn>,
}

/// A column in the convex schema.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexColumn
{
    /// The name of the column.
    pub(crate) name: String,
    /// The data type of the column.
    /// <https://docs.rs/convex/latest/convex/enum.Value.html>
    pub(crate) data_type: JsonValue,
}

/// A collection of all convex functions.
pub(crate) type ConvexFunctions = Vec<ConvexFunction>;

/// Convex functions (Queries, Mutations, and Actions)
///
/// <https://docs.convex.dev/functions>
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunction
{
    pub(crate) name: String,
    pub(crate) params: Vec<ConvexFunctionParam>,
    pub(crate) return_type: Option<JsonValue>,
    pub(crate) type_: String,
    pub(crate) file_name: String,
}

/// A parameter in a convex function.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConvexFunctionParam
{
    pub(crate) name: String,
    pub(crate) data_type: JsonValue,
}
