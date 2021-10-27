use crate::dom::TextFont;

use super::{Color, Node, Text};
use bevy::{ecs::prelude::*, transform::prelude::*, window::Windows};
use epaint::{
    emath::*,
    tessellator::tessellate_shapes,
    text::{Fonts, LayoutJob},
    ClippedShape, Color32, RectShape, Shape, Stroke, TessellationOptions, TextShape,
};

type ShapeQ<'w, 's> = Query<
    'w,
    's,
    (
        &'static Node,
        Option<&'static Text>,
        Option<&'static TextFont>,
        Option<&'static Color>,
        Option<&'static Children>,
    ),
>;

fn create_shapes_system(
    roots: Query<Entity, (With<Node>, Without<Parent>)>,
    shapes_q: ShapeQ,
    fonts: Res<Fonts>,
    windows: Res<Windows>,
) {
    fn push_shapes(
        vec: &mut Vec<ClippedShape>,
        entity: Entity,
        clip: Rect,
        q: &ShapeQ,
        fonts: &Fonts,
    ) {
        let (node, text, font, color, children) = q.get(entity).unwrap();
        let pos = Pos2::new(node.pos.x, node.pos.y);
        let color = color.map(|x| {
            let [r, g, b, a] = x.as_rgba_u8();
            Color32::from_rgba_unmultiplied(r, g, b, a)
        });
        vec.push(ClippedShape(
            clip,
            if let Some(text) = text {
                fonts.layout_job(LayoutJob::default());
                let galley = fonts.layout_delayed_color(
                    text.0.clone(),
                    font.map(|f| f.0).unwrap_or(epaint::TextStyle::Body),
                    node.size.x,
                );
                Shape::Text(TextShape {
                    pos,
                    galley,
                    override_text_color: color,
                    angle: 0.,
                    underline: Stroke::none(),
                })
            } else {
                Shape::Rect(RectShape {
                    rect: clip,
                    corner_radius: 0.,
                    fill: color.unwrap_or(Color32::TRANSPARENT),
                    stroke: Default::default(),
                })
            },
        ));
        let clip = Rect {
            min: pos,
            max: pos + Vec2::new(node.size.x, node.size.y),
        };
        for &child in children.map(|x| &**x).unwrap_or(&[]) {
            push_shapes(vec, child, clip, q, fonts);
        }
    }

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
    let meshes = tessellate_shapes(
        shapes,
        TessellationOptions::default(),
        fonts.texture().size(),
    );
}
