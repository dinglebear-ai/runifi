pub mod hybrid;
pub mod internal;
pub mod official;

use anyhow::{Result, bail};
use serde_json::Value;

use crate::capabilities::find_capability;
use crate::{UnifiConfig, api::ApiSourceFamily};

#[derive(Debug, Clone)]
pub struct ActionRequest {
    pub action: String,
    pub params: Value,
}

pub struct ActionDispatcher {
    cfg: UnifiConfig,
}

impl ActionDispatcher {
    pub fn new(cfg: UnifiConfig) -> Self {
        Self { cfg }
    }

    pub fn new_for_test(cfg: UnifiConfig) -> Self {
        Self::new(cfg)
    }

    pub async fn execute(&self, request: ActionRequest) -> Result<Value> {
        let Some(capability) = find_capability(&request.action) else {
            bail!("unknown UniFi action: {}", request.action);
        };
        match capability.source {
            ApiSourceFamily::Official => {
                official::execute(&self.cfg, capability, &request.params).await
            }
            ApiSourceFamily::Internal => {
                internal::execute(&self.cfg, capability, &request.params).await
            }
            ApiSourceFamily::Hybrid => {
                let (target, params) =
                    hybrid::resolve(capability.action.as_str(), &request.params)?;
                let Some(target_capability) = find_capability(target) else {
                    bail!(
                        "hybrid action {} resolved to unknown action {target}",
                        capability.action
                    );
                };
                match target_capability.source {
                    ApiSourceFamily::Official => {
                        official::execute(&self.cfg, target_capability, &params).await
                    }
                    ApiSourceFamily::Internal => {
                        internal::execute(&self.cfg, target_capability, &params).await
                    }
                    ApiSourceFamily::Hybrid => {
                        bail!(
                            "hybrid action {} resolved to another hybrid action",
                            capability.action
                        )
                    }
                }
            }
        }
    }
}
