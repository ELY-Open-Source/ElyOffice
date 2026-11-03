use crate::env;

/// The locale tag: `ELY_LOCALE`, else the OS one.
pub fn system() -> String {
    if let Some(tag) = env::value("ELY_LOCALE") {
        return tag;
    }
    platform()
}

/// The region's two letters, as paper follows; None when unnamed.
pub fn region() -> Option<String> {
    match env::value("ELY_LOCALE") {
        Some(tag) => region_of(&tag),
        None => regional(),
    }
}

/// The tag's region subtag; extensions and `@` keywords end it.
pub fn region_of(tag: &str) -> Option<String> {
    let alpha = |part: &str, lens: std::ops::RangeInclusive<usize>| lens.contains(&part.len()) && part.chars().all(|c| c.is_ascii_alphabetic());
    let mut parts = tag.split('@').next().unwrap_or(tag).split(['-', '_']);
    // A private-use tag names no language.
    if !alpha(parts.next()?, 2..=8) {
        return None;
    }
    let mut part = parts.next()?;
    while alpha(part, 3..=4) {
        part = parts.next()?;
    }
    alpha(part, 2..=2).then(|| part.to_ascii_uppercase())
}

#[cfg(target_os = "macos")]
fn regional() -> Option<String> {
    use objc2_foundation::NSLocale;
    NSLocale::currentLocale().regionCode().map(|code| code.to_string().to_ascii_uppercase())
}

#[cfg(target_os = "windows")]
fn regional() -> Option<String> {
    region_of(&platform())
}

#[cfg(target_os = "linux")]
fn regional() -> Option<String> {
    region_of(&resolve_env(["LC_ALL", "LC_PAPER", "LANG"], env::value))
}

#[cfg(target_os = "macos")]
fn platform() -> String {
    use objc2_foundation::NSLocale;
    if let Some(tag) = NSLocale::preferredLanguages()
        .firstObject()
        .map(|tag| tag.to_string())
        && !tag.is_empty()
    {
        return tag;
    }
    NSLocale::currentLocale()
        .localeIdentifier()
        .to_string()
        .replace('_', "-")
}

#[cfg(target_os = "linux")]
fn platform() -> String {
    resolve_env(["LC_ALL", "LC_MESSAGES", "LANG"], env::value)
}

#[cfg(target_os = "windows")]
fn platform() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if length == 0 {
        log::warn!("locale: GetUserDefaultLocaleName failed");
        return "en".into();
    }
    String::from_utf16_lossy(&buffer[..length as usize - 1])
}

/// First non-empty variable wins; C/POSIX ends as en.
#[cfg(any(target_os = "linux", test))]
fn resolve_env(keys: [&str; 3], get: impl Fn(&str) -> Option<String>) -> String {
    for key in keys {
        let Some(value) = get(key) else { continue };
        if value.is_empty() {
            continue;
        }
        let tag = value
            .split(['.', '@'])
            .next()
            .unwrap_or(&value)
            .replace('_', "-");
        return match tag.as_str() {
            "C" | "POSIX" => "en".into(),
            _ => tag,
        };
    }
    "en".into()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{region_of, resolve_env};

    const MESSAGES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn the_first_non_empty_variable_wins() {
        assert_eq!(resolve_env(MESSAGES, env(&[("LANG", "zh_CN.UTF-8")])), "zh-CN");
        assert_eq!(
            resolve_env(MESSAGES, env(&[
                ("LC_MESSAGES", "fr_FR.UTF-8"),
                ("LANG", "zh_CN.UTF-8")
            ])),
            "fr-FR"
        );
        assert_eq!(
            resolve_env(MESSAGES, env(&[("LC_ALL", ""), ("LANG", "ja_JP.UTF-8")])),
            "ja-JP"
        );
    }

    #[test]
    fn c_and_posix_resolve_to_en_and_stop() {
        assert_eq!(
            resolve_env(MESSAGES, env(&[("LC_ALL", "C.UTF-8"), ("LANG", "zh_CN.UTF-8")])),
            "en"
        );
        assert_eq!(
            resolve_env(MESSAGES, env(&[("LC_MESSAGES", "POSIX"), ("LANG", "zh_CN.UTF-8")])),
            "en"
        );
        assert_eq!(
            resolve_env(MESSAGES, env(&[("LC_ALL", "C"), ("LC_MESSAGES", "zh_CN.UTF-8")])),
            "en"
        );
    }

    #[test]
    fn the_region_is_the_two_letter_subtag() {
        assert_eq!(region_of("en-US").as_deref(), Some("US"));
        assert_eq!(region_of("zh-Hans-CN").as_deref(), Some("CN"));
        assert_eq!(region_of("es-419"), None);
        assert_eq!(region_of("en"), None);
        assert_eq!(region_of("en-US@calendar=japanese").as_deref(), Some("US"));
        assert_eq!(region_of("en-u-ca-gregory"), None);
        assert_eq!(region_of("de-DE-u-ca-gregory").as_deref(), Some("DE"));
        assert_eq!(region_of("zh-yue-HK").as_deref(), Some("HK"));
        assert_eq!(region_of("x-us"), None);
        assert_eq!(region_of("i-klingon"), None);
        let paper = ["LC_ALL", "LC_PAPER", "LANG"];
        assert_eq!(resolve_env(paper, env(&[("LC_PAPER", "en_US.UTF-8"), ("LANG", "de_DE.UTF-8")])), "en-US");
    }

    #[test]
    fn nothing_set_resolves_to_en() {
        assert_eq!(resolve_env(MESSAGES, env(&[])), "en");
        assert_eq!(resolve_env(MESSAGES, env(&[("LANG", "")])), "en");
    }
}
