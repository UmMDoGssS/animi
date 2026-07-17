use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};
use tempfile::tempdir;

use animi_lib::backups::ImportSessionStore;
use animi_lib::commands::backups::{BackupDestinationPicker, BackupSourcePicker};
use animi_lib::database::{Database, DatabasePaths};
use animi_lib::register_commands;

fn build_test_app(
    database: Database,
    destination_picker: BackupDestinationPicker,
    source_picker: BackupSourcePicker,
) -> App<MockRuntime> {
    register_commands(
        mock_builder()
            .manage(database)
            .manage(destination_picker)
            .manage(source_picker)
            .manage(ImportSessionStore::default()),
    )
    .build(mock_context(noop_assets()))
    .expect("mock Tauri application")
}

fn main_webview(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .expect("mock main webview")
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: command.to_owned(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(any(windows, target_os = "android")) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .expect("valid mock IPC URL"),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    )
    .map(|response| {
        response
            .deserialize::<Value>()
            .expect("JSON command response")
    })
}

fn invoke_ok(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Value {
    invoke(webview, command, body)
        .unwrap_or_else(|error| panic!("{command} returned an unexpected error: {error}"))
}

fn invoke_error(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Value {
    match invoke(webview, command, body) {
        Ok(value) => panic!("{command} unexpectedly succeeded: {value}"),
        Err(error) => error,
    }
}

fn library_request(progress: i64) -> Value {
    json!({
        "request": {
            "anilistId": 154_587,
            "mediaKind": "ANIME",
            "status": "WATCHING",
            "progress": progress,
            "score": 9.0,
            "notes": "IPC recovery fixture"
        }
    })
}

#[test]
fn export_backup_crosses_the_real_camel_case_ipc_boundary() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let export_root = tempdir().expect("temporary export root");
    let destination = export_root.path().join("ipc-backup.json");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(Some(destination.clone())),
        BackupSourcePicker::fixed_path(None),
    );
    let webview = main_webview(&app);

    invoke_ok(
        &webview,
        "set_theme_preference",
        json!({ "preference": "MACCHIATO" }),
    );
    invoke_ok(&webview, "upsert_library_entry", library_request(2));
    invoke_ok(&webview, "upsert_library_entry", library_request(3));

    let response = invoke_ok(&webview, "export_backup", json!({}));

    assert_eq!(response["schemaVersion"], 1);
    assert_eq!(response["settingsCount"], 1);
    assert_eq!(response["libraryEntryCount"], 1);
    assert_eq!(response["watchActivityCount"], 1);
    assert!(response.get("schema_version").is_none());

    let backup: Value =
        serde_json::from_str(&std::fs::read_to_string(destination).expect("exported backup file"))
            .expect("valid backup JSON");
    assert_eq!(backup["settings"][0]["value"], "MACCHIATO");
    assert_eq!(backup["libraryEntries"][0]["progress"], 3);
    assert_eq!(backup["libraryEntries"][0]["activityProgress"], 3);
    assert_eq!(backup["watchActivity"][0]["progressDelta"], 1);
    assert!(backup["watchActivity"][0].get("eventId").is_some());
}

#[test]
fn cancelling_the_native_destination_picker_returns_null_over_ipc() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(None),
        BackupSourcePicker::fixed_path(None),
    );
    let webview = main_webview(&app);

    assert_eq!(invoke_ok(&webview, "export_backup", json!({})), Value::Null);
}

#[test]
fn invalid_picker_results_keep_the_structured_ipc_error_contract() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(Some("relative-backup.json".into())),
        BackupSourcePicker::fixed_path(None),
    );
    let webview = main_webview(&app);

    let error = invoke_error(&webview, "export_backup", json!({}));
    assert_eq!(error["code"], "BACKUP_PATH_INVALID");
    assert_eq!(error["retryable"], false);
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
}

#[test]
fn export_clear_preview_and_restore_cross_the_real_command_boundary() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let export_root = tempdir().expect("temporary export root");
    let destination = export_root.path().join("recovery-flow.json");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(Some(destination.clone())),
        BackupSourcePicker::fixed_path(Some(destination.clone())),
    );
    let webview = main_webview(&app);

    invoke_ok(
        &webview,
        "set_theme_preference",
        json!({ "preference": "MACCHIATO" }),
    );
    invoke_ok(&webview, "upsert_library_entry", library_request(2));
    invoke_ok(&webview, "upsert_library_entry", library_request(4));
    let exported = invoke_ok(&webview, "export_backup", json!({}));
    assert_eq!(exported["libraryEntryCount"], 1);
    assert_eq!(exported["watchActivityCount"], 1);

    let confirmation_error = invoke_error(
        &webview,
        "clear_local_data",
        json!({ "request": { "confirmation": "clear" } }),
    );
    assert_eq!(confirmation_error["code"], "CLEAR_CONFIRMATION_REQUIRED");
    assert_eq!(
        invoke_ok(&webview, "list_library", json!({}))
            .as_array()
            .expect("library")
            .len(),
        1
    );

    let cleared = invoke_ok(
        &webview,
        "clear_local_data",
        json!({ "request": { "confirmation": "CLEAR MY LOCAL DATA" } }),
    );
    assert_eq!(cleared["libraryEntriesRemoved"], 1);
    assert_eq!(cleared["watchActivityRemoved"], 1);
    assert_eq!(invoke_ok(&webview, "list_library", json!({})), json!([]));
    assert_eq!(
        invoke_ok(&webview, "get_theme_preference", json!({})),
        "SYSTEM"
    );
    let overview = invoke_ok(&webview, "get_settings_overview", json!({}));
    assert_eq!(overview["cachedMediaCount"], 0);
    assert_eq!(overview["latestCacheUpdateAt"], Value::Null);

    let preview = invoke_ok(&webview, "preview_import", json!({}));
    assert_eq!(preview["canApply"], true);
    assert_eq!(preview["changes"]["libraryAdded"], 1);
    assert_eq!(preview["changes"]["activityAdded"], 1);
    assert_eq!(preview["preservesUnrelatedLocalEntries"], true);
    assert!(
        preview["validationErrors"]
            .as_array()
            .expect("validation errors")
            .is_empty()
    );
    let token = preview["importToken"]
        .as_str()
        .expect("opaque import token");

    let restored = invoke_ok(
        &webview,
        "import_backup",
        json!({ "request": { "importToken": token } }),
    );
    assert_eq!(restored["changes"], preview["changes"]);
    let library = invoke_ok(&webview, "list_library", json!({}));
    assert_eq!(library.as_array().expect("restored library").len(), 1);
    assert_eq!(library[0]["entry"]["progress"], 4);
    assert_eq!(
        invoke_ok(&webview, "get_theme_preference", json!({})),
        "MACCHIATO"
    );

    let repeated_preview = invoke_ok(&webview, "preview_import", json!({}));
    assert_eq!(repeated_preview["changes"]["activityAdded"], 0);
    assert_eq!(repeated_preview["changes"]["activityUnchanged"], 1);
}

#[test]
fn invalid_backup_preview_returns_actionable_validation_without_a_token() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let import_root = tempdir().expect("temporary import root");
    let source = import_root.path().join("invalid.json");
    std::fs::write(&source, r#"{"schemaVersion":99}"#).expect("invalid fixture");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(None),
        BackupSourcePicker::fixed_path(Some(source)),
    );
    let webview = main_webview(&app);

    let preview = invoke_ok(&webview, "preview_import", json!({}));
    assert_eq!(preview["canApply"], false);
    assert!(preview["importToken"].is_null());
    assert_eq!(
        preview["validationErrors"][0]["code"],
        "BACKUP_SCHEMA_VERSION_UNSUPPORTED"
    );
    assert_eq!(invoke_ok(&webview, "list_library", json!({})), json!([]));
}

#[test]
fn cancelling_the_native_source_picker_returns_null_without_changes() {
    let database_root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(database_root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let app = build_test_app(
        database,
        BackupDestinationPicker::fixed_path(None),
        BackupSourcePicker::fixed_path(None),
    );
    let webview = main_webview(&app);

    assert_eq!(
        invoke_ok(&webview, "preview_import", json!({})),
        Value::Null
    );
    assert_eq!(invoke_ok(&webview, "list_library", json!({})), json!([]));
}
