use std::path::PathBuf;

/// Deploy the example Convex functions to a running backend instance.
pub async fn deploy_convex(convex_url: &str, admin_key: &str) -> anyhow::Result<()> {
    let example_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/basic");

    // Ensure node_modules exist
    let npm_status = tokio::process::Command::new("npm")
        .arg("install")
        .current_dir(&example_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .status()
        .await?;

    if !npm_status.success() {
        anyhow::bail!("npm install failed with exit code: {npm_status}");
    }

    // Deploy functions (no env vars needed — example has no auth)
    let output = tokio::process::Command::new("npx")
        .arg("convex")
        .arg("deploy")
        .current_dir(&example_dir)
        .env("CONVEX_SELF_HOSTED_URL", convex_url)
        .env("CONVEX_SELF_HOSTED_ADMIN_KEY", admin_key)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!("convex deploy failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    Ok(())
}
