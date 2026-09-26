#[path = "support/harness.rs"]
mod support;

use serde_json::json;
use support::{CaptureServer, SequenceCaptureServer, test_config};
use unifi::actions::{ActionDispatcher, ActionRequest};

#[tokio::test]
async fn connector_path_rejects_non_integration_prefix() {
    let dispatcher = ActionDispatcher::new_for_test(test_config("https://gateway.local"));
    let result = dispatcher
        .execute(ActionRequest {
            action: "official_connector_get".into(),
            params: json!({"id": "console-1", "path": "/proxy/network/api/s/default/stat/sta"}),
        })
        .await;
    let message = result.unwrap_err().to_string();
    assert!(message.contains("connector path is outside"));
}

#[test]
fn existing_clients_action_is_internal() {
    let cap = unifi::capabilities::find_capability("clients").expect("clients capability");
    assert_eq!(cap.path.as_deref(), Some("/stat/sta"));
}

#[tokio::test]
async fn official_list_clients_sends_expected_get_request() {
    let server = CaptureServer::spawn(200, r#"{"items":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_list_clients".into(),
            params: json!({"siteId": "site-1", "query": {"limit": 1}}),
        })
        .await
        .expect("official list clients should succeed");

    let request = server.request();
    assert!(
        request
            .starts_with("get /proxy/network/integration/v1/sites/site-1/clients?limit=1 http/1.1")
    );
    assert!(request.contains("x-api-key: test-key"));
}

#[tokio::test]
async fn official_list_clients_resolves_configured_site_id() {
    let server = SequenceCaptureServer::spawn(vec![
        (
            200,
            r#"{"data":[{"id":"site-1","internalReference":"default","name":"Default"}]}"#,
        ),
        (200, r#"{"items":[]}"#),
    ]);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_list_clients".into(),
            params: json!({"query": {"limit": 1}}),
        })
        .await
        .expect("official list clients should resolve the configured site");

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("get /proxy/network/integration/v1/sites http/1.1"));
    assert!(
        requests[1]
            .starts_with("get /proxy/network/integration/v1/sites/site-1/clients?limit=1 http/1.1")
    );
}

#[tokio::test]
async fn official_create_network_sends_body() {
    let server = CaptureServer::spawn(201, r#"{"id":"network-1"}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_create_network".into(),
            params: json!({"siteId": "site-1", "body": {"name": "IoT"}}),
        })
        .await
        .expect("official create network should succeed");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/integration/v1/sites/site-1/networks "));
    assert!(request.contains(r#""name":"iot""#));
}

#[tokio::test]
async fn official_path_params_accept_numbers_and_encode_segments() {
    let server = CaptureServer::spawn(200, r#"{"ok":true}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_get_network_details".into(),
            params: json!({"siteId": "site-1", "networkId": "net/a?b"}),
        })
        .await
        .expect("encoded network details should succeed");

    let request = server.request();
    assert!(
        request.starts_with("get /proxy/network/integration/v1/sites/site-1/networks/net%2fa%3fb ")
    );

    let server = CaptureServer::spawn(200, r#"{"ok":true}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));
    dispatcher
        .execute(ActionRequest {
            action: "official_execute_port_action".into(),
            params: json!({
                "siteId": "site-1",
                "deviceId": "device-1",
                "portIdx": 1,
                "body": {"action": "cycle-poe"}
            }),
        })
        .await
        .expect("numeric port path parameter should succeed");

    let request = server.request();
    assert!(request.starts_with(
        "post /proxy/network/integration/v1/sites/site-1/devices/device-1/interfaces/ports/1/actions "
    ));
}

#[tokio::test]
async fn official_connector_get_allows_integration_proxy_path() {
    let server = CaptureServer::spawn(200, r#"{"ok":true}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_connector_get".into(),
            params: json!({
                "id": "console-1",
                "path": "/proxy/network/integration/v1/sites"
            }),
        })
        .await
        .expect("connector get should allow integration proxy path");

    let request = server.request();
    assert!(request.starts_with(
        "get /proxy/network/integration/v1/connector/consoles/console-1/proxy/network/integration/v1/sites "
    ));
}

#[tokio::test]
async fn http_query_must_be_object() {
    let dispatcher = ActionDispatcher::new_for_test(test_config("https://gateway.local"));
    let result = dispatcher
        .execute(ActionRequest {
            action: "official_list_clients".into(),
            params: json!({"siteId": "site-1", "query": "limit=1"}),
        })
        .await;
    let message = result.unwrap_err().to_string();
    assert!(message.contains("query must be a JSON object"));
}

#[tokio::test]
async fn hybrid_defaults_to_internal_without_site_id() {
    let dispatcher = ActionDispatcher::new_for_test(test_config("https://gateway.local"));
    let result = dispatcher
        .execute(ActionRequest {
            action: "list_clients".into(),
            params: json!({}),
        })
        .await;
    let message = result.unwrap_err().to_string();
    assert!(message.contains("/proxy/network/api/s/default/stat/sta"));
}

#[tokio::test]
async fn hybrid_list_networks_defaults_to_registered_internal_action() {
    let server = CaptureServer::spawn(200, r#"{"data":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "list_networks".into(),
            params: json!({"prefer": "internal"}),
        })
        .await
        .expect("list_networks should resolve to an existing internal action");

    let request = server.request();
    assert!(request.starts_with("get /proxy/network/api/s/default/rest/networkconf "));
}

#[tokio::test]
async fn events_action_calls_rest_event_and_applies_limit() {
    let server = CaptureServer::spawn(200, r#"{"data":[{"id":1},{"id":2}]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "events".into(),
            params: json!({"limit": 1}),
        })
        .await
        .expect("events should succeed");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/v2/api/site/default/system-log/all "));
    assert!(request.contains("{}"));
    assert_eq!(result["data"].as_array().expect("data").len(), 1);
}

#[tokio::test]
async fn generated_internal_v2_action_uses_v2_site_prefix() {
    let server = CaptureServer::spawn(200, r#"{"data":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "unifi_list_acl_rules".into(),
            params: json!({}),
        })
        .await
        .expect("v2 internal action should succeed");

    let request = server.request();
    assert!(request.starts_with("get /proxy/network/v2/api/site/default/acl-rules "));
}

#[tokio::test]
async fn internal_read_post_actions_default_empty_body() {
    let server = CaptureServer::spawn(200, r#"{"data":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "unifi_list_events".into(),
            params: json!({}),
        })
        .await
        .expect("read POST action should succeed without caller body");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/v2/api/site/default/system-log/all "));
    assert!(request.contains("{}"));
}

#[tokio::test]
async fn client_sessions_defaults_to_time_range_body() {
    let server = CaptureServer::spawn(200, r#"{"data":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "unifi_get_client_sessions".into(),
            params: json!({}),
        })
        .await
        .expect("client sessions should supply a default time range");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/api/s/default/stat/session "));
    assert!(request.contains(r#""start":"#));
    assert!(request.contains(r#""end":"#));
}

#[tokio::test]
async fn gateway_settings_uses_existing_mgmt_setting_endpoint() {
    let server = CaptureServer::spawn(200, r#"{"data":[{"key":"mgmt"}]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "unifi_get_gateway_settings".into(),
            params: json!({}),
        })
        .await
        .expect("gateway settings should use the controller's mgmt setting endpoint");

    let request = server.request();
    assert!(request.starts_with("get /proxy/network/api/s/default/get/setting/mgmt "));
}

#[tokio::test]
async fn ips_events_use_security_system_log_fallback() {
    let server = CaptureServer::spawn(
        200,
        r#"{"data":[{"id":1,"category":"SECURITY"},{"id":2,"category":"CLIENT_DEVICES"}]}"#,
    );
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "unifi_get_ips_events".into(),
            params: json!({}),
        })
        .await
        .expect("IPS events should use security system-log events");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/v2/api/site/default/system-log/all "));
    assert_eq!(result["data"].as_array().expect("data").len(), 1);
}

#[tokio::test]
async fn traffic_flow_statistics_falls_back_to_traffic_flows_search() {
    let server = CaptureServer::spawn(200, r#"{"data":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "unifi_get_traffic_flow_statistics".into(),
            params: json!({}),
        })
        .await
        .expect("traffic flow statistics should use the working traffic-flows search");

    let request = server.request();
    assert!(request.starts_with("post /proxy/network/v2/api/site/default/traffic-flows "));
    assert!(request.contains("{}"));
}

#[tokio::test]
async fn firewall_ordering_accepts_single_zone_convenience_query() {
    let server = CaptureServer::spawn(200, r#"{"orderedFirewallPolicyIds":[]}"#);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    dispatcher
        .execute(ActionRequest {
            action: "official_get_firewall_policy_ordering".into(),
            params: json!({"siteId": "site-1", "query": {"firewallZoneId": "zone-1"}}),
        })
        .await
        .expect("firewall ordering should expand a single zone query");

    let request = server.request();
    assert!(
        request.starts_with(
            "get /proxy/network/integration/v1/sites/site-1/firewall/policies/ordering?"
        )
    );
    assert!(request.contains("sourcefirewallzoneid=zone-1"));
    assert!(request.contains("destinationfirewallzoneid=zone-1"));
}

#[tokio::test]
async fn hybrid_uses_official_when_site_id_is_present() {
    let dispatcher = ActionDispatcher::new_for_test(test_config("https://gateway.local"));
    let result = dispatcher
        .execute(ActionRequest {
            action: "list_clients".into(),
            params: json!({"siteId": "site-1"}),
        })
        .await;
    let message = result.unwrap_err().to_string();
    assert!(message.contains("/proxy/network/integration/v1/sites/site-1/clients"));
}

#[test]
fn all_hybrid_aliases_resolve_to_expected_targets() {
    let cases = [
        ("list_clients", "clients", "official_list_clients"),
        ("list_devices", "devices", "official_list_devices"),
        (
            "list_networks",
            "unifi_list_networks",
            "official_list_networks",
        ),
        ("list_wifi", "wlans", "official_list_wifi"),
        ("get_system_info", "sysinfo", "official_get_info"),
    ];

    for (action, internal, official) in cases {
        let (target, params) = unifi::actions::hybrid::resolve(action, &json!({})).unwrap();
        assert_eq!(target, internal, "{action} should default to internal");
        assert_eq!(params, json!({}));

        let (target, params) =
            unifi::actions::hybrid::resolve(action, &json!({"siteId": "site-1"})).unwrap();
        assert_eq!(target, official, "{action} should use official with siteId");
        assert_eq!(params, json!({"siteId": "site-1"}));

        let (target, params) = unifi::actions::hybrid::resolve(
            action,
            &json!({"siteId": "site-1", "prefer": "internal"}),
        )
        .unwrap();
        assert_eq!(target, internal, "{action} prefer=internal should win");
        assert_eq!(params, json!({"siteId": "site-1"}));
    }
}

#[test]
fn hybrid_preference_validation_is_explicit() {
    let (target, params) =
        unifi::actions::hybrid::resolve("list_clients", &json!({"prefer": "official"})).unwrap();
    assert_eq!(target, "official_list_clients");
    assert_eq!(params, json!({}));

    let message = unifi::actions::hybrid::resolve("list_clients", &json!({"prefer": "maybe"}))
        .unwrap_err()
        .to_string();
    assert!(message.contains("unknown hybrid preference"));
}

#[tokio::test]
async fn client_stats_posts_typed_report_request() {
    let server = CaptureServer::spawn(
        200,
        r#"{"data":[{"time":1000,"signal":-60,"tx_retries":2,"wifi_tx_attempts":100}]}"#,
    );
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "unifi_get_client_stats".into(),
            params: json!({
                "client_id": "AA:BB:CC:DD:EE:FF",
                "granularity": "5minutes",
                "start": 1000,
                "end": 2000
            }),
        })
        .await
        .expect("client stats should succeed");

    assert_eq!(result["data"][0]["signal"], -60);
    let request = server.request();
    assert!(request.starts_with("post /proxy/network/api/s/default/stat/report/5minutes.user "));
    assert!(request.contains(r#""mac":"aa:bb:cc:dd:ee:ff""#));
    assert!(request.contains(r#""start":1000"#));
    assert!(request.contains(r#""end":2000"#));
    assert!(request.contains(r#""wifi_tx_attempts""#));
}

#[tokio::test]
async fn client_stats_rejects_unknown_granularity_before_network_io() {
    let dispatcher = ActionDispatcher::new_for_test(test_config("https://gateway.local"));
    let error = dispatcher
        .execute(ActionRequest {
            action: "unifi_get_client_stats".into(),
            params: json!({
                "client_id": "aa:bb:cc:dd:ee:ff",
                "granularity": "seconds"
            }),
        })
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("granularity"));
    assert!(error.contains("5minutes"));
}

#[tokio::test]
async fn client_wifi_details_filters_the_requested_client() {
    let server = CaptureServer::spawn(
        200,
        r#"{"data":[
            {"mac":"aa:bb:cc:dd:ee:ff","ip":"10.1.0.113","signal":-47,"noise":-96,"tx_rate":576000,"rx_rate":1080000,"channel":36,"radio":"na"},
            {"mac":"11:22:33:44:55:66","ip":"10.1.0.99","signal":-70}
        ]}"#,
    );
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "unifi_get_client_wifi_details".into(),
            params: json!({"client_mac": "AA:BB:CC:DD:EE:FF"}),
        })
        .await
        .expect("wifi details should succeed");

    assert_eq!(result["mac"], "aa:bb:cc:dd:ee:ff");
    assert_eq!(result["signal"], -47);
    assert_eq!(result["noise"], -96);
    let request = server.request();
    assert!(request.starts_with("get /proxy/network/api/s/default/stat/sta "));
}

#[tokio::test]
async fn rf_history_correlates_client_samples_events_and_ap_context() {
    let server = SequenceCaptureServer::spawn(vec![
        (
            200,
            r#"{"data":[{
                "mac":"aa:bb:cc:dd:ee:ff",
                "ip":"10.1.0.113",
                "name":"macpoo",
                "ap_mac":"de:ad:be:ef:00:01",
                "signal":-47,
                "noise":-96,
                "satisfaction":100,
                "tx_rate":576000,
                "rx_rate":1080000,
                "tx_retries":119,
                "wifi_tx_attempts":543,
                "wifi_tx_dropped":0,
                "channel":36,
                "channelWidth":80,
                "radio":"na",
                "bssid":"de:ad:be:ef:00:02"
            }]}"#,
        ),
        (
            200,
            r#"{"data":[{
                "time":1000,
                "signal":-60,
                "tx_rate":400000,
                "rx_rate":800000,
                "satisfaction":96,
                "tx_retries":4,
                "tx_packets":100,
                "rx_packets":120,
                "wifi_tx_attempts":105,
                "wifi_tx_dropped":1,
                "radio_protocol_most_common":"ax",
                "na-signal":-60,
                "x-set-ap_macs":["de:ad:be:ef:00:01"]
            }]}"#,
        ),
        (
            200,
            r#"{"data":[{
                "timestamp":1500,
                "event":"CLIENT_DISCONNECTED_WIRELESS",
                "category":"CLIENT_DEVICES",
                "severity":"LOW",
                "title_raw":"WiFi Client Disconnected",
                "parameters":{
                    "CLIENT":{"id":"aa:bb:cc:dd:ee:ff","ip":"10.1.0.113","name":"macpoo"},
                    "DEVICE":{"id":"de:ad:be:ef:00:01","name":"Axilla"},
                    "CHANNEL":{"name":"36"},
                    "RADIO_BAND":{"name":"na"},
                    "SIGNAL_STRENGTH":{"name":"-65"}
                }
            }],"total_page_count":1}"#,
        ),
        (
            200,
            r#"{"data":[{
                "mac":"de:ad:be:ef:00:01",
                "name":"Axilla",
                "model":"U7-Pro",
                "radio_table_stats":[{
                    "radio":"na",
                    "channel":36,
                    "channel_width":80,
                    "cu_total":6,
                    "num_sta":12,
                    "tx_retry":8
                }]
            }]}"#,
        ),
    ]);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "get_client_rf_history".into(),
            params: json!({
                "client_mac": "aa:bb:cc:dd:ee:ff",
                "start": 0,
                "end": 2000,
                "granularity": "hourly"
            }),
        })
        .await
        .expect("RF history should succeed");

    assert_eq!(result["client"]["mac"], "aa:bb:cc:dd:ee:ff");
    assert_eq!(result["current"]["signal"], -47);
    assert_eq!(result["current"]["snr"], 49);
    assert_eq!(result["ap_context"]["name"], "Axilla");
    assert_eq!(result["ap_context"]["channel_utilization"], 6);

    let timeline = result["timeline"].as_array().expect("timeline");
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0]["kind"], "client_sample");
    assert_eq!(timeline[0]["timestamp"], 1000);
    assert_eq!(
        timeline[0]["provenance"]["endpoint"],
        "/stat/report/hourly.user"
    );
    assert_eq!(timeline[1]["kind"], "wifi_event");
    assert_eq!(timeline[1]["timestamp"], 1500);
    assert_eq!(timeline[1]["ap"]["name"], "Axilla");

    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert!(requests[0].starts_with("get /proxy/network/api/s/default/stat/sta "));
    assert!(requests[1].starts_with("post /proxy/network/api/s/default/stat/report/hourly.user "));
    assert!(requests[2].starts_with("post /proxy/network/v2/api/site/default/system-log/all "));
    assert!(requests[3].starts_with("get /proxy/network/api/s/default/stat/device "));
}

#[tokio::test]
async fn rf_history_paginates_system_log_events() {
    let server = SequenceCaptureServer::spawn(vec![
        (
            200,
            r#"{"data":[{"mac":"aa:bb:cc:dd:ee:ff","ip":"10.1.0.113","ap_mac":"de:ad:be:ef:00:01"}]}"#,
        ),
        (200, r#"{"data":[]}"#),
        (
            200,
            r#"{"data":[{"timestamp":1000,"event":"CLIENT_CONNECTED_WIRELESS","parameters":{"CLIENT":{"id":"aa:bb:cc:dd:ee:ff"}}}],"total_page_count":2}"#,
        ),
        (
            200,
            r#"{"data":[{"timestamp":1500,"event":"CLIENT_DISCONNECTED_WIRELESS","parameters":{"CLIENT":{"id":"aa:bb:cc:dd:ee:ff"}}}],"total_page_count":2}"#,
        ),
        (200, r#"{"data":[]}"#),
    ]);
    let dispatcher = ActionDispatcher::new_for_test(test_config(server.url()));

    let result = dispatcher
        .execute(ActionRequest {
            action: "get_client_rf_history".into(),
            params: json!({
                "client_mac": "aa:bb:cc:dd:ee:ff",
                "start": 0,
                "end": 2000,
                "granularity": "hourly"
            }),
        })
        .await
        .expect("RF history pagination should succeed");

    let timeline = result["timeline"].as_array().expect("timeline");
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0]["timestamp"], 1000);
    assert_eq!(timeline[1]["timestamp"], 1500);
    assert!(result["warnings"].as_array().expect("warnings").is_empty());

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert!(requests[2].contains(r#""pagenumber":0"#));
    assert!(requests[3].contains(r#""pagenumber":1"#));
}
