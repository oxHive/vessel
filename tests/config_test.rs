use vessel::config::{VesselConfig, resolve_default_db_path};

#[test]
fn default_config_has_expected_ports_and_no_explicit_db_path() {
    let config = VesselConfig::default();
    assert_eq!(config.server.port, 3458);
    assert_eq!(config.hivemind.port, 3456);
    assert_eq!(config.storage.path, None);
}

#[test]
fn explicit_storage_path_wins_and_expands_tilde() {
    let mut config = VesselConfig::default();
    config.storage.path = Some("~/custom/vessel.db".into());
    let home = dirs::home_dir().unwrap_or_default();
    assert_eq!(config.db_path(), home.join("custom/vessel.db"));
}

#[test]
fn fresh_install_resolves_to_xdg_data_dir() {
    let data = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let path = resolve_default_db_path(Some(data.path().to_path_buf()), home.path().to_path_buf());
    assert_eq!(path, data.path().join("vessel").join("vessel.db"));
}

#[test]
fn legacy_db_is_used_when_xdg_db_missing() {
    let data = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let legacy_dir = home.path().join(".vessel");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(legacy_dir.join("vessel.db"), b"").unwrap();

    let path = resolve_default_db_path(Some(data.path().to_path_buf()), home.path().to_path_buf());
    assert_eq!(path, legacy_dir.join("vessel.db"));
}

#[test]
fn xdg_db_wins_when_both_exist() {
    let data = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let xdg_dir = data.path().join("vessel");
    std::fs::create_dir_all(&xdg_dir).unwrap();
    std::fs::write(xdg_dir.join("vessel.db"), b"").unwrap();
    let legacy_dir = home.path().join(".vessel");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    std::fs::write(legacy_dir.join("vessel.db"), b"").unwrap();

    let path = resolve_default_db_path(Some(data.path().to_path_buf()), home.path().to_path_buf());
    assert_eq!(path, xdg_dir.join("vessel.db"));
}

#[test]
fn missing_data_dir_falls_back_to_dot_local_share() {
    let home = tempfile::tempdir().unwrap();
    let path = resolve_default_db_path(None, home.path().to_path_buf());
    assert_eq!(
        path,
        home.path()
            .join(".local")
            .join("share")
            .join("vessel")
            .join("vessel.db")
    );
}

// Both scenarios below mutate the process-global XDG_CONFIG_HOME env var,
// so they run in a single test function to avoid races with cargo test's
// default multi-threaded execution within this binary.
#[test]
fn load_reads_or_falls_back_to_config_file() {
    let empty_dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", empty_dir.path());
    }
    let config = VesselConfig::load().unwrap();
    assert_eq!(config.server.port, 3458);

    let populated_dir = tempfile::tempdir().unwrap();
    let vessel_dir = populated_dir.path().join("vessel");
    std::fs::create_dir_all(&vessel_dir).unwrap();
    std::fs::write(
        vessel_dir.join("vessel.toml"),
        "[server]\nport = 9999\n[storage]\npath = \"/tmp/custom.db\"\n[hivemind]\nport = 1234\n",
    )
    .unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", populated_dir.path());
    }
    let config = VesselConfig::load().unwrap();
    assert_eq!(config.server.port, 9999);
    assert_eq!(config.storage.path, Some("/tmp/custom.db".to_string()));
    assert_eq!(config.hivemind.port, 1234);

    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
