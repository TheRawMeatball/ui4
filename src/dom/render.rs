use crate::dom::TextFont;

use super::{Color, Node, ShowOverflow, Text, TextDetails, TextSize};
use bevy::{
    ecs::prelude::*,
    transform::prelude::*,
    utils::HashMap,
    window::{WindowId, Windows},
};
use bevy_egui::{EguiContext, EguiShapes};
use epaint::{
    emath::*,
    text::{Fonts, LayoutJob, LayoutSection, TextFormat},
    ClippedShape, Color32, RectShape, Shape, Stroke, TextShape, TextStyle,
};

type ShapeQ<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        Option<&'static Text>,
        Option<&'static TextFont>,
        Option<&'static TextSize>,
        Option<&'static TextDetails>,
        Option<&'static Color>,
        Option<&'static ShowOverflow>,
        Option<&'static Children>,
    ),
>;

pub(crate) fn create_shapes_system(
    roots: Query<Entity, (With<Node>, Without<Parent>)>,
    shapes_q: ShapeQ,
    ctx: Res<EguiContext>,
    windows: Res<Windows>,
    mut shapes: ResMut<HashMap<WindowId, EguiShapes>>,
) {
    let fonts = ctx.ctx().fonts();
    fn push_shapes(
        vec: &mut Vec<ClippedShape>,
        entity: Entity,
        clip: Rect,
        q: &ShapeQ,
        fonts: &Fonts,
    ) {
        let (node, text, font, _size, details, color, show_overflow, children) =
            if let Ok(r) = q.get(entity) {
                r
            } else {
                return;
            };
        let pos = Pos2::new(node.pos.x, node.pos.y);
        let color = color.map(|x| {
            let [r, g, b, a] = x.as_rgba_u8();
            Color32::from_rgba_unmultiplied(r, g, b, a)
        });
        let this_rect = Rect {
            min: pos,
            max: pos + Vec2::new(node.size.x, node.size.y),
        };
        vec.push(ClippedShape(
            clip,
            if let Some(text) = text {
                let galley = fonts.layout_job(LayoutJob {
                    text: text.0.clone(),
                    wrap_width: node.size.x,
                    sections: details.map(|details| details.0.clone()).unwrap_or_else(|| {
                        vec![LayoutSection {
                            byte_range: 0..text.0.len(),
                            format: TextFormat {
                                style: font.map(|f| f.0).unwrap_or(TextStyle::Body),
                                color: color.unwrap_or(Color32::WHITE),
                                ..Default::default()
                            },
                            leading_space: 0.,
                        }]
                    }),
                    ..Default::default()
                });
                Shape::Text(TextShape {
                    pos,
                    galley,
                    override_text_color: None,
                    angle: 0.,
                    underline: Stroke::none(),
                })
            } else {
                Shape::Rect(RectShape {
                    rect: this_rect,
                    corner_radius: 0.,
                    fill: color.unwrap_or(Color32::TRANSPARENT),
                    stroke: Default::default(),
                })
            },
        ));

        for &child in children.map(|x| &**x).unwrap_or(&[]) {
            push_shapes(
                vec,
                child,
                if show_overflow.is_some() {
                    clip
                } else {
                    this_rect
                },
                q,
                fonts,
            );
        }
    }

    let old = &mut shapes.get_mut(&WindowId::primary()).unwrap().shapes;
    let mut shapes = vec![];
    let window = windows.get_primary().unwrap();
    let window_width = window.width();
    let window_height = window.height();
    for root in roots.iter() {
        push_shapes(
            &mut shapes,
            root,
            Rect {
                min: Pos2::ZERO,
                max: Pos2::new(window_width, window_height),
            },
            &shapes_q,
            &fonts,
        )
    }
    let old_owned = std::mem::replace(old, shapes);
    old.extend(old_owned);
}
