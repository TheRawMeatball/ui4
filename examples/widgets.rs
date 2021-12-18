use bevy::prelude::*;
use derive_more::{Deref, DerefMut};
use std::hash::Hash;
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root));

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct TextboxText(String);

    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct CheckboxData(bool);

    #[derive(Component, Hash, Copy, Clone, PartialEq, Eq)]
    enum RadioButtonSelect {
        A,
        B,
        C,
    }

    #[derive(Component, Deref, Lens)]
    struct Slider(f32);

    let textbox_text = ctx.component();
    let checkbox_data = ctx.component();
    let radiobutton = ctx.component();

    let slider_percent = ctx.component();

    ctx.with(TextboxText::default())
        .with(CheckboxData::default())
        .with(RadioButtonSelect::A)
        .with(Slider(0.42))
        .with(UiColor(Color::BLACK))
        .children(|ctx: &mut McCtx| {
            ctx.c(labelled_widget(
                "Button",
                button("Click me!").with(OnClick::new(|_| println!("you clicked the button!"))),
            ))
            .c(labelled_widget(
                "Textbox",
                textbox(textbox_text.lens(TextboxText::F0)),
            ))
            .c(labelled_widget(
                "Checkbox",
                checkbox(checkbox_data.lens(CheckboxData::F0)),
            ))
            .c(labelled_widget("Radio buttons", |ctx| {
                ctx.with(Width(Units::Pixels(250.)))
                    .with(Height(Units::Pixels(30.)))
                    .with(LayoutType::Row)
                    .with(ColBetween(Units::Stretch(1.)))
                    .children(|ctx: &mut McCtx| {
                        ctx.c(radio_button(RadioButtonSelect::A, radiobutton))
                            .c(text("A  "))
                            .c(radio_button(RadioButtonSelect::B, radiobutton))
                            .c(text("B  "))
                            .c(radio_button(RadioButtonSelect::C, radiobutton))
                            .c(text("C  "));
                    })
            }))
            .c(labelled_widget(
                "Dropdown",
                dropdown(
                    [
                        (RadioButtonSelect::A, "A"),
                        (RadioButtonSelect::B, "B"),
                        (RadioButtonSelect::C, "C"),
                    ],
                    radiobutton,
                ),
            ))
            .c(labelled_widget(
                "Progress",
                progressbar(slider_percent.dereffed().copied()),
            ))
            .c(labelled_widget(
                "Slider",
                slider(slider_percent.lens(Slider::F0)),
            ))
            .c(labelled_widget(
                "Tweened",
                progressbar(
                    textbox_text
                        .map(|t: &TextboxText| t.0.parse::<f32>().unwrap_or(0.42).clamp(0., 1.))
                        .dedup()
                        .copied()
                        .tween(0.2),
                ),
            ))
            .c(toggle(|| {
                toggle(|| text_fade("Hey!").with(Height(Units::Pixels(30.))))
            }));
        })
}

fn labelled_widget(
    label: &'static str,
    widget: impl FnOnce(Ctx) -> Ctx,
) -> impl FnOnce(Ctx) -> Ctx {
    move |ctx: Ctx| {
        ctx.with(Width(Units::Pixels(400.)))
            .with(Height(Units::Pixels(30.)))
            .with(LayoutType::Row)
            .children(|ctx: &mut McCtx| {
                ctx.c(text(label)
                    .with(Width(Units::Pixels(150.)))
                    .with(Height(Units::Pixels(30.))))
                    .c(widget);
            })
    }
}

fn toggle<F: FnOnce(Ctx) -> Ctx>(
    child: impl Fn() -> F + Send + Sync + 'static,
) -> impl FnOnce(Ctx) -> Ctx {
    #[derive(Component, Deref, DerefMut, Default, Lens)]
    struct Toggle(bool);
    |ctx: Ctx| {
        let checked = ctx.component::<Toggle>();
        ctx.with(Toggle(false))
            .child(checkbox(checked.lens(Toggle::F0)))
            .children(checked.dereffed().copied().map_child(move |b| {
                let child = child();
                move |ctx: &mut McCtx| {
                    if b {
                        ctx.c(child);
                    }
                }
            }))
    }
}
