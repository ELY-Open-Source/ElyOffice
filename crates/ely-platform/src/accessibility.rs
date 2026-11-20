use crate::{env, settings::Motion};

/// Whether to reduce motion, and the one line saying why.
pub fn reduce_motion(setting: Motion, software: bool) -> (bool, &'static str) {
    reduce_motion_from(env::flag("ELY_REDUCE_MOTION"), setting, os_reduce_motion(), software)
}

/// Env, Ely's explicit choice, then system or software reduces.
fn reduce_motion_from(env: Option<bool>, setting: Motion, system: Option<bool>, software: bool) -> (bool, &'static str) {
    let set = match setting {
        Motion::System => None,
        Motion::Reduced => Some(true),
        Motion::Full => Some(false),
    };
    match (env, set, system) {
        (Some(true), _, _) => (true, "on, by ELY_REDUCE_MOTION"),
        (Some(false), _, _) => (false, "off, by ELY_REDUCE_MOTION"),
        (None, Some(true), _) => (true, "on, by the user setting"),
        (None, Some(false), _) => (false, "off, by the user setting"),
        (None, None, Some(true)) => (true, "on, by the system setting"),
        // Frames drawn on the CPU starve the load (R20.63).
        (None, None, _) if software => (true, "on, by a software renderer"),
        (None, None, Some(false)) => (false, "off, by the system setting"),
        (None, None, None) => (false, "off, no setting found"),
    }
}

#[cfg(target_os = "macos")]
fn os_reduce_motion() -> Option<bool> {
    Some(objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
}

#[cfg(target_os = "linux")]
fn os_reduce_motion() -> Option<bool> {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "enable-animations"])
        .output()
        .map_err(|error| log::debug!("accessibility: gsettings did not run: {error}"))
        .ok()?;
    if !output.status.success() {
        log::debug!("accessibility: gsettings has no enable-animations key");
        return None;
    }
    animations_enabled(&String::from_utf8_lossy(&output.stdout)).map(|enabled| !enabled)
}

#[cfg(windows)]
fn os_reduce_motion() -> Option<bool> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
    };

    let mut enabled: i32 = 1;
    // SAFETY: a live i32, the size the call expects.
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut enabled as *mut i32).cast(),
            0,
        )
    };
    (read != 0).then_some(enabled == 0)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn os_reduce_motion() -> Option<bool> {
    None
}

#[cfg(any(target_os = "linux", test))]
fn animations_enabled(gsettings: &str) -> Option<bool> {
    match gsettings.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Motion, animations_enabled, reduce_motion_from};

    #[test]
    fn the_env_wins_then_the_setting_then_the_os_then_full_motion() {
        assert_eq!(reduce_motion_from(Some(true), Motion::Full, Some(false), false), (true, "on, by ELY_REDUCE_MOTION"));
        assert_eq!(reduce_motion_from(Some(false), Motion::Reduced, Some(true), false), (false, "off, by ELY_REDUCE_MOTION"));
        assert_eq!(reduce_motion_from(None, Motion::Reduced, Some(false), false), (true, "on, by the user setting"));
        assert_eq!(reduce_motion_from(None, Motion::Full, Some(true), false), (false, "off, by the user setting"));
        assert_eq!(reduce_motion_from(None, Motion::System, Some(true), false), (true, "on, by the system setting"));
        assert_eq!(reduce_motion_from(None, Motion::System, None, false), (false, "off, no setting found"));
    }

    #[test]
    fn the_system_animations_on_default_and_a_software_renderer_reduce_motion() {
        assert_eq!(reduce_motion_from(None, Motion::System, Some(false), true), (true, "on, by a software renderer"));
        assert_eq!(reduce_motion_from(None, Motion::Full, Some(false), true), (false, "off, by the user setting"));
    }

    #[test]
    fn no_system_setting_and_a_software_renderer_never_says_full_motion() {
        let (reduce, said) = reduce_motion_from(None, Motion::System, None, true);
        assert!(reduce);
        assert_eq!(said, "on, by a software renderer");
    }

    #[test]
    fn a_hardware_renderer_with_the_system_setting_off_animates() {
        assert_eq!(reduce_motion_from(None, Motion::System, Some(false), false), (false, "off, by the system setting"));
    }

    #[test]
    fn full_motion_chosen_in_ely_animates_on_a_software_renderer() {
        assert_eq!(reduce_motion_from(None, Motion::Full, Some(false), true), (false, "off, by the user setting"));
    }

    #[test]
    fn gsettings_answers_parse_and_anything_else_is_no_answer() {
        assert_eq!(animations_enabled("true\n"), Some(true));
        assert_eq!(animations_enabled("false\n"), Some(false));
        assert_eq!(animations_enabled("uint32 7\n"), None);
        assert_eq!(animations_enabled(""), None);
    }
}
