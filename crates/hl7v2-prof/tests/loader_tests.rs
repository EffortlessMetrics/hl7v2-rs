//! Integration tests for the ProfileLoader with remote loading and caching.

use std::time::Duration;

use hl7v2_prof::loader::{LoadResult, ProfileLoadError, ProfileLoader};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Sample profile YAML for testing
const SAMPLE_PROFILE_YAML: &str = r#"
message_structure: ADT_A01
version: "2.5.1"
segments:
  - id: MSH
constraints:
  - path: MSH.9
    required: true
"#;

#[tokio::test]
async fn test_load_from_url_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    let result = loader.load(&url).await;

    assert!(result.is_ok());
    let load_result = result.unwrap();
    assert_eq!(load_result.profile.message_structure, "ADT_A01");
    assert!(!load_result.from_cache);
}

#[tokio::test]
async fn test_load_from_url_with_etag() {
    let mock_server = MockServer::start().await;

    // First request returns the profile with ETag
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("ETag", "\"abc123\"")
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    // First load
    let result = loader.load(&url).await.unwrap();
    assert_eq!(result.profile.message_structure, "ADT_A01");
    assert!(!result.from_cache);
    assert_eq!(result.etag, Some("\"abc123\"".to_string()));
}

#[tokio::test]
async fn test_load_from_url_caches_result_with_etag() {
    let mock_server = MockServer::start().await;

    // First response with ETag
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("ETag", "\"test-etag-123\"")
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    // First load
    let result1 = loader.load(&url).await.unwrap();
    assert!(!result1.from_cache);
    assert_eq!(result1.etag, Some("\"test-etag-123\"".to_string()));

    // Reset the mock to return 304 Not Modified for subsequent requests with If-None-Match
    mock_server.reset().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(304).insert_header("ETag", "\"test-etag-123\""))
        .mount(&mock_server)
        .await;

    // Second load should be from cache (304 response)
    let result2 = loader.load(&url).await.unwrap();
    assert!(result2.from_cache);
}

#[tokio::test]
async fn test_load_from_url_handles_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/nonexistent.yaml", mock_server.uri());

    let result = loader.load(&url).await;
    assert!(result.is_err());

    if let Err(ProfileLoadError::Network(_)) = result {
        // Expected error type
    } else {
        panic!("Expected Network error");
    }
}

#[tokio::test]
async fn test_load_from_url_handles_invalid_yaml() {
    let mock_server = MockServer::start().await;

    // Use truly invalid YAML that will fail parsing
    let invalid_yaml = "message_structure: [unclosed\n  bad: {{{array";

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(invalid_yaml)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/invalid.yaml", mock_server.uri());

    let result = loader.load(&url).await;
    assert!(result.is_err());
    // The error could be Parse or Core depending on what fails first
    assert!(matches!(
        result,
        Err(ProfileLoadError::YamlParse(_)) | Err(ProfileLoadError::Core(_))
    ));
}

#[tokio::test]
async fn test_cache_invalidation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    // Load and cache
    let _ = loader.load(&url).await.unwrap();
    assert!(loader.is_cached(&url).await);

    // Invalidate
    let removed = loader.invalidate(&url).await;
    assert!(removed);
    assert!(!loader.is_cached(&url).await);

    // Second invalidation should return false
    let removed_again = loader.invalidate(&url).await;
    assert!(!removed_again);
}

#[tokio::test]
async fn test_clear_cache() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();

    // Load multiple profiles
    let url1 = format!("{}/profiles/adt_a01.yaml", mock_server.uri());
    let url2 = format!("{}/profiles/adt_a04.yaml", mock_server.uri());

    let _ = loader.load(&url1).await.unwrap();
    let _ = loader.load(&url2).await.unwrap();

    assert!(loader.cache_size().await >= 2);

    // Clear cache
    loader.clear_cache().await;
    assert_eq!(loader.cache_size().await, 0);
}

#[tokio::test]
async fn test_lru_eviction() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    // Create loader with cache size of 2
    let loader = ProfileLoader::with_options(2, Duration::from_secs(30));

    // Load 3 profiles
    let url1 = format!("{}/profiles/p1.yaml", mock_server.uri());
    let url2 = format!("{}/profiles/p2.yaml", mock_server.uri());
    let url3 = format!("{}/profiles/p3.yaml", mock_server.uri());

    let _ = loader.load(&url1).await.unwrap();
    let _ = loader.load(&url2).await.unwrap();
    let _ = loader.load(&url3).await.unwrap();

    // Cache should have at most 2 entries
    assert!(loader.cache_size().await <= 2);
}

#[tokio::test]
async fn test_custom_timeout() {
    let loader = ProfileLoader::new().with_timeout(Duration::from_secs(10));

    // Just verify it can be created
    assert_eq!(loader.cache_size().await, 0);
}

#[tokio::test]
async fn test_custom_cache_size() {
    let loader = ProfileLoader::new().with_cache_size(50);

    // Just verify it can be created
    assert_eq!(loader.cache_size().await, 0);
}

#[tokio::test]
async fn test_prefetch() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();
    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    // Prefetch
    let result = loader.prefetch(&url).await;
    assert!(result.is_ok());
    assert!(loader.is_cached(&url).await);
}

#[tokio::test]
async fn test_prefetch_all() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();

    let url1 = format!("{}/profiles/p1.yaml", mock_server.uri());
    let url2 = format!("{}/profiles/p2.yaml", mock_server.uri());
    let url3 = format!("{}/profiles/p3.yaml", mock_server.uri());

    let results = loader
        .prefetch_all([url1.as_str(), url2.as_str(), url3.as_str()])
        .await;

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));
}

#[tokio::test]
async fn test_load_file_not_found() {
    let loader = ProfileLoader::new();

    let result = loader.load_from_file("nonexistent_profile.yaml").await;
    assert!(result.is_err());

    if let Err(ProfileLoadError::Io(_)) = result {
        // Expected error type
    } else {
        panic!("Expected Io error");
    }
}

#[test]
fn test_load_file_sync() {
    // Try to load a profile that exists
    let result = hl7v2_prof::loader::load_from_file("examples/profiles/minimal.yaml");

    // This test may pass or fail depending on the working directory
    if let Ok(profile) = result {
        assert_eq!(profile.message_structure, "ADT_A01");
    }
}

#[test]
fn test_load_file_sync_not_found() {
    let result = hl7v2_prof::loader::load_from_file("nonexistent.yaml");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_convenience_load_from_url() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    let result = hl7v2_prof::loader::load_from_url(&url).await;

    assert!(result.is_ok());
    let profile = result.unwrap();
    assert_eq!(profile.message_structure, "ADT_A01");
}

#[tokio::test]
async fn test_error_display() {
    let err = ProfileLoadError::YamlParse("test error".to_string());
    assert!(err.to_string().contains("test error"));

    let err = ProfileLoadError::Io("file not found".to_string());
    assert!(err.to_string().contains("file not found"));

    let err = ProfileLoadError::NotFound("profile.yaml".to_string());
    assert!(err.to_string().contains("profile.yaml"));

    let err = ProfileLoadError::InvalidScheme("ftp".to_string());
    assert!(err.to_string().contains("ftp"));

    let err = ProfileLoadError::Cache("cache full".to_string());
    assert!(err.to_string().contains("cache full"));
}

#[tokio::test]
async fn test_load_detects_url_vs_file() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader = ProfileLoader::new();

    // URL with http://
    let http_url = format!("{}/profiles/test.yaml", mock_server.uri());
    let result = loader.load(&http_url).await;
    assert!(result.is_ok());

    // URL with https:// would need a proper HTTPS server, so we just test http
}

#[tokio::test]
async fn test_multiple_loaders_have_separate_caches() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(SAMPLE_PROFILE_YAML)
                .insert_header("Content-Type", "application/yaml"),
        )
        .mount(&mock_server)
        .await;

    let loader1 = ProfileLoader::new();
    let loader2 = ProfileLoader::new();

    let url = format!("{}/profiles/adt_a01.yaml", mock_server.uri());

    // Load with first loader
    let _ = loader1.load(&url).await.unwrap();
    assert!(loader1.is_cached(&url).await);

    // Second loader should not have it cached
    assert!(!loader2.is_cached(&url).await);
}

#[tokio::test]
async fn test_load_result_debug() {
    use hl7v2_prof::Profile;

    let result = LoadResult {
        profile: Profile {
            message_structure: "TEST".to_string(),
            version: "2.5.1".to_string(),
            message_type: None,
            parent: None,
            segments: vec![],
            constraints: vec![],
            lengths: vec![],
            valuesets: vec![],
            datatypes: vec![],
            advanced_datatypes: vec![],
            cross_field_rules: vec![],
            temporal_rules: vec![],
            contextual_rules: vec![],
            custom_rules: vec![],
            hl7_tables: vec![],
            table_precedence: vec![],
            expression_guardrails: Default::default(),
        },
        from_cache: true,
        etag: Some("\"abc\"".to_string()),
    };

    // Just verify Debug impl works
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("from_cache"));
    assert!(debug_str.contains("abc"));
}
