//! The one place the page reaches `window.localStorage`. The plugin set, the
//! editor-plugin set, the theme, and the session all persist through here, so
//! the access guard and the json round trip are written once.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The browser's local storage, or `None` outside a window (a worker, or a
/// context where storage is blocked).
pub fn store() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

/// The raw string stored under a key, if any.
pub fn get_string(key: &str) -> Option<String> {
    store().and_then(|store| store.get_item(key).ok().flatten())
}

/// Writes a raw string under a key, a no-op when storage is unavailable.
pub fn set_string(key: &str, value: &str) {
    if let Some(store) = store() {
        let _ = store.set_item(key, value);
    }
}

/// Deserializes the json stored under a key, or `None` when it is missing or
/// fails to parse.
pub fn get_json<T: DeserializeOwned>(key: &str) -> Option<T> {
    get_string(key).and_then(|text| serde_json::from_str(&text).ok())
}

/// Serializes a value to json and stores it under a key.
pub fn set_json<T: Serialize>(key: &str, value: &T) {
    if let Ok(text) = serde_json::to_string(value) {
        set_string(key, &text);
    }
}
