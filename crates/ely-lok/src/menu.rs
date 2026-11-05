//! The kernel's context menu: target and flags (DOC-02).

use std::{collections::HashMap, io};

use crate::{
    Doc, Worker,
    review::{entries, invalid, parse},
};

/// Where a right click landed, per the kernel's menu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Text,
    Table,
    /// A Writer graphic, object or frame.
    Frame,
    /// A spreadsheet cell.
    Cell,
    /// A drawing object outside Writer.
    Object,
    /// A picture outside Writer.
    Picture,
    /// An Impress slide, nothing selected.
    Slide,
}

/// A misspelled word's menu: suggestions as the kernel's rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Spelling {
    /// Label and `ApplyRule` value for each suggestion.
    pub suggestions: Vec<(String, String)>,
}

/// One menu the kernel offered.
#[derive(Clone, Debug, PartialEq)]
pub struct Offered {
    pub target: Target,
    pub spelling: Option<Spelling>,
    /// Each command, at any depth, and whether it runs now.
    enabled: HashMap<String, bool>,
}

impl Offered {
    /// The kernel offers `uno` and it runs now.
    pub fn allows(&self, uno: &str) -> bool {
        self.enabled.get(uno) == Some(&true)
    }
}

/// Which asked commands run now, per the kernel's dispatcher.
#[derive(Clone, Debug, PartialEq)]
pub struct Runs(HashMap<String, bool>);

impl Runs {
    /// The kernel lets `uno` run now.
    pub fn allows(&self, uno: &str) -> bool {
        self.0.get(uno) == Some(&true)
    }
}

/// The read of which commands run now (patch 0045).
pub const STATES: &str = ".uno:CommandStates";

/// Asks the kernel which of `commands` run now.
pub fn runs(worker: &mut Worker, doc: &Doc, commands: &[&str]) -> io::Result<Runs> {
    let json = worker.values(doc, &format!("{STATES}?commands={}", commands.join(",")))?;
    read_runs(&json, commands)
}

/// Reads the answer; every asked command must be in it.
pub(crate) fn read_runs(json: &str, commands: &[&str]) -> io::Result<Runs> {
    let value = parse(json)?;
    let states = value["commandStates"]
        .as_object()
        .ok_or_else(|| invalid(format!("no command states: {json}")))?;
    let read = |command: &&str| match states.get(*command).and_then(serde_json::Value::as_bool) {
        Some(on) => Ok((command.to_string(), on)),
        None => Err(invalid(format!("no state for {command}: {json}"))),
    };
    Ok(Runs(commands.iter().map(read).collect::<io::Result<_>>()?))
}

/// Reads a `LOK_CALLBACK_CONTEXT_MENU` payload.
pub fn read(json: &str) -> io::Result<Offered> {
    let list = parse(json)?;
    let mut enabled = HashMap::new();
    let mut suggestions = Vec::new();
    walk(entries(&list, "menu")?, &mut enabled, &mut suggestions)?;
    let spelling = enabled
        .keys()
        .any(|command| command.starts_with(".uno:SpellCheckIgnoreAll"))
        .then_some(Spelling { suggestions });
    let target = if enabled.contains_key(".uno:TableInsertMenu") {
        Target::Table
    } else if enabled.contains_key(".uno:WrapMenu") {
        Target::Frame
    } else if enabled.contains_key(".uno:InsertCell") {
        Target::Cell
    } else if enabled.contains_key(".uno:ChangePicture") {
        Target::Picture
    } else if enabled.contains_key(".uno:ArrangeMenu") {
        Target::Object
    } else if enabled.contains_key(".uno:SlideLayoutMenu") {
        Target::Slide
    } else {
        Target::Text
    };
    Ok(Offered {
        target,
        spelling,
        enabled,
    })
}

fn walk(
    items: &[serde_json::Value],
    enabled: &mut HashMap<String, bool>,
    suggestions: &mut Vec<(String, String)>,
) -> io::Result<()> {
    for item in items {
        if item["type"] == "separator" {
            continue;
        }
        let on = match item["enabled"].as_str() {
            Some("true") => true,
            Some("false") => false,
            _ => return Err(invalid(format!("a menu item has no enabled flag: {item}"))),
        };
        // A submenu may have no command.
        match (item["command"].as_str(), item["type"].as_str()) {
            (Some(command), _) => {
                enabled.insert(command.to_string(), on);
                if let Some(rule) =
                    command.strip_prefix(".uno:SpellCheckApplySuggestion?ApplyRule:string=")
                {
                    let label = item["text"]
                        .as_str()
                        .ok_or_else(|| invalid(format!("a suggestion has no text: {item}")))?;
                    suggestions.push((label.to_string(), rule.to_string()));
                }
            }
            (None, Some("menu")) => {}
            (None, _) => return Err(invalid(format!("a menu item has no command: {item}"))),
        }
        if let Some(sub) = item.get("menu") {
            let sub = sub
                .as_array()
                .ok_or_else(|| invalid(format!("a submenu is no list: {item}")))?;
            walk(sub, enabled, suggestions)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
