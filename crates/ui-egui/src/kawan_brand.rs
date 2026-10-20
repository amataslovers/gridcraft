//! KawanEditor branding (logo, About line) and the host's accent colour for the GridCraft UI.
//! GridCraft's own UI calls these at a few hook points; everything here reads the host policy
//! (`gridcraft_engine::kawan`) and falls back to KawanEditor's default logo and GridCraft's own
//! colours.

use egui::{Color32, Sense, vec2};
use gridcraft_engine::kawan as policy;

use crate::theme::Tokens;

/// The default logo: KawanEditor's mark, 256 x 256.
const DEFAULT_LOGO: &[u8] = include_bytes!("../assets/kawan-logo.png");
/// Larger host logos are refused (a small PNG can inflate to a huge bitmap).
const MAX_SIDE: u32 = 2048;

/// The width and height a PNG's header declares (the IHDR chunk right after the signature).
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !policy::is_png(bytes) || bytes.get(12..16) != Some(&b"IHDR"[..]) {
        return None;
    }
    let be32 = |i: usize| bytes.get(i..i + 4).and_then(|s| <[u8; 4]>::try_from(s).ok()).map(u32::from_be_bytes);
    Some((be32(16)?, be32(20)?))
}

/// Decode a PNG into an egui image; `None` for anything that is not a PNG of a sane size. The
/// header is checked before decoding, so a decompression bomb is never inflated.
pub fn decode_png(bytes: &[u8]) -> Option<egui::ColorImage> {
    let sane = |w: u32, h: u32| w > 0 && h > 0 && w <= MAX_SIDE && h <= MAX_SIDE;
    let (w, h) = png_size(bytes)?;
    if !sane(w, h) {
        return None;
    }
    let rgba = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?.to_rgba8();
    let (w, h) = rgba.dimensions();
    if !sane(w, h) {
        return None;
    }
    let size = [usize::try_from(w).ok()?, usize::try_from(h).ok()?];
    let raw = rgba.as_raw();
    (raw.len() == size[0].checked_mul(size[1])?.checked_mul(4)?).then(|| egui::ColorImage::from_rgba_unmultiplied(size, raw))
}

/// The logo texture: the host's PNG when it sent one that decodes, otherwise KawanEditor's. One
/// texture, kept with the policy generation it was built for and replaced when a new policy comes.
pub fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("kawan-logo");
    let generation = policy::generation();
    if let Some((built, texture)) = ctx.data(|d| d.get_temp::<(u64, egui::TextureHandle)>(id))
        && built == generation
    {
        return Some(texture);
    }
    let image = policy::logo_png().and_then(|png| decode_png(&png)).or_else(|| decode_png(DEFAULT_LOGO))?;
    let texture = ctx.load_texture("kawan-logo", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, (generation, texture.clone())));
    Some(texture)
}

/// The logo, `size` points square, labelled "KawanEditor". With a host `href` it is a link: a
/// click goes to `open_url` (the shell's, which on the web checks it with `links::classify`), or
/// to egui's `open_url` without one (dialogs hand that to the shell as well).
fn logo_with(ui: &mut egui::Ui, size: f32, open_url: Option<&dyn Fn(&str)>) -> egui::Response {
    let href = policy::logo_href();
    let sense = if href.is_some() { Sense::click() } else { Sense::hover() };
    let response = match logo_texture(ui.ctx()) {
        Some(texture) => ui.add(egui::Image::from_texture(&texture).fit_to_exact_size(vec2(size, size)).alt_text(policy::PRODUCT).sense(sense)),
        None => {
            let response = ui.allocate_response(vec2(size, size), sense);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, policy::PRODUCT));
            response
        }
    };
    match href {
        Some(href) => {
            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                match open_url {
                    Some(open) => open(&href),
                    None => ui.ctx().open_url(egui::OpenUrl::new_tab(href)),
                }
            }
            response
        }
        None => response,
    }
}

/// The logo, `size` points square (see [`logo_with`]).
pub fn logo(ui: &mut egui::Ui, size: f32) -> egui::Response {
    logo_with(ui, size, None)
}

/// The small logo at the left end of the title bar; `open_url` is the shell's
/// (`Services::open_url`), since nothing outside dialogs hands egui's own requests to it.
pub fn header_logo(ui: &mut egui::Ui, open_url: Option<&dyn Fn(&str)>) -> egui::Response {
    ui.add_space(2.0);
    let response = logo_with(ui, 18.0, open_url);
    ui.add_space(6.0);
    response
}

/// About's heading: the logo and the product name.
pub fn about_heading(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        logo(ui, 48.0);
        ui.heading(policy::PRODUCT);
    });
}

/// The About dialog's credit line (it replaces GridCraft's tagline and links).
pub fn about() -> String {
    policy::about("GridCraft")
}

/// GridCraft's `view.theme` mode for the host's colour scheme (`System` follows the browser).
pub fn theme_mode(theme: policy::Theme) -> &'static str {
    match theme {
        policy::Theme::System => "system",
        policy::Theme::Light => "light",
        policy::Theme::Dark => "dark",
    }
}

/// True once per new host policy: the frame that sees it re-applies the theme (the accent).
pub fn policy_changed(ctx: &egui::Context) -> bool {
    let generation = policy::generation();
    ctx.data_mut(|d| std::mem::replace(d.get_temp_mut_or(egui::Id::new("kawan-generation"), generation), generation) != generation)
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// The host accent, or `upstream` (a colour GridCraft draws outside its tokens).
pub fn accent_or(upstream: Color32) -> Color32 {
    policy::accent().map_or(upstream, rgb)
}

/// GridCraft's tokens with the host accent: the accent family, the selected header text, the
/// select-all corner and the cell selection. Unchanged without an accent.
pub fn tint(t: Tokens) -> Tokens {
    match policy::accent() {
        Some(a) => tint_with(t, a),
        None => t,
    }
}

fn tint_with(mut t: Tokens, a: [u8; 3]) -> Tokens {
    // The surface: the window colour, which is also egui's panel fill.
    let s = [t.window.r(), t.window.g(), t.window.b()];
    let (dark, soft, all) = if t.dark { (0.20, 0.30, 0.35) } else { (0.25, 0.14, 0.25) };
    t.accent = rgb(a);
    t.accent_dark = rgb(policy::mix(a, [0; 3], dark));
    t.accent_soft = rgb(policy::mix(s, a, soft));
    t.header_sel_text = rgb(a);
    t.header_all_bg = rgb(policy::mix(s, a, all));
    // GridCraft's own wash strength: 0x1C light, 0x24 dark.
    t.sel_fill = Color32::from_rgba_unmultiplied(a[0], a[1], a[2], t.sel_fill.a());
    t.sel_border = rgb(a);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_accent(accent: Option<[u8; 3]>) {
        policy::set(policy::Policy { accent, ..policy::Policy::default() });
    }

    #[test]
    fn tint_is_identity_without_accent() {
        with_accent(None);
        for t in [Tokens::light(), Tokens::dark()] {
            assert_eq!(tint(t), t);
        }
        assert_eq!(accent_or(Color32::from_rgb(0x10, 0x7C, 0x41)), Color32::from_rgb(0x10, 0x7C, 0x41));
    }

    #[test]
    fn tint_replaces_the_accent_family() {
        let a = [0xE1, 0x1D, 0x48];
        with_accent(Some(a));
        let light = tint(Tokens::light());
        assert_eq!(light.accent, rgb(a));
        assert_eq!(light.header_sel_text, rgb(a));
        assert_eq!(light.sel_border, rgb(a));
        assert_eq!(light.accent_dark, rgb([169, 22, 54]));
        assert_eq!(light.accent_soft, rgb([240, 213, 219]));
        assert_eq!(light.header_all_bg, rgb([239, 190, 200]));
        assert_eq!(light.sel_fill, Color32::from_rgba_unmultiplied(0xE1, 0x1D, 0x48, 0x1C));
        let dark = tint(Tokens::dark());
        assert_eq!(dark.accent, rgb(a));
        assert_eq!(dark.header_sel_text, rgb(a));
        assert_eq!(dark.sel_border, rgb(a));
        assert_eq!(dark.accent_dark, rgb([180, 23, 58]));
        assert_eq!(dark.accent_soft, rgb([96, 37, 50]));
        assert_eq!(dark.header_all_bg, rgb([105, 37, 52]));
        assert_eq!(dark.sel_fill, Color32::from_rgba_unmultiplied(0xE1, 0x1D, 0x48, 0x24), "GridCraft's dark alpha");
        // Everything else stays GridCraft's.
        for (tinted, t) in [(light, Tokens::light()), (dark, Tokens::dark())] {
            let restored = Tokens {
                accent: t.accent,
                accent_dark: t.accent_dark,
                accent_soft: t.accent_soft,
                header_sel_text: t.header_sel_text,
                header_all_bg: t.header_all_bg,
                sel_fill: t.sel_fill,
                sel_border: t.sel_border,
                ..tinted
            };
            assert_eq!(restored, t);
        }
        assert_eq!(accent_or(Color32::from_rgb(0x10, 0x7C, 0x41)), rgb(a));
        // The pure tokens stay upstream; only the builders (`Tokens::get`, `theme::apply`) tint.
        assert_eq!(Tokens::light().accent, Color32::from_rgb(0x10, 0x7C, 0x41));
        with_accent(None);
    }

    #[test]
    fn theme_mode_maps_the_host_scheme() {
        assert_eq!(theme_mode(policy::Theme::System), "system");
        assert_eq!(theme_mode(policy::Theme::Light), "light");
        assert_eq!(theme_mode(policy::Theme::Dark), "dark");
    }

    #[test]
    fn policy_changed_once_per_set() {
        let ctx = egui::Context::default();
        assert!(!policy_changed(&ctx), "the first look only records the generation");
        assert!(!policy_changed(&ctx));
        policy::set(policy::Policy::default());
        assert!(policy_changed(&ctx));
        assert!(!policy_changed(&ctx));
    }

    /// A 4 x 4 opaque red PNG.
    const RED_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x04, 0x08, 0x06, 0x00, 0x00, 0x00, 0xA9, 0xF1, 0x9E, 0x7E, 0x00, 0x00, 0x00, 0x12, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF,
        0xC0, 0xF0, 0x1F, 0x19, 0x33, 0x90, 0x2E, 0x00, 0x00, 0x3C, 0x40, 0x1F, 0xE1, 0xE0, 0x81, 0x77, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn bad_host_png_falls_back_to_default() {
        assert!(decode_png(b"nope").is_none());
        assert!(decode_png(&[]).is_none());
        assert!(decode_png(&RED_PNG[..40]).is_none(), "a PNG header alone is not enough");
        assert_eq!(decode_png(DEFAULT_LOGO).map(|i| i.size), Some([256, 256]));
        assert_eq!(decode_png(RED_PNG).map(|i| i.size), Some([4, 4]));
    }

    #[test]
    fn oversized_or_non_png_is_refused() {
        let mut huge = RED_PNG.to_vec();
        huge[16..20].copy_from_slice(&(MAX_SIDE + 1).to_be_bytes());
        assert_eq!(png_size(&huge), Some((MAX_SIDE + 1, 4)));
        assert!(decode_png(&huge).is_none(), "refused from the header, before decoding");
        // A GIF decodes in the `image` crate, but the host logo is PNG only.
        assert!(decode_png(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").is_none());
    }

    #[test]
    fn logo_texture_follows_the_policy() {
        let ctx = egui::Context::default();
        policy::set(policy::Policy { logo_png: Some(RED_PNG.into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([4, 4]));
        let first = logo_texture(&ctx).map(|t| t.id());
        assert_eq!(logo_texture(&ctx).map(|t| t.id()), first, "cached while the policy stays");
        policy::set(policy::Policy { logo_png: Some(b"nope".to_vec().into()), ..policy::Policy::default() });
        assert_eq!(logo_texture(&ctx).map(|t| t.size()), Some([256, 256]), "a bad host PNG falls back");
        policy::set(policy::Policy::default());
    }

    #[test]
    fn about_names_grid_craft() {
        assert_eq!(about(), policy::about("GridCraft"));
        assert!(about().starts_with("KawanEditor "));
    }
}
