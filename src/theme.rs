//! UI color themes. Each entry pairs the `data-theme` attribute value with the
//! label shown in the toolbar dropdown. The variable blocks live in
//! `public/styles.css`. Ported from the nightshade editor's theme handling.

pub const THEMES: &[(&str, &str)] = &[
    ("vscode", "VS Code Dark"),
    ("midnight", "Midnight"),
    ("ember", "Ember"),
    ("forest", "Forest"),
    ("paper", "Paper"),
];

const THEME_KEY: &str = "neon.theme";

/// The persisted theme id, falling back to the default when nothing is stored or
/// the stored id no longer exists.
pub fn stored_theme() -> String {
    crate::storage::get_string(THEME_KEY)
        .filter(|stored| THEMES.iter().any(|(id, _)| id == stored))
        .unwrap_or_else(|| THEMES[0].0.to_string())
}

/// Switches the page to the given theme without persisting, for live previews
/// while hovering the picker.
pub fn preview_theme(id: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = element.set_attribute("data-theme", id);
    }
}

/// Switches the page to the given theme and persists the choice.
pub fn apply_theme(id: &str) {
    preview_theme(id);
    crate::storage::set_string(THEME_KEY, id);
}

/// The label for a theme id.
pub fn theme_label(id: &str) -> &'static str {
    THEMES
        .iter()
        .find(|(theme_id, _)| *theme_id == id)
        .map(|(_, label)| *label)
        .unwrap_or(THEMES[0].1)
}
