use super::*;

fn local_entry(id: &str) -> AppLauncherEntry {
    AppLauncherEntry {
        id: id.to_string(),
        enabled: true,
        name: "Tool".to_string(),
        kind: AppLauncherEntryKind::LocalApp,
        scope: AppLauncherScope::All,
        target: "C:\\Tools\\Tool.exe".to_string(),
        args: String::new(),
        launch_delay_seconds: 0,
        run_policy: AppLauncherRunPolicy::Always,
        stop_policy: AppLauncherStopPolicy::KeepRunning,
        run_as_administrator: false,
        process_name: None,
        working_directory: None,
    }
}

#[test]
fn app_launcher_scope_filter_matches_desktop_and_vr() {
    assert!(scope_matches(AppLauncherScope::All, false));
    assert!(scope_matches(AppLauncherScope::All, true));
    assert!(scope_matches(AppLauncherScope::Desktop, false));
    assert!(!scope_matches(AppLauncherScope::Desktop, true));
    assert!(!scope_matches(AppLauncherScope::Vr, false));
    assert!(scope_matches(AppLauncherScope::Vr, true));
}

#[test]
fn app_launcher_sanitizes_steam_close_policy() {
    let mut entry = local_entry("steam");
    entry.kind = AppLauncherEntryKind::SteamApp;
    entry.target = "438100".to_string();
    entry.args = "--ignored".to_string();
    entry.working_directory = Some("C:\\Temp".to_string());
    entry.stop_policy = AppLauncherStopPolicy::CloseByVrcx;

    let entries = normalize_app_launcher_entries(vec![entry]);
    assert_eq!(entries[0].stop_policy, AppLauncherStopPolicy::KeepRunning);
    assert!(entries[0].args.is_empty());
    assert_eq!(entries[0].working_directory, None);
}

#[test]
fn app_launcher_untracked_close_fallback_excludes_steam_client() {
    let mut cacher_entry = local_entry("cacher");
    cacher_entry.target = if cfg!(windows) {
        "D:\\SteamLibrary\\steamapps\\common\\VRCVideoCacher\\VRCVideoCacher.exe"
    } else {
        "/home/user/.local/share/Steam/steamapps/common/VRCVideoCacher/VRCVideoCacher"
    }
    .to_string();
    let cacher_run = new_run("run-cacher", &cacher_entry, false);

    assert_eq!(
        process_name_from_target_for_platform(
            r"D:\SteamLibrary\steamapps\common\VRCVideoCacher\VRCVideoCacher.exe",
            true
        )
        .as_deref(),
        Some("vrcvideocacher")
    );
    assert_eq!(
        process_name_for_run(&cacher_run).as_deref(),
        Some("vrcvideocacher")
    );
    assert!(should_close_untracked_matching_processes(
        process_name_for_run(&cacher_run).as_deref(),
        &[],
        &[]
    ));
    assert!(!should_close_untracked_matching_processes(
        process_name_for_run(&cacher_run).as_deref(),
        &[123],
        &[]
    ));
    assert!(!should_close_untracked_matching_processes(
        process_name_for_run(&cacher_run).as_deref(),
        &[],
        &[123]
    ));

    let mut steam_entry = local_entry("steam-local");
    steam_entry.target = if cfg!(windows) {
        "C:\\Program Files (x86)\\Steam\\steam.exe"
    } else {
        "/home/user/.local/share/Steam/steam"
    }
    .to_string();
    let steam_run = new_run("run-steam", &steam_entry, false);

    assert_eq!(
        process_name_from_target_for_platform("C:\\Program Files (x86)\\Steam\\steam.exe", true)
            .as_deref(),
        Some("steam")
    );
    assert_eq!(process_name_for_run(&steam_run).as_deref(), Some("steam"));
    assert!(!should_close_untracked_matching_processes(
        process_name_for_run(&steam_run).as_deref(),
        &[],
        &[]
    ));

    assert_eq!(
        process_name_from_target_for_platform("/home/user/.local/share/Steam/steam.sh", false)
            .as_deref(),
        Some("steam.sh")
    );
    assert!(!should_close_untracked_matching_processes(
        Some("steam.sh"),
        &[],
        &[]
    ));
}

#[test]
fn app_launcher_untracked_close_fallback_requires_matching_exe_path() {
    let windows_target = r"D:\SteamLibrary\steamapps\common\VRCVideoCacher\VRCVideoCacher.exe";
    assert_eq!(
        normalized_process_path_for_platform(
            r"\\?\D:/SteamLibrary/steamapps/common/VRCVideoCacher/VRCVideoCacher.exe",
            true
        ),
        normalized_process_path_for_platform(windows_target, true)
    );
    assert_eq!(
        normalized_process_path_for_platform(
            r"d:\steamlibrary\STEAMAPPS\common\VRCVideoCacher\VRCVideoCacher.exe",
            true
        ),
        normalized_process_path_for_platform(windows_target, true)
    );
    assert_ne!(
        normalized_process_path_for_platform(r"C:\Other\VRCVideoCacher.exe", true),
        normalized_process_path_for_platform(windows_target, true)
    );

    let linux_target = "/home/User/.local/share/Steam/steamapps/common/Tool/Tool.AppImage";
    assert_eq!(
        normalized_process_path_for_platform(linux_target, false),
        normalized_process_path_for_platform(linux_target, false)
    );
    assert_ne!(
        normalized_process_path_for_platform(
            "/home/user/.local/share/Steam/steamapps/common/Tool/Tool.AppImage",
            false
        ),
        normalized_process_path_for_platform(linux_target, false)
    );
}

#[test]
fn app_launcher_omits_empty_args_from_serialized_entries() {
    let entry = normalize_app_launcher_entries(vec![local_entry("local")])
        .into_iter()
        .next()
        .unwrap();
    let value = serde_json::to_value(entry).unwrap();

    assert!(value.get("args").is_none());
}

#[test]
fn app_launcher_normalization_makes_duplicate_ids_unique() {
    let first = local_entry("same");
    let second = local_entry("same");
    let entries = normalize_app_launcher_entries(vec![first, second]);

    assert_eq!(entries[0].id, "same");
    assert_eq!(entries[1].id, "same-1");
}

#[test]
fn app_launcher_json_invalid_config_falls_back_to_empty_entries() {
    let entries = deserialize_app_launcher_entries(serde_json::json!({ "bad": true }));
    assert!(entries.is_empty());
}

#[test]
fn app_launcher_json_round_trips_entries() {
    let entry = normalize_app_launcher_entries(vec![local_entry("local")])
        .into_iter()
        .next()
        .unwrap();
    let value = serde_json::to_value(vec![entry.clone()]).unwrap();
    assert_eq!(deserialize_app_launcher_entries(value), vec![entry]);
}

#[test]
fn app_launcher_legacy_json_defaults_run_as_administrator_to_false() {
    let entries = deserialize_app_launcher_entries(serde_json::json!([{
        "id": "legacy",
        "enabled": true,
        "name": "Legacy Tool",
        "kind": "localApp",
        "scope": "all",
        "target": "C:\\Tools\\Legacy.exe",
        "launchDelaySeconds": 0,
        "runPolicy": "always",
        "stopPolicy": "keepRunning"
    }]));

    assert_eq!(entries.len(), 1);
    assert!(!entries[0].run_as_administrator);
}

#[test]
#[cfg(windows)]
fn app_launcher_picks_windows_exe_with_parent_working_directory() {
    let picked = picked_app_launcher_target(r"C:\Tools\Overlay\Overlay.exe").unwrap();

    assert_eq!(
        picked.working_directory.as_deref(),
        Some(r"C:\Tools\Overlay")
    );
}

#[test]
fn app_launcher_preserves_explicit_working_directory() {
    let mut entry = local_entry("local");
    entry.working_directory = Some(r"C:\Custom".to_string());

    let entries = normalize_app_launcher_entries(vec![entry]);

    assert_eq!(entries[0].working_directory.as_deref(), Some(r"C:\Custom"));
}

#[test]
#[cfg(windows)]
fn app_launcher_empty_working_directory_uses_exe_parent() {
    let mut entry = local_entry("local");
    entry.target = r"C:\Tools\Overlay\Overlay.exe".to_string();
    entry.working_directory = Some("  ".to_string());

    let entries = normalize_app_launcher_entries(vec![entry]);

    assert_eq!(
        entries[0].working_directory.as_deref(),
        Some(r"C:\Tools\Overlay")
    );
}

#[test]
fn app_launcher_shortcut_keeps_shell_working_directory() {
    let picked = picked_app_launcher_target(r"C:\Tools\Overlay.lnk").unwrap();

    assert_eq!(picked.working_directory, None);
}

#[test]
fn app_launcher_steam_entry_disables_run_as_administrator() {
    let mut entry = local_entry("steam");
    entry.kind = AppLauncherEntryKind::SteamApp;
    entry.target = "438100".to_string();
    entry.run_as_administrator = true;

    let entries = normalize_app_launcher_entries(vec![entry]);

    assert!(!entries[0].run_as_administrator);
}

#[test]
fn app_launcher_elevated_entry_keeps_running_after_vrchat() {
    let mut entry = local_entry("elevated");
    entry.run_as_administrator = true;
    entry.stop_policy = AppLauncherStopPolicy::CloseByVrcx;

    let entries = normalize_app_launcher_entries(vec![entry]);

    assert_eq!(entries[0].stop_policy, AppLauncherStopPolicy::KeepRunning);
}

#[test]
fn app_launcher_windows_launch_strategy_elevates_only_explicit_entries() {
    let direct = local_entry("direct");
    assert_eq!(
        local_launch_strategy(&direct, true),
        LocalLaunchStrategy::Direct
    );

    let mut elevated = local_entry("elevated");
    elevated.run_as_administrator = true;
    assert_eq!(
        local_launch_strategy(&elevated, true),
        LocalLaunchStrategy::ShellExecute(ShellExecuteVerb::RunAs)
    );

    let mut shortcut = local_entry("shortcut");
    shortcut.target = r"C:\Tools\Tool.lnk".to_string();
    assert_eq!(
        local_launch_strategy(&shortcut, true),
        LocalLaunchStrategy::ShellExecute(ShellExecuteVerb::Open)
    );
    assert_eq!(
        local_launch_strategy(&elevated, false),
        LocalLaunchStrategy::Direct
    );
}

#[test]
fn app_launcher_launch_failure_preserves_os_error_code() {
    let elevation = LaunchFailure::from_io("Tool.exe", std::io::Error::from_raw_os_error(740));
    assert_eq!(elevation.os_error_code, Some(740));

    let cancelled = LaunchFailure::from_io("Tool.exe", std::io::Error::from_raw_os_error(1223));
    assert_eq!(cancelled.os_error_code, Some(1223));

    let denied = LaunchFailure::from_io("Tool.exe", std::io::Error::from_raw_os_error(5));
    assert_eq!(denied.os_error_code, Some(5));
}

#[test]
fn app_launcher_shell_pid_failure_preserves_os_error_code() {
    let failure =
        tracked_shell_process_id("Tool.exe", 0, std::io::Error::from_raw_os_error(6)).unwrap_err();

    assert_eq!(failure.os_error_code, Some(6));
    assert_eq!(
        tracked_shell_process_id("Tool.exe", 42, std::io::Error::from_raw_os_error(6)).unwrap(),
        Some(42)
    );
}

#[test]
fn app_launcher_run_policy_skips_when_process_is_running() {
    let mut entry = local_entry("local");
    entry.run_policy = AppLauncherRunPolicy::SkipIfRunning;
    entry.process_name = Some("Tool.exe".to_string());

    assert!(should_skip_entry(&entry, |name| name == "tool"));
    assert!(!should_skip_entry(&entry, |name| name == "other"));
}

#[test]
fn app_launcher_always_policy_does_not_skip_existing_process() {
    let entry = local_entry("local");
    assert!(!should_skip_entry(&entry, |_| true));
}

#[test]
fn app_launcher_stop_policy_uses_only_tracked_pids() {
    let mut run = new_run("run", &local_entry("local"), false);
    run.root_pid = Some(42);
    run.tracked_pids = vec![10, 42, 99];
    assert_eq!(tracked_stop_pids(&run), vec![10, 42, 99]);
}

#[test]
fn app_launcher_steam_url_uses_launch_scheme() {
    assert_eq!(
        steam_launch_url("438100"),
        Some("steam://launch/438100".to_string())
    );
    assert_eq!(steam_launch_url(""), None);
}

#[test]
fn app_launcher_picks_steam_url_shortcut() {
    let path = std::env::temp_dir().join(format!("vrcx-steam-shortcut-{}.url", now_timestamp()));
    std::fs::write(&path, "[InternetShortcut]\nURL=steam://rungameid/438100\n").unwrap();

    let picked = picked_app_launcher_target(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(picked.kind, AppLauncherEntryKind::SteamApp);
    assert_eq!(picked.target, "438100");
    assert_eq!(picked.process_name, None);
}

#[test]
fn app_launcher_picks_local_exe_with_process_name() {
    let picked = picked_app_launcher_target("Overlay.exe").unwrap();

    assert_eq!(picked.kind, AppLauncherEntryKind::LocalApp);
    assert_eq!(picked.name, "Overlay");
    assert_eq!(picked.process_name, Some("Overlay".to_string()));
    assert_eq!(picked.working_directory, None);
}

#[test]
fn app_launcher_picks_shortcut_without_guessing_process_name() {
    let picked = picked_app_launcher_target("Overlay.lnk").unwrap();

    assert_eq!(picked.kind, AppLauncherEntryKind::LocalApp);
    assert_eq!(picked.name, "Overlay");
    assert_eq!(picked.process_name, None);
    assert_eq!(picked.working_directory, None);
}

#[test]
fn app_launcher_shortcut_skip_policy_requires_explicit_process_name() {
    let mut entry = local_entry("shortcut");
    entry.target = "Overlay.lnk".to_string();
    entry.run_policy = AppLauncherRunPolicy::SkipIfRunning;

    assert!(!should_skip_entry(&entry, |_| true));
}

#[test]
fn app_launcher_entries_set_applies_to_active_session() {
    let manager = AutoAppLaunchManager::new(true, Vec::new());
    manager.on_game_started(false);

    let snapshot = manager.set_entries(vec![local_entry("local")]);
    assert_eq!(snapshot.active_session.unwrap().runs.len(), 1);

    let snapshot = manager.set_entries(Vec::new());
    assert!(snapshot.active_session.unwrap().runs.is_empty());
}

#[test]
fn app_launcher_disabling_entry_preserves_started_session_run_tracking() {
    let manager = AutoAppLaunchManager::new(true, vec![local_entry("local")]);
    manager.on_game_started(false);
    let pid = std::process::id();
    manager.set_active_run_tracking_for_test("local", pid);

    let mut disabled = local_entry("local");
    disabled.enabled = false;
    let snapshot = manager.set_entries(vec![disabled]);
    let runs = snapshot.active_session.unwrap().runs;

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, AppLauncherRunStatus::Running);
    assert_eq!(runs[0].tracked_pids, vec![pid]);
}

#[test]
fn app_launcher_editing_entry_preserves_old_run_tracking_while_starting_new_config() {
    let manager = AutoAppLaunchManager::new(true, vec![local_entry("local")]);
    manager.on_game_started(false);
    let pid = std::process::id();
    manager.set_active_run_tracking_for_test("local", pid);

    let mut edited = local_entry("local");
    edited.target = "C:\\Tools\\Edited.exe".to_string();
    let snapshot = manager.set_entries(vec![edited]);
    let runs = snapshot.active_session.unwrap().runs;

    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].status, AppLauncherRunStatus::Running);
    assert_eq!(runs[0].tracked_pids, vec![pid]);
    assert_eq!(runs[1].target, "C:\\Tools\\Edited.exe");
}

#[test]
fn app_launcher_stop_failure_keeps_run_tracked_and_not_stopped() {
    let mut run = new_run("run", &local_entry("local"), false);
    run.status = AppLauncherRunStatus::Running;
    run.root_pid = Some(42);
    run.tracked_pids = vec![42, 99];

    finish_stop_attempt(&mut run, &[99]);

    assert_eq!(run.status, AppLauncherRunStatus::Running);
    assert_eq!(run.root_pid, None);
    assert_eq!(run.tracked_pids, vec![99]);
    assert_eq!(run.finished_at, None);
    assert!(run
        .error
        .as_deref()
        .is_some_and(|error| error.contains("99")));
}

#[test]
fn app_launcher_global_disable_reenable_applies_to_active_session() {
    let manager = AutoAppLaunchManager::new(true, vec![local_entry("local")]);
    manager.on_game_started(false);

    let snapshot = manager.set_enabled(false);
    assert!(snapshot.active_session.as_ref().unwrap().runs.is_empty());

    let snapshot = manager.set_enabled(true);
    assert_eq!(snapshot.active_session.unwrap().runs.len(), 1);
}

#[test]
fn app_launcher_enabling_after_disabled_game_start_applies_rules() {
    let manager = AutoAppLaunchManager::new(false, vec![local_entry("local")]);
    manager.on_game_started(false);

    let snapshot = manager.set_enabled(true);

    assert_eq!(snapshot.active_session.unwrap().runs.len(), 1);
}

#[test]
fn app_launcher_steamvr_change_applies_vr_scope_to_active_session() {
    let mut desktop = local_entry("desktop");
    desktop.scope = AppLauncherScope::Desktop;
    let mut vr = local_entry("vr");
    vr.scope = AppLauncherScope::Vr;
    let manager = AutoAppLaunchManager::new(true, vec![desktop, vr]);
    manager.on_game_started(false);

    let snapshot = manager.snapshot();
    let session = snapshot.active_session.unwrap();
    assert!(!session.steamvr_running);
    assert_eq!(session.runs.len(), 1);
    assert_eq!(session.runs[0].entry_id, "desktop");

    manager.on_steamvr_changed(true);

    let snapshot = manager.snapshot();
    let session = snapshot.active_session.unwrap();
    assert!(session.steamvr_running);
    assert_eq!(session.runs.len(), 1);
    assert_eq!(session.runs[0].entry_id, "vr");
}

#[test]
fn app_launcher_game_start_with_steamvr_applies_vr_scope() {
    let mut desktop = local_entry("desktop");
    desktop.scope = AppLauncherScope::Desktop;
    let mut vr = local_entry("vr");
    vr.scope = AppLauncherScope::Vr;
    let manager = AutoAppLaunchManager::new(true, vec![desktop, vr]);

    manager.on_game_started(true);

    let snapshot = manager.snapshot();
    let session = snapshot.active_session.unwrap();
    assert!(session.steamvr_running);
    assert_eq!(session.runs.len(), 1);
    assert_eq!(session.runs[0].entry_id, "vr");
}

#[test]
fn app_launcher_args_split_preserves_quoted_values() {
    assert_eq!(
        split_command_line_args(r#"--flag "two words" \"literal\""#).unwrap(),
        vec!["--flag", "two words", r#""literal""#]
    );
    assert!(split_command_line_args(r#""unterminated"#).is_err());
}
