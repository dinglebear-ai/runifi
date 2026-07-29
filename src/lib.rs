pub mod app {
    pub use unifi::UnifiService;
}

pub mod cli;
pub mod config;
pub mod mcp;
pub mod setup;

pub use unifi::{UnifiService, actions, capabilities};

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing {
    use std::sync::Arc;

    use crate::{
        app::UnifiService,
        config::McpConfig,
        mcp::{AppState, AuthPolicy},
    };
    use unifi::{UnifiClient, UnifiConfig};

    fn stub_service() -> UnifiService {
        let client = UnifiClient::new(&UnifiConfig {
            url: "https://localhost:1".into(),
            api_key: "test".into(),
            site: "default".into(),
            skip_tls_verify: true,
            legacy: false,
        })
        .expect("stub client should build");
        UnifiService::new(client)
    }

    /// Invoke a named tool against an AppState, bypassing HTTP transport.
    /// Used by unit tests that want to test dispatch logic without the MCP server.
    pub async fn call_tool(
        state: &AppState,
        tool: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        crate::mcp::tools_dispatch(state, tool, args).await
    }

    pub fn loopback_state() -> AppState {
        AppState {
            config: McpConfig::default(),
            auth_policy: AuthPolicy::LoopbackDev,
            service: stub_service(),
        }
    }

    pub fn bearer_state(token: &str) -> AppState {
        AppState {
            config: McpConfig {
                api_token: Some(token.to_string()),
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted { auth_state: None },
            service: stub_service(),
        }
    }

    pub async fn oauth_state(data_dir: &std::path::Path) -> AppState {
        let auth_state = build_auth_state(data_dir).await;
        AppState {
            config: McpConfig {
                auth: crate::config::AuthConfig {
                    public_url: Some("https://unifi.example.com".to_string()),
                    ..Default::default()
                },
                ..McpConfig::default()
            },
            auth_policy: AuthPolicy::Mounted {
                auth_state: Some(Arc::new(auth_state)),
            },
            service: stub_service(),
        }
    }

    pub async fn build_auth_state(data_dir: &std::path::Path) -> lab_auth::state::AuthState {
        let vars: Vec<(String, String)> = vec![
            ("UNIFI_MCP_AUTH_MODE".into(), "oauth".into()),
            (
                "UNIFI_MCP_PUBLIC_URL".into(),
                "https://unifi.example.com".into(),
            ),
            ("UNIFI_MCP_GOOGLE_CLIENT_ID".into(), "test-client-id".into()),
            (
                "UNIFI_MCP_GOOGLE_CLIENT_SECRET".into(),
                "test-client-secret".into(),
            ),
            (
                "UNIFI_MCP_AUTH_ADMIN_EMAIL".into(),
                "admin@example.com".into(),
            ),
            (
                "UNIFI_MCP_AUTH_SQLITE_PATH".into(),
                data_dir.join("auth.db").to_str().unwrap().into(),
            ),
            (
                "UNIFI_MCP_AUTH_KEY_PATH".into(),
                data_dir.join("auth-jwt.pem").to_str().unwrap().into(),
            ),
        ];

        let auth_config = lab_auth::config::AuthConfigBuilder::new()
            .env_prefix("UNIFI_MCP")
            .session_cookie_name("unifi_mcp_session")
            .scopes_supported(vec!["unifi:read".into(), "unifi:admin".into()])
            .default_scope("unifi:read")
            .resource_path("/mcp")
            .build_from_sources(vars)
            .expect("test auth config should build");

        lab_auth::state::AuthState::new(auth_config)
            .await
            .expect("test auth state should init")
    }
}
