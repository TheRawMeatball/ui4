use bevy::ecs::prelude::*;
use morphorm::{Hierarchy, Node, Units};

use super::{Control, FirstChild, NextSibling, Parent};

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

#[derive(Clone, Copy)]
pub struct Tree<'borrow, 'world, 'state> {
    root: Entity,
    parent_query: &'borrow Query<'world, 'state, &'static Parent>,
    first_child_query: &'borrow Query<'world, 'state, &'static FirstChild>,
    next_sibling_query: &'borrow Query<'world, 'state, &'static NextSibling>,
    control_node_query: &'borrow Query<'world, 'state, &'static Control>,
}

impl<'borrow, 'world, 'state> Tree<'borrow, 'world, 'state> {
    pub fn new(
        root: Entity,
        parent_query: &'borrow Query<'world, 'state, &'static Parent>,
        first_child_query: &'borrow Query<'world, 'state, &'static FirstChild>,
        next_sibling_query: &'borrow Query<'world, 'state, &'static NextSibling>,
        control_node_query: &'borrow Query<'world, 'state, &'static Control>,
    ) -> Self {
        Self {
            root,
            parent_query,
            first_child_query,
            next_sibling_query,
            control_node_query,
        }
    }
}

impl<'borrow, 'world, 'state> Tree<'borrow, 'world, 'state> {
    pub fn flatten(&self) -> Vec<NodeEntity> {
        let iterator = DownwardIterator {
            tree: *self,
            current_node: Some(self.root),
        };

        iterator.collect::<Vec<_>>()
    }

    pub fn get_first_child(&self, node: Entity) -> Option<Entity> {
        let mut first = self.first_child_query.get(node).map(|f| f.0).ok();

        while let Some(entity) = first {
            if let Ok(_control_node) = self.control_node_query.get(entity) {
                first = self.first_child_query.get(entity).map(|f| f.0).ok();
            } else {
                break;
            }
        }

        first
    }

    pub fn get_next_sibling(&self, node: Entity) -> Option<Entity> {
        let mut next = self.next_sibling_query.get(node).map(|ns| ns.0).ok();

        while let Some(entity) = next {
            if let Ok(_control_node) = self.control_node_query.get(entity) {
                next = self.next_sibling_query.get(entity).map(|ns| ns.0).ok();
            } else {
                break;
            }
        }

        next
    }
}

impl<'borrow, 'world, 'state> Hierarchy<'borrow> for Tree<'borrow, 'world, 'state> {
    type Item = NodeEntity;
    type DownIter = std::vec::IntoIter<NodeEntity>;
    type UpIter = std::iter::Rev<std::vec::IntoIter<NodeEntity>>;
    type ChildIter = ChildIterator<'borrow, 'world, 'state>;

    fn up_iter(&self) -> Self::UpIter {
        self.flatten().into_iter().rev()
    }

    fn down_iter(&self) -> Self::DownIter {
        self.flatten().into_iter()
    }

    fn parent(&self, node: Self::Item) -> Option<Self::Item> {
        self.parent_query
            .get(node.entity())
            .map_or(None, |parent| Some(NodeEntity(parent.0)))
    }

    fn child_iter(&'borrow self, node: Self::Item) -> Self::ChildIter {
        ChildIterator {
            next_sibling_query: &self.next_sibling_query,
            current_node: self.get_first_child(node.entity()).map(NodeEntity),
        }
    }

    fn is_first_child(&self, node: Self::Item) -> bool {
        if let Some(parent) = self.parent(node) {
            if let Some(first_child) = self.get_first_child(parent.entity()) {
                if first_child == node.entity() {
                    return true;
                }
            }
        }

        false
    }

    fn is_last_child(&self, node: Self::Item) -> bool {
        if let Some(parent) = self.parent(node) {
            if let Some(mut temp) = self.get_first_child(parent.entity()) {
                while let Some(next_sibling) = self.get_next_sibling(temp) {
                    temp = next_sibling;
                }

                if temp == node.entity() {
                    return true;
                }
            }
        }

        false
    }
}

pub struct DownwardIterator<'borrow, 'world, 'state> {
    tree: Tree<'borrow, 'world, 'state>,
    current_node: Option<Entity>,
}

impl<'borrow, 'world, 'state> Iterator for DownwardIterator<'borrow, 'world, 'state> {
    type Item = NodeEntity;
    fn next(&mut self) -> Option<NodeEntity> {
        let r = self.current_node;

        if let Some(current) = self.current_node {
            if let Some(first_child) = self.tree.get_first_child(current) {
                self.current_node = Some(first_child);
            } else {
                let mut temp = Some(current);
                while let Some(entity) = temp {
                    if let Some(next_sibling) = self.tree.get_next_sibling(entity) {
                        self.current_node = Some(next_sibling);
                        return r.map(NodeEntity);
                    } else {
                        temp = self.tree.parent(NodeEntity(entity)).map(|p| p.0);
                    }
                }

                self.current_node = None;
            }
        }

        return r.map(NodeEntity);
    }
}

pub struct ChildIterator<'borrow, 'world, 'state> {
    pub next_sibling_query: &'borrow Query<'world, 'state, &'static NextSibling>,
    pub current_node: Option<NodeEntity>,
}

impl<'borrow, 'world, 'state> Iterator for ChildIterator<'borrow, 'world, 'state> {
    type Item = NodeEntity;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entity) = self.current_node {
            //self.current_node = self.tree.next_sibling[entity.index()].as_ref();
            self.current_node = self
                .next_sibling_query
                .get(entity.entity())
                .map_or(None, |next_sibling| Some(NodeEntity(next_sibling.0)));
            return Some(entity);
        }

        None
    }
}
