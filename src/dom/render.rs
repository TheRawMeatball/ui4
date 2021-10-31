use crate::dom::TextFont;

use super::{ClippedNode, Color, HideOverflow, Node, Text, TextDetails, TextSize};
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
        Option<&'static Node>,
        Option<&'static Text>,
        Option<&'static TextFont>,
        Option<&'static TextSize>,
        Option<&'static TextDetails>,
        Option<&'static Color>,
        Option<&'static HideOverflow>,
        Option<&'static Children>,
    ),
>;

pub(crate) fn create_shapes_system(
    mut local_shapes: Local<Vec<ClippedShape>>,
    roots: Query<Entity, (With<Node>, Without<Parent>)>,
    shapes_q: ShapeQ,
    mut cn_query: Query<&mut ClippedNode>,
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
        cn_query: &mut Query<&mut ClippedNode>,
        fonts: &Fonts,
        mut z: u32,
    ) {
        let (node, text, font, _size, details, color, hide_overflow, children) =
            q.get(entity).unwrap();
        let clip = if let Some(node) = node {
            let mut clipped = cn_query.get_mut(entity).unwrap();
            clipped.z_layer = z;
            let pos = Pos2::new(node.pos.x, node.pos.y);
            let color = color.map(|x| {
                let [r, g, b, a] = x.as_rgba_u8();
                Color32::from_rgba_unmultiplied(r, g, b, a)
            });
            let this_rect = Rect {
                min: pos,
                max: pos + Vec2::new(node.size.x, node.size.y),
            };
            clipped.min =
                bevy::math::Vec2::from(<[f32; 2]>::from(clip.clamp(this_rect.min).to_vec2()));
            clipped.max =
                bevy::math::Vec2::from(<[f32; 2]>::from(clip.clamp(this_rect.max).to_vec2()));
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

            z += 1;
            if hide_overflow.is_some() {
                this_rect
            } else {
                clip
            }
        } else {
            clip
        };

        for &child in children.map(|x| &**x).unwrap_or(&[]) {
            push_shapes(vec, child, clip, q, cn_query, fonts, z);
        }
    }

    let window = windows.get_primary().unwrap();
    let window_width = window.width();
    let window_height = window.height();
    for root in roots.iter() {
        push_shapes(
            &mut local_shapes,
            root,
            Rect {
                min: Pos2::ZERO,
                max: Pos2::new(window_width, window_height),
            },
            &shapes_q,
            &mut cn_query,
            &fonts,
            0,
        )
    }
    let res_shapes = &mut shapes.get_mut(&WindowId::primary()).unwrap().shapes;
    std::mem::swap(res_shapes, &mut local_shapes);
    res_shapes.extend(local_shapes.drain(..));
}
