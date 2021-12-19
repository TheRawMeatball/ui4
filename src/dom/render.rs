use super::{
    ClippedNode, HideOverflow, Node, Text, TextAlign, TextBoxCursor, TextDetails, TextSize,
};
use bevy::{
    ecs::prelude::*,
    math::{Mat4, Vec2, Vec3},
    prelude::{Assets, Color, Handle, HandleUntyped, Image, TextureAtlas},
    reflect::TypeUuid,
    render::RenderWorld,
    sprite::Rect,
    text::{DefaultTextPipeline, Font, FontAtlasSet, TextSection, TextStyle},
    transform::prelude::*,
    ui::{ExtractedUiNode, ExtractedUiNodes, UiColor, UiImage},
    window::Windows,
};

type ShapeQ<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Node>,
        Option<&'static TextBoxCursor>,
        Option<&'static TextDetails>,
        Option<&'static UiColor>,
        Option<&'static HideOverflow>,
        Option<&'static UiImage>,
        Option<&'static Children>,
    ),
>;

fn map_z(z: u32) -> f32 {
    0.001 + z as f32 / 100.
}

fn y_inv(pos: Vec2, window_height: f32) -> Vec2 {
    Vec2::new(0., window_height) + Vec2::new(1., -1.) * pos
}

pub const DEFAULT_FONT: HandleUntyped =
    HandleUntyped::weak_from_u64(Font::TYPE_UUID, 9182127759878421895);

fn push_shapes(
    vec: &mut Vec<ExtractedUiNode>,
    entity: Entity,
    clip: Rect,
    q: &ShapeQ,
    cn_query: &mut Query<&mut ClippedNode>,
    text_pipeline: &DefaultTextPipeline,
    images: &Assets<Image>,
    atlases: &Assets<TextureAtlas>,
    mut z: u32,
    window_height: f32,
    scale_factor: f32,
) {
    let (node, tb, text_details, color, hide_overflow, image, children) = q.get(entity).unwrap();

    let clip = if let Some(node) = node {
        let mut clipped = cn_query.get_mut(entity).unwrap();
        clipped.z_layer = z;
        let pos = node.pos;
        let color = color.map(|x| x.0);
        let this_rect = Rect {
            min: pos,
            max: pos + node.size,
        };
        clipped.min = clip.min.max(this_rect.min);
        clipped.max = clip.max.min(this_rect.max);
        if let Some(glyphs) = text_pipeline.get_glyphs(&entity) {
            let alignment_offset = (node.size / -2.0).extend(0.0);
            let text_details = text_details.map(|x| &*x.0).unwrap_or(&[]);

            let mut details = text_details
                .iter()
                .map(|(style, ends_at)| (style.color, *ends_at))
                .chain(std::iter::once((color.unwrap_or(Color::WHITE), usize::MAX)));

            let (mut cur_color, mut ends_at) = details.next().unwrap();
            for text_glyph in &glyphs.glyphs {
                if text_glyph.byte_index >= ends_at {
                    let (color, end) = details.next().unwrap();
                    cur_color = color;
                    ends_at = end;
                }

                let atlas = atlases
                    .get(text_glyph.atlas_info.texture_atlas.clone_weak())
                    .unwrap();

                let texture = atlas.texture.clone_weak();
                let index = text_glyph.atlas_info.glyph_index as usize;
                let rect = atlas.textures[index];
                let atlas_size = Some(atlas.size);

                let transform = Mat4::from_translation(
                    y_inv(pos + node.size / 2., window_height).extend(map_z(z)),
                ) * Mat4::from_scale(Vec3::ONE / scale_factor)
                    * Mat4::from_translation(
                        alignment_offset * scale_factor + text_glyph.position.extend(0.),
                    );

                vec.push(ExtractedUiNode {
                    transform,
                    color: cur_color,
                    rect,
                    image: texture,
                    atlas_size,
                });
            }

            let shape = tb
                .and_then(|tb| tb.0)
                .and_then(|cursor| glyphs.glyphs.iter().find(|g| g.byte_index == cursor))
                .map(|glyph| {
                    let glyph_pos = glyph.position;

                    ExtractedUiNode {
                        transform: Mat4::from_translation(
                            y_inv(pos + glyph_pos, window_height).extend(map_z(z) + 0.1),
                        ),
                        color: Color::WHITE,
                        rect: Rect {
                            min: Vec2::ZERO,
                            max: Vec2::new(2., 14.),
                        },
                        image: bevy::render::texture::DEFAULT_IMAGE_HANDLE.typed(),
                        atlas_size: None,
                    }
                });
            vec.extend(shape);
        } else if image
            .map(|img| images.contains(img.0.clone_weak()))
            .unwrap_or(true)
        {
            let pos = y_inv(pos + node.size / 2., window_height);
            vec.push(ExtractedUiNode {
                transform: Mat4::from_translation(pos.extend(map_z(z))),
                color: color.unwrap_or(Color::NONE),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: node.size,
                },
                image: image
                    .map(|i| i.0.clone_weak())
                    .unwrap_or(bevy::render::texture::DEFAULT_IMAGE_HANDLE.typed()),
                atlas_size: None,
            });
        }

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
        push_shapes(
            vec,
            child,
            clip,
            q,
            cn_query,
            text_pipeline,
            images,
            atlases,
            z,
            window_height,
            scale_factor,
        );
    }
}

#[derive(Default)]
pub(crate) struct PreExtractedUiNodes(Vec<ExtractedUiNode>);

pub(crate) fn create_shapes_system(
    roots: Query<Entity, (With<Node>, Without<Parent>)>,
    shapes_q: ShapeQ,
    mut cn_query: Query<&mut ClippedNode>,
    windows: Res<Windows>,
    text_pipeline: Res<DefaultTextPipeline>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlas>>,
    mut shapes: ResMut<PreExtractedUiNodes>,
) {
    let window = if let Some(w) = windows.get_primary() {
        w
    } else {
        return;
    };
    let window_width = window.width();
    let window_height = window.height();
    for root in roots.iter() {
        push_shapes(
            &mut shapes.0,
            root,
            Rect {
                min: Vec2::ZERO,
                max: Vec2::new(window_width, window_height),
            },
            &shapes_q,
            &mut cn_query,
            &text_pipeline,
            &images,
            &atlases,
            0,
            window_height,
            window.scale_factor() as f32,
        );
    }
}

// todo: replace with layout-driven method when morphorm supports it
pub(crate) fn process_text_system(
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    text_nodes: Query<(
        Entity,
        &Node,
        (&Text, ChangeTrackers<Text>),
        Option<(&TextSize, ChangeTrackers<TextSize>)>,
        Option<(&Handle<Font>, ChangeTrackers<Handle<Font>>)>,
        Option<(&TextDetails, ChangeTrackers<TextDetails>)>,
        Option<(&TextAlign, ChangeTrackers<TextAlign>)>,
    )>,
    fonts: Res<Assets<Font>>,

    mut textures: ResMut<Assets<Image>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,

    windows: Res<Windows>,
) {
    fn check_tuple<T: Component>(x: &Option<(&T, ChangeTrackers<T>)>) -> bool {
        x.as_ref().map(|(_, x)| x.is_changed()).unwrap_or(false)
    }

    for (entity, node, text, size, font, details, align) in text_nodes.iter() {
        // if !(text.1.is_changed()
        //     || check_tuple(&size)
        //     || check_tuple(&font)
        //     || check_tuple(&details)
        //     || check_tuple(&align))
        // {
        //     continue;
        // }

        let text_details = details.map(|x| &*x.0 .0).unwrap_or(&[]);
        let mut style = None;
        let mut start = 0;

        let sections = text_details
            .iter()
            .map(|(style, ends_at)| (style, *ends_at))
            .chain(std::iter::once_with(|| {
                style = Some(TextStyle {
                    font: font
                        .map(|f| f.0.clone_weak())
                        .unwrap_or_else(|| DEFAULT_FONT.typed()),
                    font_size: size.map(|s| s.0 .0).unwrap_or(14.),
                    color: Color::WHITE, // color not relevant to layout
                });
                (style.as_ref().unwrap(), text.0 .0.as_bytes().len())
            }))
            .map(|(style, end)| {
                let r = TextSection {
                    value: text.0 .0[start..end].to_owned(),
                    style: style.clone(),
                };
                start = end;
                r
            })
            .collect::<Vec<_>>();

        text_pipeline
            .queue_text(
                entity,
                &fonts,
                &sections,
                windows
                    .get_primary()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.),
                align.map(|(a, _)| a.0).unwrap_or_default(),
                bevy::math::Size::new(node.size.x, node.size.y),
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
            )
            .ok();
    }
}

pub(crate) fn move_uinodes(
    mut render_world: ResMut<RenderWorld>,
    mut prepared: ResMut<PreExtractedUiNodes>,
) {
    let mut extracted_uinodes = render_world.get_resource_mut::<ExtractedUiNodes>().unwrap();
    extracted_uinodes
        .uinodes
        .extend(std::mem::take(&mut prepared.0).into_iter());
}
