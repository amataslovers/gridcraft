//! KawanEditor policy in the GridCraft UI: a hidden active tab falls back to a visible one, a
//! read-only sheet opens no cell editor, and the autofilter fallback is closed. KawanEditor
//! branding: no ArtCraft mark, the logo in the title bar and the host's logo and link.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
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

/// The topmost node labelled `label` (`get_all`: About, Home and the title bar can all carry one).
fn topmost<'a>(h: &'a Harness<'static, SheetApp>, label: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_label(label).min_by(|a, b| a.rect().top().total_cmp(&b.rect().top())).expect("get_all_by_label found one")
}

/// No ArtCraft mark and no community link; the attribution sentence of the About line is allowed.
/// A `Role::Label` node (`ui.label`) keeps its text in `value`, every other node in `label`.
fn no_artcraft(h: &Harness<'static, SheetApp>, place: &str) {
    let mark = h
        .query_all_by(|n| {
            [n.label(), n.value()]
                .into_iter()
                .flatten()
                .any(|l| (l.to_ascii_lowercase().contains("artcraft") && !l.contains("by the ArtCraft team")) || l.contains("Discord"))
        })
        .next();
    assert!(mark.is_none(), "{place}: {mark:?}");
}

/// A 4 x 4 opaque red PNG.
const RED_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x04,
    0x08, 0x06, 0x00, 0x00, 0x00, 0xA9, 0xF1, 0x9E, 0x7E, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0,
    0x1F, 0x19, 0x33, 0x90, 0x2E, 0x00, 0x00, 0x3C, 0x40, 0x1F, 0xE1, 0xE0, 0x81, 0x77, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// An app whose shell records the URLs it is asked to open (the web shell's `open_url`).
fn app_recording_urls() -> (SheetApp, Rc<RefCell<Vec<String>>>) {
    let opened = Rc::new(RefCell::new(Vec::new()));
    let sink = opened.clone();
    let services = Services { open_url: Some(Box::new(move |url: &str| sink.borrow_mut().push(url.to_string()))), ..Services::default() };
    let mut session = Session::new();
    session.new_workbook();
    (SheetApp::new(session, services), opened)
}

/// A few frames after a click; true when any of them handed a URL to egui instead of the shell.
fn steps_open_url_in_egui(h: &mut Harness<'static, SheetApp>) -> bool {
    let mut egui_opened = false;
    for _ in 0..4 {
        h.step();
        egui_opened |= h.output().platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::OpenUrl(_)));
    }
    egui_opened
}

#[test]
fn no_artcraft_label_in_home_and_about() {
    kawan::set(Policy::default());
    let mut h = harness(app());
    no_artcraft(&h, "workbook");
    h.state_mut().open_backstage(gridcraft_ui_egui::backstage::Page::Home);
    h.run_steps(4);
    no_artcraft(&h, "File > Home");
    h.state_mut().backstage = None;
    h.state_mut().open_dialog("start", json!({}));
    h.run_steps(4);
    no_artcraft(&h, "Start");
    assert_eq!(h.state().dialog.as_ref().map(|d| d.title.as_str()), Some("KawanEditor"), "the Start screen's title");
    h.state_mut().dialog = None;
    h.state_mut().open_dialog("about", json!({}));
    h.run_steps(4);
    no_artcraft(&h, "About");
    assert!(h.query_all_by_label("About KawanEditor").next().is_some(), "the dialog title");
    let about = kawan::about("GridCraft");
    assert!(h.query_all_by_label(&about).next().is_some(), "the About line {about:?}");
    assert!(h.query_all_by_label("KawanEditor").count() >= 3, "the About logo and heading, and the title bar logo");
}

#[test]
fn header_logo_is_drawn() {
    kawan::set(Policy::default());
    let h = harness(app());
    assert!(h.state().session.active().is_some(), "a workbook is open");
    let logo = topmost(&h, "KawanEditor").rect();
    let file = h.get_all_by_label("File").next().map(|n| n.rect());
    assert!(file.is_some_and(|f| logo.bottom() <= f.top()), "in the title bar, above the ribbon tabs: {logo:?} {file:?}");
    // Its neighbour, not a fixed x: macOS leaves room for the traffic lights first.
    let autosave = topmost(&h, "AutoSave").rect();
    assert!(logo.right() <= autosave.left(), "at its left end, before AutoSave: {logo:?} {autosave:?}");
}

#[test]
fn host_logo_with_href_opens_it() {
    kawan::set(Policy { logo_png: Some(RED_PNG.into()), logo_href: Some("https://example.com/".into()), ..Policy::default() });
    let (a, opened) = app_recording_urls();
    let mut h = harness(a);
    topmost(&h, "KawanEditor").click();
    assert!(!steps_open_url_in_egui(&mut h), "the shell opens it, once");
    assert_eq!(*opened.borrow(), ["https://example.com/"]);
    let logo = gridcraft_ui_egui::kawan_brand::logo_texture(&h.ctx).map(|t| t.size());
    assert_eq!(logo, Some([4, 4]), "the host's PNG, not the default");
    kawan::set(Policy::default());
}

#[test]
fn host_logo_without_href_is_not_a_link() {
    kawan::set(Policy { logo_png: Some(b"not a png".to_vec().into()), ..Policy::default() });
    let (a, opened) = app_recording_urls();
    let mut h = harness(a);
    topmost(&h, "KawanEditor").click();
    assert!(!steps_open_url_in_egui(&mut h));
    assert!(opened.borrow().is_empty());
    let logo = gridcraft_ui_egui::kawan_brand::logo_texture(&h.ctx).map(|t| t.size());
    assert_eq!(logo, Some([256, 256]), "a bad host PNG falls back to the default logo");
    kawan::set(Policy::default());
}

#[test]
fn host_accent_reaches_visuals() {
    use egui::Color32;
    use gridcraft_ui_egui::theme::Tokens;
    let visuals = |h: &Harness<'static, SheetApp>| h.ctx.global_style().visuals.clone();
    kawan::set(Policy::default());
    let mut h = harness(app());
    assert_eq!(visuals(&h).selection.stroke.color, Color32::from_rgb(0x10, 0x7C, 0x41), "GridCraft's own accent without a host accent");
    // A policy arriving after start-up: the bridge's init then runs `view.theme` with the host theme.
    let accent = Color32::from_rgb(0xE1, 0x1D, 0x48);
    kawan::set(Policy { accent: Some([0xE1, 0x1D, 0x48]), ..Policy::default() });
    h.state_mut().run("view.theme", json!({"mode": "light"})).unwrap();
    h.run_steps(4);
    assert_eq!(visuals(&h).selection.stroke.color, accent);
    assert_eq!(visuals(&h).hyperlink_color, accent);
    assert_eq!(visuals(&h).selection.bg_fill, Color32::from_rgb(240, 213, 219), "accent_soft, light");
    assert_eq!(Tokens::get(&h.ctx).sel_border, accent, "every Tokens::get caller");
    h.state_mut().run("view.theme", json!({"mode": "dark"})).unwrap();
    h.run_steps(4);
    assert!(visuals(&h).dark_mode);
    assert_eq!(visuals(&h).selection.stroke.color, accent);
    assert_eq!(visuals(&h).selection.bg_fill, Color32::from_rgb(96, 37, 50), "accent_soft, dark");
    kawan::set(Policy::default());
    h.run_steps(4);
    assert_eq!(visuals(&h).selection.stroke.color, Tokens::dark().accent, "GridCraft's own dark accent again");
    assert_eq!(Tokens::get(&h.ctx), Tokens::dark());
}
