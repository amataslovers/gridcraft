//! KawanEditor policy in the GridCraft UI: a hidden active tab falls back to a visible one, a
//! read-only sheet opens no cell editor, and the autofilter fallback is closed.

use egui_kittest::Harness;
use gridcraft_engine::Session;
use gridcraft_engine::kawan::{self, Policy};
use gridcraft_ui_egui::{Services, SheetApp};
use serde_json::json;

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn harness(app: SheetApp) -> Harness<'static, SheetApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

#[test]
fn hidden_active_tab_falls_back() {
    kawan::set(Policy { hidden_tabs: vec!["Home".into()], ..Policy::default() });
    let mut a = app();
    a.ui.ribbon_tab = "Home".into();
    let h = harness(a);
    assert_ne!(h.state().ui.ribbon_tab, "Home");
    kawan::set(Policy::default());
}

#[test]
fn read_only_does_not_open_the_cell_editor() {
    kawan::set(Policy { read_only: true, ..Policy::default() });
    let mut a = app();
    a.begin_edit(None, false);
    assert!(a.editor.is_none());
    kawan::set(Policy::default());
    a.begin_edit(None, false);
    assert!(a.editor.is_some(), "upstream opens the editor");
}

#[test]
fn read_only_autofilter_fallback_is_closed() {
    let mut a = app();
    a.session.execute("cell.set", json!({"cell": "A1", "input": "h"})).unwrap();
    a.session.execute("insert.table", json!({"range": "A1:B3", "hasHeaders": true})).unwrap();
    let had = a.session.active().and_then(|d| d.wb.active()).map(|s| s.autofilter.is_some()).unwrap_or(false);
    kawan::set(Policy { read_only: true, ..Policy::default() });
    a.grid.filter_menu = Some((0, 0, egui::pos2(10.0, 10.0)));
    let h = harness(a);
    let has = h.state().session.active().and_then(|d| d.wb.active()).map(|s| s.autofilter.is_some()).unwrap_or(false);
    assert_eq!(has, had, "read-only adds no autofilter");
    kawan::set(Policy::default());
}
