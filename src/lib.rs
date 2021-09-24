#![feature(associated_type_bounds)]

use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bevy::ecs::world::EntityMut;
use bevy::ecs::{component::Component, prelude::*};
use bevy::prelude::{BuildWorldChildren, DespawnRecursiveExt, Plugin};
use bevy::utils::HashSet;

#[derive(Default)]
struct UiScratchSpace {
    update_hashset: HashSet<UpdateFunc>,
}

struct ResUpdateFuncs<T>(Vec<UpdateFunc>, PhantomData<T>);

struct UiManagedSystems(SystemStage);

pub struct Ui4Plugin;
impl Plugin for Ui4Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<UiScratchSpace>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end());
    }
}

pub fn init_ui(world: &mut World, root: impl Fn(&mut Ctx)) {
    root(&mut Ctx {
        current_entity: world.spawn().id(),
        world,
    })
}

fn change_track_system<T: Send + Sync + 'static>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ResUpdateFuncs<T>>,
    detector: Res<T>,
) {
    if detector.is_changed() {
        let list = &mut update_funcs.0;
        let mut i = 0usize;
        loop {
            if i == list.len() {
                break;
            }
            let relevant_uf = &list[i];
            if relevant_uf.flagged() {
                list.swap_remove(i);
            } else {
                ui.update_hashset.insert(relevant_uf.clone());
                i += 1;
            }
        }
    }
}

fn primary_ui_system(world: &mut World) {
    world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
        systems.0.run(world);
        world.resource_scope(|world, mut ui: Mut<UiScratchSpace>| {
            let ui = &mut *ui;
            for uf in ui.update_hashset.iter() {
                uf.run(world);
            }

            ui.update_hashset.clear();
        });
    });
}

pub struct Ctx<'a> {
    world: &'a mut World,
    current_entity: Entity,
}

impl Ctx<'_> {
    pub fn insert<T: Component, M>(&mut self, item: impl UiVal<T, M>) -> &mut Self {
        item.insert_ui_val(self);
        self
    }

    pub fn insert_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.current_entity().insert_bundle(bundle);
        self
    }

    pub fn static_child(&mut self, f: impl Fn(&mut Ctx) + Send + Sync + 'static) -> &mut Self {
        let new_entity = self.world.spawn().id();
        f(&mut Ctx {
            current_entity: new_entity,
            world: self.world,
        });
        self.current_entity().push_children(&[new_entity]);
        let list = self.get_child_tracker_list();
        match list.last_mut() {
            Some(ChildNodeGroupKind::StaticChildren(count)) => *count += 1,
            _ => list.push(ChildNodeGroupKind::StaticChildren(1)),
        }
        self
    }

    pub fn optional_child<O>(&mut self, f: impl Fn(&mut Ctx) + Send + Sync + 'static, o: O)
    where
        for<'s, 'w> O: Observable<'s, 'w, Return = bool>,
    {
        let observer = Arc::new(o);
        let observer_clone = observer.clone();
        let parent = self.current_entity;
        let list = self.get_child_tracker_list();
        let group_index = list.len();
        let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
            let new_value = observer_clone.get(world);
            let list = get_marker_list(world.entity_mut(parent));
            let index = list[..group_index]
                .iter()
                .map(|node| match node {
                    ChildNodeGroupKind::StaticChildren(len) => *len,
                    ChildNodeGroupKind::Optional(node, _) => node.is_some() as usize,
                })
                .sum();
            let mut node = match &mut list[group_index] {
                ChildNodeGroupKind::Optional(node, _) => node,
                _ => unreachable!(),
            };
            match (&mut node, new_value) {
                // spawn the node
                (None, true) => {
                    let new_entity = world.spawn().id();
                    f(&mut Ctx {
                        current_entity: new_entity,
                        world,
                    });
                    world
                        .entity_mut(parent)
                        .insert_children(index, &[new_entity]);
                    let list = get_marker_list(world.entity_mut(parent));
                    let node = match &mut list[group_index] {
                        ChildNodeGroupKind::Optional(node, _) => node,
                        _ => unreachable!(),
                    };
                    *node = Some(new_entity);
                }
                // despawn the node
                (Some(entity), false) => {
                    let entity = *entity;
                    *node = None;
                    world.entity_mut(entity).despawn_recursive();
                }
                _ => {}
            }
        });

        list.push(ChildNodeGroupKind::Optional(None, marker));

        observer.register_self(self.world, uf);
    }

    fn current_entity(&mut self) -> EntityMut<'_> {
        self.world.entity_mut(self.current_entity)
    }

    fn get_child_tracker_list(&mut self) -> &mut Vec<ChildNodeGroupKind> {
        get_marker_list(self.current_entity())
    }
}

fn get_marker_list(mut current: EntityMut) -> &mut Vec<ChildNodeGroupKind> {
    if current.get::<ManagedChildrenTracker>().is_none() {
        current.insert(ManagedChildrenTracker::default());
    }
    &mut current
        .get_mut::<ManagedChildrenTracker>()
        .unwrap()
        .into_inner()
        .children
}

struct ChildNodeUpdateFuncMarker;

enum ChildNodeGroupKind {
    StaticChildren(usize),
    Optional(Option<Entity>, UfMarker<ChildNodeUpdateFuncMarker>),
    // List(UfMarker<ChildNodeUpdateFuncMarker>),
}

#[derive(Default)]
struct ManagedChildrenTracker {
    children: Vec<ChildNodeGroupKind>,
}

pub trait Observable<'s, 'w>: Clone + Send + Sync + 'static {
    type Return;

    fn get(&'s self, world: &'w World) -> Self::Return;
    fn register_self(&self, world: &mut World, uf: UpdateFunc);
}

#[derive(Clone)]
pub struct Map<O, F>(O, F);
impl<'s, 'w, O, F, R> Observable<'s, 'w> for Map<O, F>
where
    O: Observable<'s, 'w>,
    F: Fn(O::Return) -> R + Clone + Send + Sync + 'static,
{
    type Return = R;

    fn get(&'s self, world: &'w World) -> Self::Return {
        (self.1)(self.0.get(world))
    }

    fn register_self(&self, world: &mut World, uf: UpdateFunc) {
        self.0.register_self(world, uf);
    }
}

#[derive(Clone)]
pub struct And<O1, O2>(O1, O2);
impl<'s, 'w, O1, O2> Observable<'s, 'w> for And<O1, O2>
where
    O1: Observable<'s, 'w>,
    O2: Observable<'s, 'w>,
{
    type Return = (O1::Return, O2::Return);

    fn get(&'s self, world: &'w World) -> Self::Return {
        (self.0.get(world), self.1.get(world))
    }

    fn register_self(&self, world: &mut World, uf: UpdateFunc) {
        self.0.register_self(world, uf.clone());
        self.1.register_self(world, uf);
    }
}

pub struct ResObserver<R>(PhantomData<R>);

impl<R> Clone for ResObserver<R> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<'s, 'w, R: Send + Sync + 'static> Observable<'s, 'w> for ResObserver<R> {
    type Return = &'w R;

    fn get(&'s self, world: &'w World) -> Self::Return {
        world.get_resource::<R>().unwrap()
    }

    fn register_self(&self, world: &mut World, uf: UpdateFunc) {
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut list) = world.get_resource_mut::<ResUpdateFuncs<R>>() {
                list.0.push(uf);
            } else {
                systems.0.add_system(change_track_system::<R>);
                world.insert_resource(ResUpdateFuncs::<R>(vec![uf], PhantomData));
            };
        });
    }
}

pub trait ObserverExt: for<'s, 'w> Observable<'s, 'w> + Sized {
    fn map<F, R>(self, f: F) -> Map<Self, F>
    where
        F: for<'s, 'w> Fn(<Self as Observable<'s, 'w>>::Return) -> R + Send + Sync + 'static;
    fn and<O>(self, o: O) -> And<Self, O>
    where
        O: for<'s, 'w> Observable<'s, 'w>;
}

impl<T> ObserverExt for T
where
    T: for<'s, 'w> Observable<'s, 'w> + Sized,
{
    fn map<F, R>(self, f: F) -> Map<Self, F>
    where
        F: for<'s, 'w> Fn(<T as Observable<'s, 'w>>::Return) -> R + Send + Sync + 'static,
    {
        Map(self, f)
    }

    fn and<O>(self, o: O) -> And<Self, O>
    where
        O: for<'s, 'w> Observable<'s, 'w>,
    {
        And(self, o)
    }
}

pub fn res<R: Send + Sync + 'static>() -> ResObserver<R> {
    ResObserver(PhantomData)
}

#[derive(Clone)]
pub struct UpdateFunc(Arc<UfInner<dyn Fn(&mut World) + Send + Sync>>);
struct UfInner<F: ?Sized> {
    flag: AtomicBool,
    func: F,
}
struct UfMarker<T>(
    Arc<UfInner<dyn Fn(&mut World) + Send + Sync>>,
    PhantomData<T>,
);

impl<T> Drop for UfMarker<T> {
    fn drop(&mut self) {
        self.0
            .flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl UpdateFunc {
    fn new<T, F: Fn(&mut World) + Send + Sync + 'static>(func: F) -> (Self, UfMarker<T>) {
        let arc = Arc::new(UfInner {
            flag: AtomicBool::new(false),
            func,
        });
        (Self(arc.clone()), UfMarker(arc, PhantomData))
    }
    fn run(&self, world: &mut World) {
        (self.0.func)(world);
    }

    fn flagged(&self) -> bool {
        self.0.flag.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Eq for UpdateFunc {}
impl PartialEq for UpdateFunc {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}
impl Hash for UpdateFunc {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}
impl PartialOrd for UpdateFunc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Arc::as_ptr(&self.0).partial_cmp(&Arc::as_ptr(&other.0))
    }
}
impl Ord for UpdateFunc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Arc::as_ptr(&self.0).cmp(&Arc::as_ptr(&other.0))
    }
}

pub trait UiVal<T, M>: Clone + Send + Sync + 'static {
    type Observable: for<'s, 'w> Observable<'s, 'w, Return = T>;
    fn insert_ui_val(self, ctx: &mut Ctx);
    fn as_observable(self) -> Self::Observable;
}

pub struct Static;
pub struct Dynamic;

#[derive(Clone)]
pub struct StaticObserver<T>(T);

impl<'s, 'w, T: Component + Clone> Observable<'s, 'w> for StaticObserver<T> {
    type Return = T;

    fn get(&'s self, _: &'w bevy::prelude::World) -> <Self as Observable<'s, 'w>>::Return {
        self.0.clone()
    }
    fn register_self(&self, _: &mut bevy::prelude::World, _: UpdateFunc) {}
}

impl<T: Component + Clone> UiVal<T, Static> for T {
    type Observable = StaticObserver<T>;

    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        ctx.current_entity().insert(self);
    }

    fn as_observable(self) -> Self::Observable {
        StaticObserver(self)
    }
}

impl<T: Component, O> UiVal<T, Dynamic> for O
where
    for<'s, 'w> O: Observable<'s, 'w, Return = T>,
{
    type Observable = O;

    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        let entity = ctx.current_entity;
        let observer = Arc::new(self);
        let observer_clone = observer.clone();
        let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
            let val = observer_clone.get(world);
            world.entity_mut(entity).insert(val);
        });
        ctx.current_entity().insert(marker);
        observer.register_self(ctx.world, uf);
    }

    fn as_observable(self) -> Self::Observable {
        self
    }
}
