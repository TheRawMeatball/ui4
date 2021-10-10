use bevy::ecs::prelude::*;
use morphorm::{Node, Units};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeEntity(pub Entity);

impl NodeEntity {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

macro_rules! derive_all {
    ($(
        $name:ident($unit_type:ident);
    )*) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Component)]
            pub struct $name(pub $unit_type);
        )*
    };
}

macro_rules! func_all {
    (
        $ret:ty;
        $(
            [$func:ident, $typ:ident],
        )*
    ) => {
        $(
            fn $func(&self, store: &'_ Self::Data) -> Option<$ret> {
                store.get_component::<layout_components::$typ>(self.0).map(|x| x.0.clone()).ok()
            }
        )*
    };
}

macro_rules! query_all {
    ($last:ident,) => {
        &'static layout_components::$last
    };
    ($first:ident, $($rest:ident,)*) => {
        (&'static layout_components::$first, query_all!($($rest,)*))
    };
}

pub mod layout_components {
    use super::*;
    #[derive(Debug, Clone, Copy, PartialEq, Component)]
    pub enum PositionType {
        /// Node is positioned relative to parent but ignores its siblings
        SelfDirected,
        /// Node is positioned relative to parent and in-line with siblings
        ParentDirected,
    }
    #[derive(Debug, Clone, Copy, PartialEq, Component)]
    pub enum LayoutType {
        /// Stack child elements horizontally
        Row,
        /// Stack child elements vertically
        Column,
        /// Position child elements into specified rows and columns
        Grid,
    }
    derive_all!(
        Left(Units);
        Right(Units);
        Top(Units);
        Bottom(Units);
        MinLeft(Units);
        MaxLeft(Units);
        MinRight(Units);
        MaxRight(Units);
        MinTop(Units);
        MaxTop(Units);
        MinBottom(Units);
        MaxBottom(Units);
        Width(Units);
        Height(Units);
        MinWidth(Units);
        MaxWidth(Units);
        MinHeight(Units);
        MaxHeight(Units);
        ChildLeft(Units);
        ChildRight(Units);
        ChildTop(Units);
        ChildBottom(Units);
        RowBetween(Units);
        ColBetween(Units);
        RowIndex(usize);
        ColIndex(usize);
        RowSpan(usize);
        ColSpan(usize);
        Border(Units);
    );
    #[derive(Debug, Clone, PartialEq, Component)]
    pub struct GridRows(pub Vec<Units>);
    #[derive(Debug, Clone, PartialEq, Component)]
    pub struct GridCols(pub Vec<Units>);
}

impl<'w> Node<'w> for NodeEntity {
    type Data = &'w Query<
        'w,
        'w,
        query_all![
            Left,
            Right,
            Top,
            Bottom,
            MinLeft,
            MaxLeft,
            MinRight,
            MaxRight,
            MinTop,
            MaxTop,
            MinBottom,
            MaxBottom,
            Width,
            Height,
            MinWidth,
            MaxWidth,
            MinHeight,
            MaxHeight,
            ChildLeft,
            ChildRight,
            ChildTop,
            ChildBottom,
            RowBetween,
            ColBetween,
            RowIndex,
            ColIndex,
            RowSpan,
            ColSpan,
            Border,
            PositionType,
            LayoutType,
            GridRows,
            GridCols,
        ],
    >;

    fn layout_type(&self, store: &'_ Self::Data) -> Option<morphorm::LayoutType> {
        store
            .get_component::<layout_components::LayoutType>(self.0)
            .map(|x| match x {
                layout_components::LayoutType::Row => morphorm::LayoutType::Row,
                layout_components::LayoutType::Column => morphorm::LayoutType::Column,
                layout_components::LayoutType::Grid => morphorm::LayoutType::Grid,
            })
            .ok()
    }

    fn position_type(&self, store: &'_ Self::Data) -> Option<morphorm::PositionType> {
        store
            .get_component::<layout_components::PositionType>(self.0)
            .map(|x| match x {
                layout_components::PositionType::ParentDirected => {
                    morphorm::PositionType::ParentDirected
                }
                layout_components::PositionType::SelfDirected => {
                    morphorm::PositionType::SelfDirected
                }
            })
            .ok()
    }

    func_all!(
        Units;
        [width, Width],
        [height, Height],
        [min_width, MinWidth],
        [min_height, MinHeight],
        [max_width, MaxWidth],
        [max_height, MaxHeight],
        [left, Left],
        [right, Right],
        [top, Top],
        [bottom, Bottom],
        [max_left, MaxLeft],
        [max_right, MaxRight],
        [max_top, MaxTop],
        [max_bottom, MaxBottom],
        [min_left, MinLeft],
        [min_right, MinRight],
        [min_top, MinTop],
        [min_bottom, MinBottom],
        [child_left, ChildLeft],
        [child_right, ChildRight],
        [child_top, ChildTop],
        [child_bottom, ChildBottom],
        [row_between, RowBetween],
        [col_between, ColBetween],
        [border_left, Border],
        [border_right, Border],
        [border_top, Border],
        [border_bottom, Border],
    );

    func_all!(
        Vec<Units>;
        [grid_rows, GridRows],
        [grid_cols, GridCols],
    );

    func_all!(
        usize;
        [row_index, RowIndex],
        [col_index, ColIndex],
        [row_span, RowSpan],
        [col_span, ColSpan],
    );
}
