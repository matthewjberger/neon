//! Builds the language vocabulary the page needs, from the api manifest and the
//! standard library, sent once when the renderer is ready.

use crate::stdlib;
use nightshade_api::prelude::{command_manifest, command_method_name, command_schema};
use protocol::{CommandInfo, FieldInfo, StdModule};

/// Every api command as a [`CommandInfo`], the method name a script calls
/// alongside its variant, fields, and reply.
pub fn commands() -> Vec<CommandInfo> {
    command_manifest()
        .into_iter()
        .map(|spec| CommandInfo {
            method: command_method_name(spec.name),
            variant: spec.name.to_string(),
            description: spec.description.to_string(),
            fields: spec
                .fields
                .iter()
                .map(|field| FieldInfo {
                    name: field.name.to_string(),
                    type_name: field.type_name.to_string(),
                    role: field.role.to_string(),
                })
                .collect(),
            reply: spec.reply.to_string(),
        })
        .collect()
}

/// The command json schema as a string.
pub fn schema() -> String {
    command_schema().to_string()
}

/// The standard library modules.
pub fn modules() -> Vec<StdModule> {
    stdlib::modules()
}
