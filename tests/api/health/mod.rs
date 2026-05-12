use reqwest::StatusCode;

use crate::get_client;

#[test]
fn test_health_live_success() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health/live", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Expected 200 OK for /health/live, got: {:#?}",
        res
    );

    let body: serde_json::Value = res.json().expect("Failed to parse response");
    assert_eq!(body["status"], "UP", "Expected status UP in response");
    assert!(
        body["checks"].is_array(),
        "Expected checks array in response"
    );
}

#[test]
fn test_health_ready_success() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health/ready", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Expected 200 OK for /health/ready, got: {:#?}",
        res
    );

    let body: serde_json::Value = res.json().expect("Failed to parse response");
    assert_eq!(body["status"], "UP", "Expected status UP in response");
    assert!(
        body["checks"].is_array(),
        "Expected checks array in response"
    );

    // Verify all checks passed
    let checks = body["checks"]
        .as_array()
        .expect("checks should be an array");
    assert!(!checks.is_empty(), "Expected at least one check");

    for check in checks {
        assert_eq!(
            check["status"], "UP",
            "Expected all checks to have status UP, but got: {:#?}",
            check
        );
        assert!(
            check["name"].is_string(),
            "Expected check to have a name field"
        );
    }
}

#[test]
fn test_health_success() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Expected 200 OK for /health, got: {:#?}",
        res
    );

    let body: serde_json::Value = res.json().expect("Failed to parse response");
    assert_eq!(body["status"], "UP", "Expected status UP in response");
    assert!(
        body["checks"].is_array(),
        "Expected checks array in response"
    );

    // Verify all checks passed
    let checks = body["checks"]
        .as_array()
        .expect("checks should be an array");
    assert!(!checks.is_empty(), "Expected at least one check");

    for check in checks {
        assert_eq!(
            check["status"], "UP",
            "Expected all checks to have status UP, but got: {:#?}",
            check
        );
    }
}

#[test]
fn test_health_response_structure_live() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health/live", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    let body: serde_json::Value = res.json().expect("Failed to parse response");

    // Verify structure
    assert!(body["status"].is_string(), "status should be a string");
    assert!(body["checks"].is_array(), "checks should be an array");

    let checks = body["checks"].as_array().unwrap();
    for check in checks {
        assert!(check["name"].is_string(), "check name should be a string");
        assert!(
            check["status"].is_string(),
            "check status should be a string"
        );
        // details can be null or a string
        if !check["details"].is_null() {
            assert!(
                check["details"].is_string(),
                "check details should be a string or null"
            );
        }
    }
}

#[test]
fn test_health_response_structure_ready() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health/ready", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    let body: serde_json::Value = res.json().expect("Failed to parse response");

    // Verify structure
    assert!(body["status"].is_string(), "status should be a string");
    assert!(body["checks"].is_array(), "checks should be an array");

    let checks = body["checks"].as_array().unwrap();
    for check in checks {
        assert!(check["name"].is_string(), "check name should be a string");
        assert!(
            check["status"].is_string(),
            "check status should be a string"
        );
        // details can be null or a string
        if !check["details"].is_null() {
            assert!(
                check["details"].is_string(),
                "check details should be a string or null"
            );
        }
    }
}

#[test]
fn test_health_checks_include_database_and_auth() {
    let (_client, base_url) = get_client();
    let res = _client
        .get(format!("{}/health/ready", base_url))
        .header("x-forwarded-for", "127.0.0.1")
        .send()
        .expect("Failed to send request");

    let body: serde_json::Value = res.json().expect("Failed to parse response");
    let checks = body["checks"]
        .as_array()
        .expect("checks should be an array");

    let check_names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();

    assert!(
        check_names.contains(&"database"),
        "Expected 'database' check to be present. Found checks: {:?}",
        check_names
    );
    assert!(
        check_names.contains(&"auth_server"),
        "Expected 'auth_server' check to be present. Found checks: {:?}",
        check_names
    );
}
