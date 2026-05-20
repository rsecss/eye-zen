fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_state_snapshot",
            "start_rest",
            "skip_rest",
            "pause_timer",
            "resume_timer",
            "get_config",
            "get_hotkey_status",
            "update_timer_config",
            "update_behavior_config",
            "update_display_config",
            "update_schedule_config",
            "update_hotkeys_config",
        ]),
    ))
    .expect("failed to build tauri app manifest");
}
