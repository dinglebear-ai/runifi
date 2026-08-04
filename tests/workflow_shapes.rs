#[test]
fn mcp_registry_publication_uses_the_shared_dns_only_workflow() {
    let docker = include_str!("../.github/workflows/docker-publish.yml");
    let registry = include_str!("../.github/workflows/mcp-registry.yml");

    assert!(!docker.contains("mcp-publisher"));
    assert!(!docker.contains("registry.modelcontextprotocol.io"));
    assert!(!docker.contains("MCP_PRIVATE_KEY"));
    assert!(registry.contains("mcp-registry-publish.yml@befa67c7b7f976235bf3fbced6ede93293a7f405"));
    assert!(registry.contains("workflow_dispatch:"));
    assert!(registry.contains("expected-version:"));
    assert!(registry.contains("manifest-path: server.json"));
    assert!(registry.contains("MCP_PRIVATE_KEY"));
    assert!(!registry.contains("auth-method:"));
}

#[test]
fn registry_description_stays_within_the_public_schema_limit() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../server.json")).expect("server.json must parse");
    let description = manifest["description"]
        .as_str()
        .expect("description must be a string");
    assert!(description.chars().count() <= 100);
}
