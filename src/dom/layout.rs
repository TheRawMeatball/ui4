use bevy::{
    ecs::prelude::*,
    ecs::system::SystemParam,
    math::Vec2,
    prelude::{Children, Parent},
    utils::HashMap,
};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use concat_idents::concat_idents;
use derive_more::{Deref, DerefMut};
use morphorm::{Cache, Hierarchy, Node};

#[derive(Debug, Clone, Copy, PartialEq, Inspectable)]
pub enum Units {
    Pixels(f32),
    Percentage(f32),
    Stretch(f32),
    Auto,
}

impl From<Units> for morphorm::Units {
    fn from(this: Units) -> Self {
        match this {
            Units::Pixels(v) => morphorm::Units::Pixels(v),
            Units::Percentage(v) => morphorm::Units::Percentage(v),
            Units::Stretch(v) => morphorm::Units::Stretch(v),
            Units::Auto => morphorm::Units::Auto,
        }
    }
}

use super::{Control, Node as UiNode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeEntity(pub Entity);

impl NodeEntity {
    pub fn entity(&self) -> Entity {
        self.0
    }
}

macro_rules! derive_all {
    ($(
        $name:ident($unit_type:ty);
    )*) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Component, Deref, DerefMut, Inspectable)]
            pub struct $name(pub $unit_type);
        )*
        #[allow(unused)]
        pub(crate) fn register_all(app: &mut bevy::app::App) {
            $(
                app.register_inspectable::<$name>();
            )*
        }
    };
}

macro_rules! func_all {
    (
        $ret:ty;
        $([$func:ident, $typ:ident],)*
    ) => {
        $(
            fn $func(&self, store: &'_ Self::Data) -> Option<$ret> {
                store.get_component::<layout_components::$typ>(self.0)
                    .map(|x| x.0.clone())
                    .ok()
                    .map(|v| v.into())
            }
        )*
    };
    (void $($tt:tt)*) => {}
}

macro_rules! get_func_all {
    ($($func:ident,)*) => {
        $(
            fn $func(&self, node: Self::Item) -> f32 {
                self.cache
                    .$func
                    .get(&node)
                    .copied()
                    .unwrap_or_default()
            }
        )*
    };
}

macro_rules! set_func_all {
    ($($func:ident,)*) => {
        $(
            concat_idents!(fn_name = set_, $func {
                fn fn_name(&mut self, node: Self::Item, value: f32) {
                    *self.cache.$func.entry(node).or_default() = value;
                }
            });
        )*
    };
}

macro_rules! query_all {
    ($last:ident,) => {
        Option<&'static layout_components::$last>
    };
    ($first:ident, $($rest:ident,)*) => {
        (Option<&'static layout_components::$first>, query_all!($($rest,)*))
    };
}

macro_rules! cd_all {
    ($last:ident,) => {
        Option<ChangeTrackers<layout_components::$last>>
    };
    ($first:ident, $($rest:ident,)*) => {
        (Option<ChangeTrackers<layout_components::$first>>, cd_all!($($rest,)*))
    };
}

macro_rules! tuple_expand {
    ($last:ident,) => {
        $last
    };
    ($first:ident, $($rest:ident,)*) => {
        ($first, tuple_expand!($($rest,)*))
    };
}

macro_rules! generate {
    ($($list:ident,)*) => {
        type StyleQuery = query_all![$($list,)*];
        type CdQuery = cd_all![$($list,)*];

        fn check_cd(entity: Entity, query: &Query<CdQuery>) -> bool {
            #[allow(non_snake_case)]
            let tuple_expand!($($list,)*) = query.get(entity).unwrap();
            false $(|| $list.map(|q| q.is_changed()).unwrap_or(false) )*
        }
    };
}

pub mod layout_components {
    use super::*;
    #[derive(Debug, Clone, Copy, PartialEq, Component, Inspectable)]
    pub enum PositionType {
        /// Node is positioned relative to parent but ignores its siblings
        SelfDirected,
        /// Node is positioned relative to parent and in-line with siblings
        ParentDirected,
    }
    #[derive(Debug, Clone, Copy, PartialEq, Component, Inspectable)]
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

generate![
    Width,
    Height,
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
];

impl<'a> Node<'a> for NodeEntity {
    type Data = Query<'a, 'a, StyleQuery>;

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
        morphorm::Units;
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

    fn grid_rows(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get_component::<layout_components::GridRows>(self.0)
            .map(|x| x.0.clone())
            .ok()
            .map(|v| v.into_iter().map(|v| v.into()).collect())
    }
    fn grid_cols(&self, store: &'_ Self::Data) -> Option<Vec<morphorm::Units>> {
        store
            .get_component::<layout_components::GridCols>(self.0)
            .map(|x| x.0.clone())
            .ok()
            .map(|v| v.into_iter().map(|v| v.into()).collect())
    }

    func_all!(
        usize;
        [row_index, RowIndex],
        [col_index, ColIndex],
        [row_span, RowSpan],
        [col_span, ColSpan],
    );
}

#[derive(SystemParam)]
pub(crate) struct TreeQueries<'w, 's> {
    parent_query: Query<'w, 's, &'static Parent>,
    children_query: Query<'w, 's, &'static Children>,
    control_node_query: Query<'w, 's, &'static Control>,
}

#[derive(Clone, Copy)]
pub struct Tree<'borrow, 'world, 'state> {
    list: &'borrow [NodeEntity],
    queries: &'borrow TreeQueries<'world, 'state>,
}

impl<'borrow, 'world, 'state> Tree<'borrow, 'world, 'state> {
    fn new(list: &'borrow [NodeEntity], queries: &'borrow TreeQueries<'world, 'state>) -> Self {
        Self { list, queries }
    }

    fn parent_unfiltered(&self, entity: Entity) -> Option<Entity> {
        self.queries
            .parent_query
            .get(entity)
            .ok()
            .map(|parent| parent.0)
    }

    fn children(&self, entity: Entity) -> &[Entity] {
        self.queries
            .children_query
            .get(entity)
            .map(|x| &**x)
            .unwrap_or(&[])
    }
}

fn push_all_children(
    root: Entity,
    queries: &TreeQueries,
    vec: &mut Vec<NodeEntity>,
    // Set to true if you detect a change
    cf: &mut bool,
    cd_query: &Query<CdQuery>,
) {
    let children = queries
        .children_query
        .get(root)
        .map(|x| &**x)
        .unwrap_or(&[]);

    for &child in children {
        if !queries.control_node_query.get(child).is_ok() {
            *cf = *cf || check_cd(child, cd_query);
            vec.push(NodeEntity(child));
        }
        push_all_children(child, queries, vec, cf, cd_query);
    }
}

impl<'borrow, 'world, 'state> Hierarchy<'borrow> for Tree<'borrow, 'world, 'state> {
    type Item = NodeEntity;
    type DownIter = std::iter::Copied<std::slice::Iter<'borrow, NodeEntity>>;
    type UpIter = std::iter::Rev<std::iter::Copied<std::slice::Iter<'borrow, NodeEntity>>>;
    type ChildIter = ChildIterator<'borrow, 'world, 'state>;

    fn up_iter(&self) -> Self::UpIter {
        self.list.iter().copied().rev()
    }

    fn down_iter(&self) -> Self::DownIter {
        self.list.iter().copied()
    }

    fn parent(&self, node: Self::Item) -> Option<Self::Item> {
        let mut next_candidate = self.parent_unfiltered(node.entity());
        while let Some(candidate) = next_candidate {
            if self.queries.control_node_query.get(candidate).is_ok() {
                next_candidate = self.parent_unfiltered(candidate);
            } else {
                return Some(NodeEntity(candidate));
            }
        }
        None
    }

    fn child_iter(&'borrow self, node: Self::Item) -> Self::ChildIter {
        ChildIterator {
            inners: vec![self
                .queries
                .children_query
                .get(node.entity())
                .map(|x| &**x)
                .unwrap_or(&[])
                .iter()],
            control_node_query: &self.queries.control_node_query,
            children_query: &self.queries.children_query,
        }
    }

    fn is_first_child(&self, node: Self::Item) -> bool {
        let mut node = node.entity();
        let mut parent = if let Some(p) = self.parent_unfiltered(node) {
            p
        } else {
            return false;
        };
        loop {
            if self.queries.control_node_query.get(parent).is_ok() {
                // Root node never a control node, so unwrap never fails
                let grandparent = self.parent_unfiltered(parent).unwrap();
                let gp_children = self.children(grandparent);
                if gp_children.first() == Some(&parent) {
                    // check if grandparent is also a control node and if so, whether it's the first child
                    node = parent;
                    parent = grandparent;
                } else {
                    return false;
                }
            } else {
                // parent isn't a control node, check if we're the first node.
                let siblings = self.children(parent);
                return siblings.first() == Some(&node);
            }
        }
    }

    fn is_last_child(&self, node: Self::Item) -> bool {
        let mut node = node.entity();
        let mut parent = if let Some(p) = self.parent_unfiltered(node) {
            p
        } else {
            return false;
        };
        loop {
            if self.queries.control_node_query.get(parent).is_ok() {
                // Root node never a control node, so unwrap never fails
                let grandparent = self.parent_unfiltered(parent).unwrap();
                let gp_children = self.children(grandparent);
                if gp_children.last() == Some(&parent) {
                    // check if grandparent is also a control node and if so, whether it's the last child
                    node = parent;
                    parent = grandparent;
                } else {
                    return false;
                }
            } else {
                // parent isn't a control node, check if we're the last node.
                let siblings = self.children(parent);
                return siblings.last() == Some(&node);
            }
        }
    }
}

pub struct ChildIterator<'borrow, 'world, 'state> {
    // TODO: make this a smallvec
    inners: Vec<std::slice::Iter<'borrow, Entity>>,
    children_query: &'borrow Query<'world, 'state, &'static Children>,
    control_node_query: &'borrow Query<'world, 'state, &'static Control>,
}

impl<'borrow, 'world, 'state> Iterator for ChildIterator<'borrow, 'world, 'state> {
    type Item = NodeEntity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let candidate = loop {
                if let Some(last) = self.inners.last_mut() {
                    if let Some(candidate) = last.next() {
                        break *candidate;
                    } else {
                        self.inners.pop();
                    }
                } else {
                    return None;
                }
            };

            if self.control_node_query.get(candidate).is_ok() {
                self.inners.push(
                    self.children_query
                        .get(candidate)
                        .map(|x| &**x)
                        .unwrap_or(&[])
                        .iter(),
                );
            } else {
                return Some(NodeEntity(candidate));
            }
        }
    }
}

struct DataCache<'borrow, 'world, 'state> {
    query: &'borrow mut Query<'world, 'state, &'static mut UiNode>,
    cache: &'borrow mut LayoutScratchpad,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Space {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Default)]
pub struct LayoutScratchpad {
    space: HashMap<NodeEntity, Space>,
    size: HashMap<NodeEntity, Size>,

    child_width_max: HashMap<NodeEntity, f32>,
    child_height_max: HashMap<NodeEntity, f32>,
    child_width_sum: HashMap<NodeEntity, f32>,
    child_height_sum: HashMap<NodeEntity, f32>,

    grid_row_max: HashMap<NodeEntity, f32>,
    grid_col_max: HashMap<NodeEntity, f32>,

    horizontal_free_space: HashMap<NodeEntity, f32>,
    horizontal_stretch_sum: HashMap<NodeEntity, f32>,

    vertical_free_space: HashMap<NodeEntity, f32>,
    vertical_stretch_sum: HashMap<NodeEntity, f32>,

    stack_first_child: HashMap<NodeEntity, bool>,
    stack_last_child: HashMap<NodeEntity, bool>,
}

impl LayoutScratchpad {
    fn clear(&mut self) {
        self.space.clear();
        self.size.clear();
        self.child_width_max.clear();
        self.child_height_max.clear();
        self.child_width_sum.clear();
        self.child_height_sum.clear();
        self.grid_row_max.clear();
        self.grid_col_max.clear();
        self.horizontal_free_space.clear();
        self.horizontal_stretch_sum.clear();
        self.vertical_free_space.clear();
        self.vertical_stretch_sum.clear();
        self.stack_first_child.clear();
        self.stack_last_child.clear();
    }
}

impl<'borrow, 'world, 'state> Cache for DataCache<'borrow, 'world, 'state> {
    type Item = NodeEntity;

    fn visible(&self, _: Self::Item) -> bool {
        true
    }

    fn set_visible(&mut self, _: Self::Item, _: bool) {
        // TODO
    }

    fn geometry_changed(&self, _: Self::Item) -> morphorm::GeometryChanged {
        morphorm::GeometryChanged::empty()
    }

    fn set_geo_changed(&mut self, _: Self::Item, _: morphorm::GeometryChanged, _: bool) {
        // TODO
    }

    fn new_width(&self, node: Self::Item) -> f32 {
        self.cache
            .size
            .get(&node)
            .map(|x| x.width)
            .unwrap_or_default()
    }

    fn new_height(&self, node: Self::Item) -> f32 {
        self.cache
            .size
            .get(&node)
            .map(|x| x.height)
            .unwrap_or_default()
    }

    fn set_new_width(&mut self, node: Self::Item, value: f32) {
        let size = self.cache.size.entry(node).or_default();
        size.width = value;
    }

    fn set_new_height(&mut self, node: Self::Item, value: f32) {
        let size = self.cache.size.entry(node).or_default();
        size.height = value;
    }

    fn width(&self, node: Self::Item) -> f32 {
        self.query
            .get_component::<UiNode>(node.entity())
            .unwrap()
            .size
            .x
    }

    fn height(&self, node: Self::Item) -> f32 {
        self.query
            .get_component::<UiNode>(node.entity())
            .unwrap()
            .size
            .y
    }

    fn posx(&self, node: Self::Item) -> f32 {
        self.query
            .get_component::<UiNode>(node.entity())
            .unwrap()
            .pos
            .x
    }

    fn posy(&self, node: Self::Item) -> f32 {
        self.query
            .get_component::<UiNode>(node.entity())
            .unwrap()
            .pos
            .y
    }

    fn left(&self, node: Self::Item) -> f32 {
        self.cache
            .space
            .get(&node)
            .map(|x| x.left)
            .unwrap_or_default()
    }

    fn right(&self, node: Self::Item) -> f32 {
        self.cache
            .space
            .get(&node)
            .map(|x| x.right)
            .unwrap_or_default()
    }

    fn top(&self, node: Self::Item) -> f32 {
        self.cache
            .space
            .get(&node)
            .map(|x| x.top)
            .unwrap_or_default()
    }

    fn bottom(&self, node: Self::Item) -> f32 {
        self.cache
            .space
            .get(&node)
            .map(|x| x.bottom)
            .unwrap_or_default()
    }

    get_func_all![
        child_width_max,
        child_width_sum,
        child_height_max,
        child_height_sum,
        grid_row_max,
        grid_col_max,
        horizontal_free_space,
        horizontal_stretch_sum,
        vertical_free_space,
        vertical_stretch_sum,
    ];

    set_func_all![
        child_width_max,
        child_height_max,
        child_width_sum,
        child_height_sum,
        horizontal_free_space,
        horizontal_stretch_sum,
        vertical_free_space,
        vertical_stretch_sum,
    ];

    fn set_width(&mut self, node: Self::Item, value: f32) {
        self.query
            .get_component_mut::<UiNode>(node.entity())
            .unwrap()
            .size
            .x = value;
    }
    fn set_height(&mut self, node: Self::Item, value: f32) {
        self.query
            .get_component_mut::<UiNode>(node.entity())
            .unwrap()
            .size
            .y = value;
    }
    fn set_posx(&mut self, node: Self::Item, value: f32) {
        self.query
            .get_component_mut::<UiNode>(node.entity())
            .unwrap()
            .pos
            .x = value;
    }
    fn set_posy(&mut self, node: Self::Item, value: f32) {
        self.query
            .get_component_mut::<UiNode>(node.entity())
            .unwrap()
            .pos
            .y = value;
    }

    fn set_left(&mut self, node: Self::Item, value: f32) {
        self.cache.space.entry(node).or_default().left = value;
    }
    fn set_right(&mut self, node: Self::Item, value: f32) {
        self.cache.space.entry(node).or_default().right = value;
    }
    fn set_top(&mut self, node: Self::Item, value: f32) {
        self.cache.space.entry(node).or_default().top = value;
    }
    fn set_bottom(&mut self, node: Self::Item, value: f32) {
        self.cache.space.entry(node).or_default().bottom = value;
    }

    fn stack_first_child(&self, node: Self::Item) -> bool {
        self.cache
            .stack_first_child
            .get(&node)
            .copied()
            .unwrap_or_default()
    }

    fn set_stack_first_child(&mut self, node: Self::Item, value: bool) {
        *self.cache.stack_first_child.entry(node).or_default() = value;
    }

    fn stack_last_child(&self, node: Self::Item) -> bool {
        self.cache
            .stack_last_child
            .get(&node)
            .copied()
            .unwrap_or_default()
    }

    fn set_stack_last_child(&mut self, node: Self::Item, value: bool) {
        *self.cache.stack_last_child.entry(node).or_default() = value;
    }
}

pub(crate) fn root_node_system(
    windows: Res<bevy::window::Windows>,
    mut root_query: Query<
        (
            &mut layout_components::Width,
            &mut layout_components::Height,
            &mut UiNode,
        ),
        Without<Parent>,
    >,
) {
    let window = windows.get_primary().unwrap();

    let window_width = window.physical_width() as f32;
    let window_height = window.physical_height() as f32;

    for (mut width, mut height, mut node) in root_query.iter_mut() {
        **width = Units::Pixels(window_width);
        **height = Units::Pixels(window_height);

        node.pos = Vec2::ZERO;
        node.size = Vec2::new(window_width, window_height);
    }
}

pub(crate) fn layout_node_system(
    mut list: Local<Vec<NodeEntity>>,
    mut layout_cache: Local<LayoutScratchpad>,
    queries: TreeQueries,
    style_query: Query<StyleQuery>,
    cd_query: Query<CdQuery>,
    mut cache_query: Query<&'static mut UiNode>,
    root_node_query: Query<Entity, (With<UiNode>, Without<Parent>)>,
    removed: RemovedComponents<UiNode>,
) {
    for root_node in root_node_query.iter() {
        list.clear();
        layout_cache.clear();
        let mut any_changes = removed.iter().next().is_some() || check_cd(root_node, &cd_query);
        list.push(NodeEntity(root_node));
        push_all_children(root_node, &queries, &mut list, &mut any_changes, &cd_query);

        let tree = Tree::new(&list, &queries);
        let mut cache = DataCache {
            cache: &mut *layout_cache,
            query: &mut cache_query,
        };

        morphorm::layout(&mut cache, &tree, &style_query);
    }
}
