use bevy::prelude::*;
use ui4_2::{init_ui, res, Ctx, ObserverExt, Ui4Plugin};

struct UiAssets {
    background: Handle<ColorMaterial>,
    text_style: TextStyle,
}

fn init_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(UiAssets {
        background: assets.add(Color::BLACK.into()),
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
        .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_startup_system(init_system);

    app.world.spawn().insert_bundle(UiCameraBundle::default());
    init_ui(&mut app.world, root);

    app.run()
}

fn root(ctx: &mut Ctx) {
    ctx.insert_bundle(NodeBundle::default())
        .insert(Style {
            size: Size {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
            },
            flex_direction: FlexDirection::ColumnReverse,
            ..Default::default()
        })
        .insert_dynamic(res::<UiAssets>().map(|assets| assets.background.clone()))
        .static_child(|ctx| {
            ctx.insert_bundle(TextBundle::default())
                .insert(Style {
                    align_self: AlignSelf::FlexStart,
                    ..Default::default()
                })
                .insert_dynamic(res::<UiAssets>().map(|assets| {
                    Text::with_section("Hello!", assets.text_style.clone(), Default::default())
                }));
        })
        .static_child(|ctx| {
            ctx.insert_bundle(TextBundle::default())
                .insert(Style {
                    align_self: AlignSelf::FlexStart,
                    ..Default::default()
                })
                .insert_dynamic(res::<UiAssets>().map(|assets| {
                    Text::with_section(
                        "How are you doing!",
                        assets.text_style.clone(),
                        Default::default(),
                    )
                }));
        });
}
