use rmcp::model::{
    GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt, PromptMessage, Role,
};

pub(super) fn list_prompts() -> ListPromptsResult {
    ListPromptsResult {
        prompts: vec![Prompt::new(
            "network_summary",
            Some(
                "Check site health, connected clients, and alarms, then summarize the network status.",
            ),
            None,
        )],
        ..Default::default()
    }
}

pub(super) fn get_prompt(request: GetPromptRequestParams) -> anyhow::Result<GetPromptResult> {
    match request.name.as_str() {
        "network_summary" => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
            Role::User,
            "Use the unifi tool with action=health to retrieve site health, \
             then action=clients for connected client counts, \
             then action=alarms for active alarms. \
             Provide a concise summary covering overall site health, \
             number of connected clients, active alarms, and any devices with issues.",
        )])
        .with_description("Summarize the UniFi network status")),
        other => Err(anyhow::anyhow!("unknown prompt: {other}")),
    }
}
