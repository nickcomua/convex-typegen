use convex_typegen::{generate, Configuration};
use std::path::PathBuf;

#[test]
fn test_crm_chat_schema() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../crm-chat/bins/convex-backend/convex/schema.ts");

    if !schema_path.exists() {
        eprintln!("Skipping: schema file not found at {:?}", schema_path);
        return;
    }

    let dir = tempdir::TempDir::new("convex_typegen_test").unwrap();
    let out_file = dir.path().join("convex_types.rs");

    let config = Configuration {
        schema_path,
        out_file: out_file.to_string_lossy().to_string(),
        function_paths: vec![],
    };

    generate(config).expect("Failed to generate types from CRM chat schema");

    let output = std::fs::read_to_string(&out_file).expect("Failed to read generated file");
    println!("Generated output:\n{}", output);

    // Verify key types were generated
    assert!(output.contains("HumansTable"), "Missing HumansTable struct");
    assert!(output.contains("ClientsTable"), "Missing ClientsTable struct");
    assert!(output.contains("PhoneAuthsTable"), "Missing PhoneAuthsTable struct");
    assert!(output.contains("QrAuthsTable"), "Missing QrAuthsTable struct");
    assert!(output.contains("NotificationsTable"), "Missing NotificationsTable struct");
    assert!(output.contains("MessagesTable"), "Missing MessagesTable struct");
    assert!(output.contains("ChatsTable"), "Missing ChatsTable struct");
}
