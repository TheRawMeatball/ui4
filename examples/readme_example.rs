//! Keep this in sync with the readme

use bevy::prelude::*;
use ui4::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        .add_plugin(Ui4Root(root));

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: Ctx) -> Ctx {
    #[derive(Component)]
    struct State(i32);

    let state = ctx.component();
    let this = ctx.current_entity();

    ctx.with(State(0))
        .with(Top(Units::Pixels(50.)))
        .with(Left(Units::Pixels(50.)))
        .with(UiColor(Color::BLACK))
        .with(Height(Units::Auto))
        .with(Width(Units::Auto))
        .child(text("Hello!").with(Height(Units::Pixels(30.))))
        .child(|ctx| {
            ctx.with(Width(Units::Pixels(300.)))
                .with(Height(Units::Pixels(30.)))
                .with(LayoutType::Row)
                .child(button("Increment").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 += 1;
                })))
                .child(button("Decrement").with(OnClick::new(move |world| {
                    world.get_mut::<State>(this).unwrap().0 -= 1;
                })))
        })
        .child(
            text(state.map(|s: &State| format!("The number is {}", s.0)))
                .with(Height(Units::Pixels(30.))),
        )
}
