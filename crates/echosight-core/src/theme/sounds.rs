//! Pure event resolution and volume mixing. Platform owns decoding, caching,
//! restart-at-zero and playback; this module never creates an audio device.
use super::{assets::ThemeRoot, v1::model::Theme};
use serde::Serialize;
use serde_json::Value;
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundDefinition {
    pub asset_name: String,
    pub volume: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedSound {
    pub asset: Value,
    pub volume: f64,
}

pub fn resolve_theme_sound_definition(
    theme: Option<&Theme>,
    event: &str,
) -> Option<SoundDefinition> {
    let definition = theme?.get("sounds")?.get("events")?.get(event)?;
    let (file, volume) = if let Some(file) = definition.as_str() {
        (file, 1.0)
    } else {
        (
            definition.as_object()?.get("file")?.as_str()?,
            unit_volume(definition.get("volume").and_then(Value::as_f64), 1.0),
        )
    };
    let filename = file
        .rsplit(['/', '\\'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(file);
    let asset_name = match filename.rfind('.') {
        Some(i) if i + 1 < filename.len() => &filename[..i],
        _ => filename,
    };
    Some(SoundDefinition {
        asset_name: asset_name.into(),
        volume,
    })
}
pub fn can_play_sounds(theme: Option<&Theme>, enabled: bool) -> bool {
    enabled
        && theme.is_some_and(|t| {
            t.get("sounds").and_then(|v| v.get("enabled")) != Some(&Value::Bool(false))
        })
}
pub fn playback_volume(settings_percent: f64, theme: &Theme, definition: &SoundDefinition) -> f64 {
    unit_volume(Some(settings_percent / 100.0), 0.6)
        * unit_volume(
            theme
                .get("sounds")
                .and_then(|v| v.get("volume"))
                .and_then(Value::as_f64),
            1.0,
        )
        * definition.volume
}
fn unit_volume(value: Option<f64>, fallback: f64) -> f64 {
    value
        .filter(|v| v.is_finite())
        .map_or(fallback, |v| v.clamp(0.0, 1.0))
}
/// Resolve an enabled event to contained audio bytes and mixed gain. Missing
/// files/non-audio assets are silent, matching the controller's failure boundary.
/// A platform cache should call this on a cache miss and recompute volume after
/// settings changes; this is deliberately not a duplicate sound player.
pub fn resolve_sound(
    root: &ThemeRoot,
    theme: Option<&Theme>,
    event: &str,
    enabled: bool,
    settings_percent: f64,
) -> io::Result<Option<ResolvedSound>> {
    if !can_play_sounds(theme, enabled) {
        return Ok(None);
    }
    let theme = theme.unwrap();
    let Some(definition) = resolve_theme_sound_definition(Some(theme), event) else {
        return Ok(None);
    };
    let asset = match root.load_asset(Some(theme), &Value::String(definition.asset_name.clone())) {
        Ok(Some(asset))
            if asset
                .get("mimeType")
                .and_then(Value::as_str)
                .is_some_and(|m| m.starts_with("audio/")) =>
        {
            asset
        }
        Ok(_) => return Ok(None),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    Ok(Some(ResolvedSound {
        asset,
        volume: playback_volume(settings_percent, theme, &definition),
    }))
}
