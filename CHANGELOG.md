# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.5](https://github.com/dinglebear-ai/runifi/compare/v0.2.4...v0.2.5) (2026-07-28)


### Bug Fixes

* align unifi rmcp runtime naming ([01583b2](https://github.com/dinglebear-ai/runifi/commit/01583b2ac4484f2b442eebe249b7788829338f4c))
* complete unifi rmcp package rename ([d9555c5](https://github.com/dinglebear-ai/runifi/commit/d9555c50996c9c379116fe1c0d0b4a3e7defd72f))
* load production runtime environment ([#12](https://github.com/dinglebear-ai/runifi/issues/12)) ([a220e87](https://github.com/dinglebear-ai/runifi/commit/a220e87d0ecd4e2c9a83baaafcc0174c34bb7b00))
* route rust builds through sccache wrapper ([965ab05](https://github.com/dinglebear-ai/runifi/commit/965ab0570d15a6e33e72b94373d94245d3c9063b))


### Documentation

* align README with RMCP guide ([31cbc01](https://github.com/dinglebear-ai/runifi/commit/31cbc01dd56883c095bac6dcaddffe9f1ee6de79))
* save session log ([959c7ea](https://github.com/dinglebear-ai/runifi/commit/959c7eab3a0fd81f0cfa8f386bc9ad631c367323))
* update OpenWiki ([#10](https://github.com/dinglebear-ai/runifi/issues/10)) ([0bf002c](https://github.com/dinglebear-ai/runifi/commit/0bf002cbce9f0c304b829d2bf039a3c2a960228f))


### Refactoring

* extract core unifi crate ([8a2d68e](https://github.com/dinglebear-ai/runifi/commit/8a2d68ee88e4aa8c6cb38c10550f703cfc5b2c49))
* move unifi actions into core crate ([9bccb82](https://github.com/dinglebear-ai/runifi/commit/9bccb82fd1b40d6b4c67259d4ee148049dd91a8d))


### CI

* authenticate OpenWiki Tailscale with OAuth ([e9ec961](https://github.com/dinglebear-ai/runifi/commit/e9ec96158db1f0129fb9a872a53e436f4e3b49d1))
* connect OpenWiki workflow to Tailscale ([4c3b68a](https://github.com/dinglebear-ai/runifi/commit/4c3b68a6e8c38f718b6cf86be7d67ad202ffefa4))
* drop the redundant gitleaks secret-scan job ([fe9740d](https://github.com/dinglebear-ai/runifi/commit/fe9740dd51c705248cfdb4726e06111bb3ef0a99))
* migrate Rust build to soldr/zccache caching ([c639033](https://github.com/dinglebear-ai/runifi/commit/c639033c601a91e9cb54782980c52abd2a49bb0d))
* retire no-MCP marketplace variant workflows ([f313dff](https://github.com/dinglebear-ai/runifi/commit/f313dffe53e34659ad71f7daf43c8361b88a5e7d))
* run OpenWiki on Tailscale runner ([166b0b5](https://github.com/dinglebear-ai/runifi/commit/166b0b57251945c92a6e289d2f91684ae204002d))
* update workflows to runs-on [self-hosted, unraid] ([4fc8761](https://github.com/dinglebear-ai/runifi/commit/4fc8761250740f161d6bbf11bdf4b2addd40dc9d))

## [0.2.4](https://github.com/jmagar/runifi/compare/v0.2.3...v0.2.4) (2026-07-11)


### CI

* harden npm release smoke ([60f8fc6](https://github.com/jmagar/runifi/commit/60f8fc6ab9d3423cbcfa6b205a50f9a9d03cfd9b))
* use tootie Tailscale OpenWiki endpoint ([7427e9d](https://github.com/jmagar/runifi/commit/7427e9dc1563d0ec85e2cadf00d375eb08ddd629))

## [0.2.3](https://github.com/jmagar/runifi/compare/v0.2.2...v0.2.3) (2026-07-11)


### Bug Fixes

* load runtime env from unifi-rmcp home ([24cc64b](https://github.com/jmagar/runifi/commit/24cc64b12d105ab2205f1f9f6e42a9a4ecd5fd2a))


### Documentation

* update OpenWiki ([f4306a0](https://github.com/jmagar/runifi/commit/f4306a0bb6c29a554374da63c8dda0d9684518ab))


### Tests

* point mcporter at local unifi server ([294e34b](https://github.com/jmagar/runifi/commit/294e34bc60b1c7fea48942fe7ba882d491a99ea0))


### CI

* skip mcp registry publish without secret ([2647842](https://github.com/jmagar/runifi/commit/26478425c4594c7c4ada00a69a7f3c50eeac463f))

## [0.2.2](https://github.com/jmagar/runifi/compare/v0.2.1...v0.2.2) (2026-07-10)


### CI

* skip mcp registry publish without secret ([2647842](https://github.com/jmagar/runifi/commit/26478425c4594c7c4ada00a69a7f3c50eeac463f))

## [0.2.0](https://github.com/jmagar/runifi/compare/v0.1.1...v0.2.0) (2026-07-05)


### Features

* add canonical unifi internal endpoint models ([d59a575](https://github.com/jmagar/runifi/commit/d59a575ab5d8c9f8baf8035794e8e9c19dbdf672))
* add full upstream network mcp parity ([94c5204](https://github.com/jmagar/runifi/commit/94c5204a1ed7ac4463ec0324ada9eef0f08c00f5))
* add official and internal unifi api actions ([3334e1a](https://github.com/jmagar/runifi/commit/3334e1a4a9e801f0745d9e55dd0396b38de758cc))
* add safe unifi verification modes ([fa48eb8](https://github.com/jmagar/runifi/commit/fa48eb888f62723a0b95885aa975ec61bccf1852))
* add unifi endpoint verifier ([1760e89](https://github.com/jmagar/runifi/commit/1760e89ecad87dae25e06429b5a585ec7ed8799d))
* **cli:** add 'unifi setup install' + self-install in plugin-hook ([33ae612](https://github.com/jmagar/runifi/commit/33ae6121da90624998651ee6fd460467b6be540f))
* **config:** load ~/.unifi-rmcp/.env at startup (dotenvy, symlink-guarded); appdata resolves to ~/.unifi-rmcp, not CLAUDE_PLUGIN_DATA ([307d866](https://github.com/jmagar/runifi/commit/307d8669e30aa8ef3caa5b9b0e0591321a2eda7c))
* enforce unifi capability auth scopes ([44be806](https://github.com/jmagar/runifi/commit/44be80648eaee5be740be2d6cf61a0257c47b051))
* verify official unifi api parity ([be9a361](https://github.com/jmagar/runifi/commit/be9a36195bcfb92e0bbcd1f7f2d3eb68753729e3))


### Bug Fixes

* address lavra review findings ([d18bacf](https://github.com/jmagar/runifi/commit/d18bacfc6b668089316c569eddf644ec8474d9ed))
* address pr toolkit review findings ([69ab391](https://github.com/jmagar/runifi/commit/69ab391d1639ebdafc09c165ece229ea478bd0f8))
* address unifi parity review findings ([9c11f7b](https://github.com/jmagar/runifi/commit/9c11f7b9de55a75f4e327a74e9c23d4b984cee05))
* **auth:** pass std::env::vars() to lab-auth build_from_sources ([b5bfca7](https://github.com/jmagar/runifi/commit/b5bfca71cf404d583746faca78b8c0caf99bcaea))
* classify firewall ordering as fixture gated ([605ce6c](https://github.com/jmagar/runifi/commit/605ce6cefa599ed33ea2ccc4995b8596cea9af11))
* **compose:** use repo-name default for Docker network ([e3e90c2](https://github.com/jmagar/runifi/commit/e3e90c25e330cc0c5886effe71896876ec09dd69))
* correct Tier 2 CLI binary name unifi -&gt; runifi ([09ddc93](https://github.com/jmagar/runifi/commit/09ddc931b6b82573f59a71d4338ecefed9d4ac5d))
* harden unifi endpoint verification ([201ec50](https://github.com/jmagar/runifi/commit/201ec50f54d8c00b0c5722d1a6e55e56bb3f61ff))
* harden unifi path substitution ([b5bff28](https://github.com/jmagar/runifi/commit/b5bff28be20a9d4b0bdbc094dd6a205f73272f42))
* keep docker build workspace intact ([03495ef](https://github.com/jmagar/runifi/commit/03495ef8a1f3ddf1c177d99116b9936f027075de))
* keep internal reference live verified ([06ac79e](https://github.com/jmagar/runifi/commit/06ac79e79ef8935f504c8a240eea741edf59f95c))
* **oauth:** parse UNIFI_MCP_AUTH_MODE in config load ([8cd1a1f](https://github.com/jmagar/runifi/commit/8cd1a1f2e186f16370025d18cbfced06f69f9a2d))
* remove mutation confirmation parameter ([c204684](https://github.com/jmagar/runifi/commit/c204684ee24c27b5ffad5bb1663fa5f3a8d89cb9))
* resolve audit blockers ([87b3a86](https://github.com/jmagar/runifi/commit/87b3a86d7852eb5ffba635061d66eb4dfcf111d8))
* tighten unifi api action coverage ([a5e5cd4](https://github.com/jmagar/runifi/commit/a5e5cd4fed1f04ed8654892ab05f0da6bb1597a1))


### Documentation

* align unifi skill event coverage ([5f939fa](https://github.com/jmagar/runifi/commit/5f939fa20a10606cf6ca623427341a382f15de6b))
* capture unifi api coverage inventories ([897129f](https://github.com/jmagar/runifi/commit/897129f24f73d65b23c5873c94745da17eae414c))
* document unifi parity verification ([6e9bd6d](https://github.com/jmagar/runifi/commit/6e9bd6d0fb57bb575dda1de470fcc88b12541681))
* plan unifi full parity ([745ff23](https://github.com/jmagar/runifi/commit/745ff23a9393a99786ab55e3f7521e0ceaaf1053))
* **rust:** align .cargo/config.toml and add docs/RUST.md ([58a7a6c](https://github.com/jmagar/runifi/commit/58a7a6c6108a41a44613dd4a98fce9fc53f78caa))
* save session log ([71504d6](https://github.com/jmagar/runifi/commit/71504d6e3db1ca5d67ec6d6dfb3d52ed1abe9cbc))


### Refactoring

* **plugin:** call runifi binary directly from hooks; port env mapping into the binary ([bc54db8](https://github.com/jmagar/runifi/commit/bc54db85b27656823cba503495ea43a07b1c4efe))
* rename binary unifi -&gt; runifi (avoid official-name collision) ([4060707](https://github.com/jmagar/runifi/commit/4060707d8d270a1d50b76e0b8f7c0879a70b9d4a))


### CI

* add marketplace-no-mcp auto-sync workflow [skip ci] ([5d3257e](https://github.com/jmagar/runifi/commit/5d3257ecaad5364b007ae709d56e315dac5af88c))
* add release-please automation ([b2faf9b](https://github.com/jmagar/runifi/commit/b2faf9b324b209a723a4ba8d6b845cdaa2ad87cc))

## [Unreleased]

## [0.1.1] — 2026-06-01

### Changed

- Plugin `SessionStart`/`ConfigChange` hooks now call `${CLAUDE_PLUGIN_ROOT}/bin/runifi setup plugin-hook` directly instead of going through the `plugin-setup.sh` shell wrapper. The env-var mapping the script performed (`CLAUDE_PLUGIN_OPTION_*` → `UNIFI_*`, plus `CLAUDE_PLUGIN_DATA` → `UNIFI_MCP_HOME`) now lives in `apply_plugin_options()` in `src/setup.rs`, applied at the top of the plugin-hook path. The script's `.env`-fallback was dropped (immaterial: the binary never persists option values to `.env` and the setup checks read live process env).

### Removed

- `plugins/unifi/hooks/plugin-setup.sh` — the wrapper was a pure env-mapping middleman now handled by the binary's `setup plugin-hook` command.

### Added

- Initial release of `unifi-rmcp` — UniFi MCP server bridging Claude to Ubiquiti network controllers
- MCP server with action-based tool dispatch (`unifi` tool, `action` parameter)
- Actions: `clients`, `devices`, `wlans`, `health`, `alarms`, `events`, `sysinfo`, `me`, `help`
- CLI thin shim with human-readable formatters and `--json` passthrough
- Bearer token + Google OAuth authentication via `lab-auth`
- Streamable HTTP transport on port 7474 + stdio transport
- Self-signed TLS support (`UNIFI_SKIP_TLS_VERIFY=true` default)
- Docker deployment with `ghcr.io/jmagar/runifi` image
- Claude Code plugin with userConfig
- `entrypoint.sh` with permission setup and runtime validation
- Git LFS for pre-built plugin binaries in `bin/`
- nextest configuration with `ci` profile
- taplo TOML formatter configuration
- lefthook pre-commit hooks (diff check, TOML format, env guard)
- GitHub Actions: CI, Docker publish, release workflows
- xtask crate with `dist`, `ci`, `symlink-docs`, `check-env` commands
