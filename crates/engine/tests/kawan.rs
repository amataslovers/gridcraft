//! KawanEditor policy (`kawan.rs`): deny and read-only reach `Session::execute` and the
//! workbook commit, read-only leaves the saved bytes alone, and the default policy leaves
//! GridCraft as upstream.

use gridcraft_engine::kawan::{self, Policy};
use gridcraft_engine::{EngineError, Session};
use serde_json::json;

fn session() -> Session {
    let mut s = Session::new();
    s.new_workbook();
    s
}

fn xlsx(s: &mut Session) -> String {
    let out = s.execute("file.saveBytes", json!({"format": "xlsx"})).unwrap();
    out["base64"].as_str().unwrap().to_string()
}

fn refused(r: Result<serde_json::Value, EngineError>) -> bool {
    matches!(r, Err(EngineError::Disabled(..)))
}

#[test]
fn deny_refuses_execute_and_hides_from_info() {
    let mut s = session();
    kawan::set(Policy { deny: vec!["cell.set".into()], ..Policy::default() });
    assert!(refused(s.execute("cell.set", json!({"cell": "A1", "input": "x"}))));
    let info = s.commands().into_iter().find(|c| c.id == "cell.set").unwrap();
    assert!(!info.enabled);
    kawan::set(Policy::default());
}

#[test]
fn read_only_refuses_commits_and_undo() {
    let mut s = session();
    s.execute("cell.set", json!({"cell": "A1", "input": "before"})).unwrap();
    kawan::set(Policy { read_only: true, mutating: vec!["edit.undo".into()], ..Policy::default() });
    assert!(refused(s.execute("cell.set", json!({"cell": "A1", "input": "x"}))));
    assert!(refused(s.execute("edit.undo", json!({}))));
    // Selection and copy are not edits.
    assert!(s.execute("selection.set", json!({"range": "A1"})).is_ok());
    assert!(s.execute("edit.copy", json!({})).is_ok());
    kawan::set(Policy::default());
}

#[test]
fn read_only_keeps_saved_bytes() {
    let mut s = session();
    s.execute("cell.set", json!({"cell": "A1", "input": "kept"})).unwrap();
    let before = xlsx(&mut s);
    kawan::set(Policy { read_only: true, ..Policy::default() });
    let _ = s.execute("cell.set", json!({"cell": "A1", "input": "changed"}));
    let _ = s.execute("home.bold", json!({"range": "A1"}));
    assert_eq!(xlsx(&mut s), before);
    kawan::set(Policy::default());
}

#[test]
fn upstream_policy_changes_nothing() {
    let mut s = session();
    kawan::set(Policy::default());
    s.execute("cell.set", json!({"cell": "A1", "input": "x"})).unwrap();
}

#[test]
fn allow_and_deny_spare_the_queries_the_ui_reads() {
    let mut s = session();
    let queries = vec!["cell.get".to_string(), "selection.*".to_string()];
    kawan::set(Policy { allow: Some(vec!["cell.set".into()]), deny: vec!["cell.*".into(), "selection.*".into()], queries, ..Policy::default() });
    assert!(s.execute("cell.get", json!({"cell": "A1"})).is_ok());
    assert!(s.execute("selection.set", json!({"range": "B2"})).is_ok());
    assert!(refused(s.execute("home.bold", json!({"range": "A1"}))));
    kawan::set(Policy::default());
}
