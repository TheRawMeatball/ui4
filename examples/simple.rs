use bevy::prelude::*;
use std::borrow::Borrow;
use ui4::{init_ui, res, Ctx, McCtx, ObserverExt, Ui4Plugin};
use ui4::{ButtonFunc, IntoObserver};

struct UiAssets {
    background: Handle<ColorMaterial>,
    button: Handle<ColorMaterial>,
    text_style: TextStyle,
}

fn init_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(UiAssets {
        background: assets.add(Color::BLACK.into()),
        button: assets.add(Color::GRAY.into()),
        text_style: TextStyle {
            color: Color::WHITE,
            font: asset_server.load("FiraMono-Medium.ttf"),
            font_size: 32.0,
        },
    })
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Ui4Plugin)
        // .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_startup_system(init_system)
        .add_startup_system(
            (|world: &mut World| init_ui(world, root))
                .exclusive_system()
                .at_end(),
        );

    app.world.spawn().insert_bundle(UiCameraBundle::default());

    app.run()
}

fn root(ctx: &mut Ctx) {
    #[derive(Component)]
    struct State(i32);

    let state = ctx.component();
    let this = ctx.this();

    ctx.with_bundle(NodeBundle::default())
        .with(Style {
            size: Size {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Default::default()
        })
        .with(res().map(|assets: &UiAssets| assets.background.clone()))
        .with(State(0))
        .children(|ctx: &mut McCtx| {
            ctx.ctx(text("Hello!".to_string()))
                .ctx(text("How are you doing?".to_string()))
                .ctx(button(
                    "Increment".to_string(),
                    ButtonFunc::new(move |world| {
                        world.get_mut::<State>(this).unwrap().0 += 1;
                    }),
                ))
                .ctx(button(
                    "Decrement".to_string(),
                    ButtonFunc::new(move |world| {
                        world.get_mut::<State>(this).unwrap().0 -= 1;
                    }),
                ))
                .ctx(text(
                    state.map(|s: &State| format!("The number is {}", s.0)),
                ));
        })
        .children(
            res()
                .map(|time: &Time| time.seconds_since_startup() as usize % 2 == 0)
                .dedup()
                .map(|b: &bool| {
                    let b = *b;
                    move |ctx: &mut McCtx| {
                        if b {
                            ctx.ctx(text("Now you see me".to_string()));
                        }
                    }
                }),
        );
}

fn text<O: IntoObserver<String, M>, M>(text: O) -> impl Fn(&mut Ctx) {
    move |ctx: &mut Ctx| {
        ctx.with_bundle(TextBundle::default())
            .with(Style {
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            })
            .with(res().and(text.clone().into_observable()).map(
                move |(assets, text): (&UiAssets, O::ObserverReturn<'_, '_>)| {
                    Text::with_section(text.borrow(), assets.text_style.clone(), Default::default())
                },
            ));
    }
}

fn button<O: IntoObserver<String, M>, M: 'static>(
    t: O,
    button_func: ButtonFunc,
) -> impl Fn(&mut Ctx) {
    move |ctx: &mut Ctx| {
        ctx.with_bundle(ButtonBundle::default())
            .with(Style {
                size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            })
            .with(res().map(|assets: &UiAssets| assets.button.clone()))
            .with(button_func.clone())
            .child(text(t.clone()));
    }
}
