use vessel::config::VesselConfig;

#[test]
fn default_config_has_expected_ports() {
    let config = VesselConfig::default();
    assert_eq!(config.server.port, 3458);
    assert_eq!(config.hivemind.port, 3456);
    assert_eq!(config.storage.path, "~/.vessel/vessel.db");
}

#[test]
fn db_path_expands_tilde_to_home_dir() {
    let config = VesselConfig::default();
    let path = config.db_path();
    let home = dirs::home_dir().unwrap_or_default();
    assert!(path.starts_with(&home));
    assert!(path.ends_with(".vessel/vessel.db"));
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
    assert_eq!(config.storage.path, "/tmp/custom.db");
    assert_eq!(config.hivemind.port, 1234);

    unsafe {
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
