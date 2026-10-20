//! GridCraft as a KawanEditor bridge host.

use gridcraft_engine::io::{base64_decode, base64_encode};
use gridcraft_ui_egui::SheetApp;
use serde_json::{Value, json};

use super::policy::{self, Policy};
use super::protocol::{Classify, CommandInfo, Host};

/// Commands that change the document although they are not undoable, or replace it with another
/// one (a save would then hand that one to the page under this document's key),
/// and the user's own saves: view mode saves nothing.
const ALSO_MUTATING: &[&str] = &["edit.undo", "edit.redo", "edit.cut", "file.new", "file.open", "file.close", "file.save", "file.saveAs"];

fn undoable(id: &str) -> bool {
    gridcraft_engine::find_command(id).is_some_and(|s| s.undoable)
}

/// Formats GridCraft writes; a document opened in one of them is saved back in it, others save as xlsx.
const SAVE_FORMATS: &[&str] = &["xlsx", "csv", "tsv", "json", "html"];
/// Formats that keep everything; only a save in one of these leaves the workbook clean (as upstream).
const LOSSLESS: &[&str] = &["xlsx", "json"];

/// State queries and selection moves the UI runs on its own; the host's lists skip them.
const QUERIES: &[&str] = &["cell.get", "selection.*", "sheet.activate"];

pub struct GridHost<'a>(pub &'a mut SheetApp);

fn ext(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

impl GridHost<'_> {
    /// The format `save` writes: the opened file's own format when GridCraft writes it, else xlsx.
    fn save_format(&self) -> String {
        let opened = self.0.session.active().map(|d| ext(&d.title)).unwrap_or_default();
        if SAVE_FORMATS.contains(&opened.as_str()) { opened } else { "xlsx".to_string() }
    }

    fn serialize(&mut self) -> Result<(String, Vec<u8>), String> {
        self.0.session.active().ok_or("no document")?;
        // A cell still being typed into belongs in the save, as with the app's own Save.
        self.0.commit_edit(0, 0, false, false);
        let format = self.save_format();
        // The bridge's own save: the host's command deny list does not apply to it.
        let out = policy::as_host(|| self.0.run("file.saveBytes", json!({"format": format})))?;
        let name = out.get("name").and_then(Value::as_str).ok_or("saveBytes returned no name")?.to_string();
        let bytes = out.get("base64").and_then(Value::as_str).and_then(base64_decode).ok_or("saveBytes returned no data")?;
        Ok((name, bytes))
    }
}

impl Host for GridHost<'_> {
    fn open(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        policy::as_host(|| self.0.run("file.open", json!({"name": name, "base64": base64_encode(&bytes)}))).map(|_| ())
    }

    fn save(&mut self) -> Result<(String, Vec<u8>), String> {
        let saved = self.serialize()?;
        self.mark_saved();
        Ok(saved)
    }

    fn export_copy(&mut self) -> Result<(String, Vec<u8>), String> {
        self.serialize()
    }

    fn exec(&mut self, id: &str, params: Value) -> Result<Value, String> {
        self.0.run(id, params)
    }

    fn commands(&self) -> Vec<CommandInfo> {
        self.0
            .session
            .commands()
            .into_iter()
            .filter(|c| policy::allows(c.id, undoable(c.id)).is_ok())
            .map(|c| CommandInfo { id: c.id.to_string(), label: c.label.to_string(), menu: c.menu.join(" › "), mutates: undoable(c.id) })
            .collect()
    }

    fn dirty(&self) -> bool {
        self.0.session.documents().iter().any(|d| d.is_dirty())
    }

    fn revision(&self) -> u64 {
        self.0.session.active().map_or(0, |d| d.revision)
    }

    fn set_author(&mut self, name: &str) {
        self.0.session.prefs.user_name = name.chars().take(200).collect();
    }

    fn set_policy(&mut self, mut p: Policy) {
        p.mutating = ALSO_MUTATING.iter().map(|s| s.to_string()).collect();
        p.queries = QUERIES.iter().map(|s| s.to_string()).collect();
        let mode = gridcraft_ui_egui::kawan_brand::theme_mode(p.theme);
        policy::set(p);
        // Always, `System` too (the browser's, live); the next frame re-applies the theme with
        // the host accent. As the bridge: the host's own command lists do not block it.
        let _ = policy::as_host(|| self.0.run("view.theme", json!({"mode": mode})));
    }

    fn classify(&self, name: &str) -> Classify {
        // Only a file in the save format is the document's save; Export as CSV/Web Page are copies.
        if ext(name) == self.save_format() { Classify::Save } else { Classify::Export }
    }

    fn mark_saved(&mut self) {
        if LOSSLESS.contains(&self.save_format().as_str())
            && let Some(doc) = self.0.session.active_mut()
        {
            doc.saved = doc.wb.clone();
        }
    }
}
