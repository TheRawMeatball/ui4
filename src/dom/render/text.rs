use std::sync::Arc;

use bevy::{prelude::Component, reflect::TypeUuid};
use fontdue::layout::{GlyphPosition, Layout, LayoutSettings, TextStyle};

#[derive(TypeUuid, Clone)]
#[uuid = "c4966996-7eaf-4a75-9179-d7cdd9ad2a96"]
pub struct FdFont(Arc<fontdue::Font>);

impl std::ops::Deref for FdFont {
    type Target = fontdue::Font;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for FdFont {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

// a font and it's fallbacks
#[derive(Clone)]
pub struct Font(Arc<[FdFont]>);

#[derive(Component)]
pub struct ProcessedText(Vec<GlyphPosition>);

pub struct FallbackFont(pub(crate) Font);

pub struct TextSection {
    pub char_len: usize,
    pub font: Font,
    pub size: f32,
}

impl ProcessedText {
    pub fn new(
        s: &str,
        layout: &mut Layout,
        settings: LayoutSettings,
        sections: &[TextSection],
        fallback_char: char,
    ) -> Self {
        layout.reset(&settings);

        let mut chunk_start = 0usize; // in bytes
        let mut chunk_end = 0usize; // in bytes
        let mut current_chunk_index = 0usize;
        let mut current_section = &sections[current_chunk_index];
        let mut current_section_len = 0usize; // in chars
        let mut current_fonts = &*current_section.font.0;
        let mut current_font = &current_fonts[0];

        for ch in s.chars() {
            if current_section.char_len == current_section_len {
                layout.append(
                    &[&*current_font.0],
                    &TextStyle {
                        text: &s[chunk_start..chunk_end],
                        px: current_section.size,
                        font_index: 0,
                        user_data: (),
                    },
                );
                current_chunk_index += 1;
                current_chunk_index = current_chunk_index.min(sections.len() - 1);
                current_section = &sections[current_chunk_index];
                current_fonts = &*current_section.font.0;
                chunk_start = chunk_end;
            }

            let font = current_fonts
                .iter()
                .find(|&f| f.lookup_glyph_index(ch) != 0);

            match font {
                Some(font) if font == current_font => {}
                Some(font) => {
                    layout.append(
                        &[&*current_font.0],
                        &TextStyle {
                            text: &s[chunk_start..chunk_end],
                            px: current_section.size,
                            font_index: 0,
                            user_data: (),
                        },
                    );
                    current_font = font;
                    chunk_start = chunk_end;
                }
                None => {
                    layout.append(
                        &[&*current_font.0],
                        &TextStyle {
                            text: &s[chunk_start..chunk_end],
                            px: current_section.size,
                            font_index: 0,
                            user_data: (),
                        },
                    );
                    chunk_end += ch.len_utf8();
                    chunk_start = chunk_end;
                    current_section_len += 1;
                    layout.append(
                        &[&*current_font.0],
                        &TextStyle {
                            text: &fallback_char.encode_utf8(&mut [0u8; 4]),
                            px: current_section.size,
                            font_index: 0,
                            user_data: (),
                        },
                    );
                    continue;
                }
            }

            chunk_end += ch.len_utf8();
            current_section_len += 1;
        }

        layout.append(
            &[&*current_font.0],
            &TextStyle {
                text: &s[chunk_start..chunk_end],
                px: current_section.size,
                font_index: 0,
                user_data: (),
            },
        );

        Self(layout.glyphs().clone())
    }
}
