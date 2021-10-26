use super::{Color, Node, Text};
use bevy::{ecs::prelude::*, transform::prelude::*, window::Windows};
use epaint::{
    emath::*, tessellator::tessellate_shapes, text::Fonts, ClippedShape, Color32, Shape,
    TessellationOptions,
};

fn create_shapes_system(
    roots: Query<Entity, (With<Node>, Without<Parent>)>,
    shapes_q: Query<(&Node, Option<&Text>, Option<&Color>, Option<&Children>)>,
    fonts: Res<Fonts>,
    windows: Res<Windows>,
) {
    fn push_shapes(
        vec: &mut Vec<ClippedShape>,
        entity: Entity,
        clip: Rect,
        q: &Query<(&Node, Option<&Text>, Option<&Color>, Option<&Children>)>,
        fonts: &Fonts,
    ) {
        let (node, text, color, children) = q.get(entity).unwrap();
        let pos = Pos2::new(node.pos.x, node.pos.y);
        let color = color.map(|x| {
            let [r, g, b, a] = x.as_rgba_u8();
            Color32::from_rgba_premultiplied(r, g, b, a)
        });
        vec.push(ClippedShape(
            clip,
            match text {
                None => Shape::Rect {
                    rect: clip,
                    corner_radius: 0.,
                    fill: color.unwrap_or(Color32::TRANSPARENT),
                    stroke: Default::default(),
                },
                Some(text) => {
                    let galley = fonts.layout_multiline(text.style, text.text.clone(), node.size.x);
                    Shape::Text {
                        pos,
                        galley,
                        color: color.unwrap_or(Color32::WHITE),
                        fake_italics: false,
                    }
                }
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
