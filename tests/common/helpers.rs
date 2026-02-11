use convex::{FunctionResult, Value};

/// Assert that a query returned a value and pass it to the validator.
pub fn assert_query_value(
    result: anyhow::Result<FunctionResult>,
    validator: impl FnOnce(Value),
) {
    match result {
        Ok(FunctionResult::Value(v)) => validator(v),
        Ok(FunctionResult::ErrorMessage(msg)) => panic!("Query returned error: {msg}"),
        Ok(FunctionResult::ConvexError(err)) => {
            panic!("Query returned ConvexError: {}", err.message)
        }
        Err(e) => panic!("Query call failed: {e}"),
    }
}
