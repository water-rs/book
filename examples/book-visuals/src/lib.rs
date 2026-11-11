use waterui::app::App;
use waterui::barcode::Barcode;
use waterui::color::Srgb;
use waterui::color::working::from_linear_srgb;
use waterui::prelude::*;
use waterui::preview;
use waterui::theme::{install_color_signal, installed_color_signal};
use waterui::{Plugin, Resolvable, contract_violation};

fn working_color(red: f32, green: f32, blue: f32) -> WorkingColor {
    from_linear_srgb([red, green, blue], 1.0)
}

/// The signal installed for a book-defined color token.
fn installed_token<T: 'static>(env: &Environment) -> Computed<WorkingColor> {
    installed_color_signal::<T>(env).unwrap_or_else(|| {
        contract_violation!(
            "{} is not installed in the environment",
            core::any::type_name::<T>()
        );
        Computed::constant(WorkingColor::BLACK)
    })
}

/// A book-defined token for positive status.
#[derive(Debug, Clone, Copy)]
struct SuccessColor;

impl Resolvable for SuccessColor {
    type Resolved = WorkingColor;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = Self::Resolved> {
        installed_token::<Self>(env)
    }
}

/// A book-defined token for highlighted content.
#[derive(Debug, Clone, Copy)]
struct HighlightColor;

impl Resolvable for HighlightColor {
    type Resolved = WorkingColor;

    fn resolve(&self, env: &Environment) -> impl Signal<Output = Self::Resolved> {
        installed_token::<Self>(env)
    }
}

/// Installs the signals behind the book-defined color tokens.
struct BookTokens;

impl Plugin for BookTokens {
    fn install(self, env: &mut Environment) {
        install_color_signal::<SuccessColor>(
            env,
            Computed::constant(Srgb::new_u8(16, 185, 129).resolve()),
        );
        install_color_signal::<HighlightColor>(
            env,
            Computed::constant(Srgb::new_u8(244, 114, 182).resolve()),
        );
    }
}

fn solid_background() -> impl View {
    Color::srgb(247, 249, 252)
}

fn centered_panel(content: impl View) -> impl View {
    zstack((solid_background(), content))
}

fn dark_text() -> Color {
    Color::srgb(20, 35, 57)
}

fn muted_text() -> Color {
    Color::srgb(89, 105, 128)
}

fn accent_blue() -> Color {
    Color::srgb(41, 111, 255)
}

fn swatch(color: impl Into<Color>, label: &'static str) -> impl View {
    let color: Color = color.into();
    vstack((
        color.size(74.0, 52.0).border(Color::srgb(255, 255, 255), 2.0),
        text(label).caption().foreground(muted_text()),
    ))
    .spacing(6.0)
}

#[preview]
fn barcode_qr_book_waterui_dev() -> impl View {
    centered_panel(
        Barcode::qr("https://book.waterui.dev")
            .dark_color(Color::srgb(12, 26, 45))
            .light_color(Color::srgb(255, 255, 255))
            .size(300.0, 300.0),
    )
}

#[preview]
fn barcode_custom_colors() -> impl View {
    centered_panel(
        hstack((
            Barcode::qr("https://book.waterui.dev")
                .linear_gradient(
                    Color::srgb(0, 108, 255),
                    Color::srgb(255, 62, 122),
                    UnitPoint::TOP_LEADING,
                    UnitPoint::BOTTOM_TRAILING,
                )
                .light_color(Color::srgb(246, 250, 255))
                .size(230.0, 230.0),
            vstack((
                text("Custom colors").headline().foreground(dark_text()),
                text("Solid, gradient, and quiet-zone colors stay crisp on the GPU.")
                    .foreground(muted_text()),
                Barcode::code128("WATERUI-BOOK")
                    .dark_color(Color::srgb(22, 103, 82))
                    .light_color(Color::srgb(241, 252, 246))
                    .size(250.0, 92.0),
            ))
            .spacing(16.0),
        ))
        .spacing(30.0)
        .padding(),
    )
}

#[preview]
fn modifiers_visual_stack() -> impl View {
    centered_panel(
        hstack((
            vstack((
                text("Modifiers").size(28.0).foreground(dark_text()),
                text("padding -> background -> border -> filter").foreground(muted_text()),
                text("Order changes what the renderer sees")
                    .padding()
                    .background(Color::srgb(230, 240, 255))
                    .border(accent_blue(), 2.0),
            ))
            .spacing(14.0)
            .padding(),
            vstack((
                Color::srgb(30, 82, 168)
                    .size(120.0, 68.0)
                    .opacity(0.84)
                    .border(Color::srgb(255, 255, 255), 2.0),
                Color::srgb(238, 86, 118)
                    .size(120.0, 68.0)
                    .blur(3.0)
                    .brightness(0.06),
                Color::srgb(255, 204, 82)
                    .size(120.0, 68.0)
                    .contrast(1.18),
            ))
            .spacing(12.0),
        ))
        .spacing(34.0)
        .padding(),
    )
}

#[preview]
fn text_typography_sample() -> impl View {
    centered_panel(
        vstack((
            text("Text and typography").size(28.0).foreground(dark_text()),
            text("Semantic fonts, explicit colors, and inline emphasis").foreground(muted_text()),
            hstack((
                text("Title").title().foreground(Color::srgb(21, 76, 170)),
                text("Headline").headline().foreground(Color::srgb(25, 128, 104)),
                text("Caption").caption().foreground(Color::srgb(113, 88, 180)),
            ))
            .spacing(24.0),
            vstack((
                text("Readable defaults first").headline().foreground(dark_text()),
                (text("Override only the pieces that ") + text("carry meaning").bold() + ".")
                    .foreground(muted_text())
                    .padding()
                    .background(Color::srgb(238, 243, 252)),
            ))
            .spacing(8.0),
        ))
        .spacing(18.0)
        .padding(),
    )
}

#[preview]
fn layout_stack_sample() -> impl View {
    centered_panel(
        hstack((
            vstack((
                text("VStack").headline().foreground(dark_text()),
                Color::srgb(80, 132, 255).size(118.0, 52.0),
                Color::srgb(31, 190, 164).size(118.0, 78.0),
                Color::srgb(255, 199, 79).size(118.0, 42.0),
            ))
            .spacing(12.0)
            .padding()
            .background(Color::srgb(238, 244, 255)),
            zstack((
                Color::srgb(248, 110, 136).size(172.0, 172.0),
                Color::srgb(255, 255, 255).size(112.0, 112.0).opacity(0.86),
                text("ZStack").headline().foreground(dark_text()),
            )),
            vstack((
                text("HStack").headline().foreground(dark_text()),
                hstack((
                    Color::srgb(26, 82, 146).size(52.0, 126.0),
                    Color::srgb(46, 181, 125).size(52.0, 126.0),
                    Color::srgb(247, 190, 68).size(52.0, 126.0),
                ))
                .spacing(10.0),
            ))
            .spacing(12.0)
            .padding()
            .background(Color::srgb(245, 247, 250)),
        ))
        .spacing(22.0)
        .padding(),
    )
}

#[preview]
fn controls_input_sample() -> impl View {
    let enabled = Binding::bool(true);
    let volume = Binding::f64(0.68);
    let seats = Binding::i32(3);

    centered_panel(
        vstack((
            text("Controls").size(28.0).foreground(dark_text()),
            hstack((
                button("Primary").bordered_prominent().action(|| {}),
                button("Secondary").bordered().action(|| {}),
            ))
            .spacing(12.0),
            toggle("Live preview", &enabled).switch(),
            slider("Volume", &volume)
                .min_value_label("0")
                .max_value_label("100"),
            stepper("Seats", &seats)
                .range(1..=8)
                .value_formatter(|value| format!("{value}")),
            progress(0.72).label("Syncing assets"),
        ))
        .spacing(16.0)
        .padding()
        .background(Color::srgb(255, 255, 255))
        .border(Color::srgb(226, 232, 240), 1.0),
    )
}

#[preview]
fn forms_data_entry_sample() -> impl View {
    let name = Binding::container(Str::from("Ava Rivers"));
    let marketing = Binding::bool(true);
    let seats = Binding::i32(4);
    let confidence = Binding::f64(0.62);

    centered_panel(
        vstack((
            text("Form controls").size(28.0).foreground(dark_text()),
            field("Display name", &name).prompt("Required"),
            toggle("Send release notes", &marketing),
            stepper("Team seats", &seats).range(1..=10),
            slider("Confidence", &confidence),
            hstack((
                text("Accent color").foreground(dark_text()),
                spacer(),
                Color::srgb(66, 119, 255)
                    .size(52.0, 36.0)
                    .border(Color::srgb(219, 226, 239), 1.0),
            ))
            .spacing(12.0),
        ))
        .spacing(14.0)
        .padding()
        .background(Color::srgb(255, 255, 255))
        .border(Color::srgb(226, 232, 240), 1.0),
    )
}

#[preview]
fn lists_sections_sample() -> impl View {
    centered_panel(
        List::content((
            Section::new("Today").content((
                row("Hydrolysis", "Preview renderer").value_color(accent_blue()),
                row("Book visuals", "Generated"),
                detail_row("Checkpoint", "Pinned WaterUI commit drives every row."),
            )),
            Section::new("Next").footer("Sections are native list metadata.").content((
                row("Audit", "Chapter links"),
                row("Deploy", "Cloudflare Pages"),
            )),
        ))
        .size(520.0, 300.0),
    )
}

#[preview]
fn navigation_stack_sample() -> impl View {
    NavigationStack::new(
        vstack((
            text("Project").headline().foreground(dark_text()),
            NavigationLink::new("Open details", || {
                vstack((
                    text("Details").size(28.0),
                    text("This destination is built lazily."),
                ))
                .title("Details")
                .inline_title()
            }),
            NavigationLink::new("Settings", || {
                vstack((text("Settings").size(28.0), text("Theme and preview options")))
                    .title("Settings")
                    .inline_title()
            }),
        ))
        .spacing(14.0)
        .padding()
        .title("Navigation")
        .large_title(),
    )
}

#[preview]
fn theme_color_resolver_sample() -> impl View {
    centered_panel(
        vstack((
            text("Custom color tokens").size(28.0).foreground(dark_text()),
            text("Resolvers keep semantic colors reactive across a subtree.")
                .foreground(muted_text()),
            hstack((
                swatch(theme_color::Accent, "Accent"),
                swatch(SuccessColor, "Success"),
                swatch(HighlightColor, "Highlight"),
                swatch(theme_color::Foreground, "Foreground"),
            ))
            .spacing(16.0),
            text("The view asks for tokens; the environment supplies pixels.")
                .padding()
                .background(Color::srgb(237, 242, 255))
                .foreground(dark_text()),
        ))
        .spacing(18.0)
        .padding(),
    )
    .install(BookTokens)
}

#[preview]
fn filter_frosted_gradient() -> impl View {
    zstack((
        Gradient::linear(
            vec![
                (0.0, working_color(0.10, 0.40, 0.92)),
                (0.45, working_color(0.18, 0.82, 0.70)),
                (1.0, working_color(0.98, 0.42, 0.55)),
            ],
            [0.0, 0.0],
            [1.0, 1.0],
        ),
        vstack((
            text("Filtered View").size(44.0).foreground(Color::srgb(255, 255, 255)),
            text("blur + brightness + contrast").foreground(Color::srgb(234, 242, 255)),
        ))
        .spacing(10.0)
        .padding(),
    ))
    .blur(7.0)
    .brightness(0.08)
    .contrast(1.18)
}

#[preview]
fn gradient_mesh() -> impl View {
    Gradient::mesh(
        3,
        3,
        vec![
            ([0.0, 0.0], working_color(0.08, 0.23, 0.42)),
            ([0.5, 0.0], working_color(0.10, 0.62, 0.72)),
            ([1.0, 0.0], working_color(0.95, 0.43, 0.52)),
            ([0.0, 0.5], working_color(0.29, 0.17, 0.58)),
            ([0.5, 0.5], working_color(1.00, 0.86, 0.38)),
            ([1.0, 0.5], working_color(0.22, 0.78, 0.52)),
            ([0.0, 1.0], working_color(0.03, 0.08, 0.18)),
            ([0.5, 1.0], working_color(0.20, 0.40, 0.88)),
            ([1.0, 1.0], working_color(0.88, 0.20, 0.46)),
        ],
        true,
    )
}

fn book_visual_catalog() -> impl View {
    vstack((
        vstack((
            barcode_qr_book_waterui_dev(),
            barcode_custom_colors(),
            modifiers_visual_stack(),
            text_typography_sample(),
            layout_stack_sample(),
            controls_input_sample(),
            forms_data_entry_sample(),
            lists_sections_sample(),
        )),
        vstack((
            navigation_stack_sample(),
            theme_color_resolver_sample(),
            filter_frosted_gradient(),
            gradient_mesh(),
        )),
    ))
}

pub fn app(environment: Environment) -> App {
    App::new(book_visual_catalog, environment)
}
