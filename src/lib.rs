#![feature(generic_associated_types)]
#![feature(unboxed_closures)]

use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::panic::Location;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bevy::ecs::world::EntityMut;
use bevy::ecs::{component::Component, prelude::*};
use bevy::prelude::{BuildWorldChildren, DespawnRecursiveExt, Plugin};
use bevy::ui::Interaction;
use bevy::utils::{HashMap, HashSet};
use crossbeam_channel::Sender;

#[derive(Default)]
struct UiScratchSpace {
    update_hashset: HashSet<UpdateFunc>,
}

struct ResUpdateFuncs<T>(Vec<UpdateFunc>, PhantomData<T>);
struct ComponentUpdateFuncs<T>(HashMap<Entity, Vec<UpdateFunc>>, PhantomData<T>);

struct UiManagedSystems(SystemStage);

pub struct Ui4Plugin;
impl Plugin for Ui4Plugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<UiScratchSpace>()
            .init_resource::<ButtonSystemState>()
            .insert_resource(UiManagedSystems(SystemStage::parallel()))
            .add_system(primary_ui_system.exclusive_system().at_end());
    }
}

#[derive(Clone)]
pub struct ButtonFunc(Arc<dyn Fn(&mut World) + Send + Sync>);
impl ButtonFunc {
    pub fn new(f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
    fn run(&self, world: &mut World) {
        (self.0)(world)
    }
}

#[derive(Component)]
pub struct ClickFunc(pub ButtonFunc);
#[derive(Component)]
pub struct HoverFunc(pub ButtonFunc);
#[derive(Component)]
pub struct ReleaseFunc(pub ButtonFunc);
#[derive(Component)]
pub struct UnhoverFunc(pub ButtonFunc);
/// Needed for *Func components to work
#[derive(Component, Default)]
pub struct FuncScratch(Interaction);

struct ButtonSystemState {
    query: QueryState<
        (
            Option<&'static ClickFunc>,
            Option<&'static HoverFunc>,
            Option<&'static ReleaseFunc>,
            Option<&'static UnhoverFunc>,
            &'static mut FuncScratch,
            &'static Interaction,
        ),
        (
            Changed<Interaction>,
            Or<(
                With<ClickFunc>,
                With<HoverFunc>,
                With<ReleaseFunc>,
                With<UnhoverFunc>,
            )>,
        ),
    >,
    button_list: Vec<ButtonFunc>,
}
impl FromWorld for ButtonSystemState {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
            button_list: vec![],
        }
    }
}

pub fn init_ui(world: &mut World, root: impl Fn(&mut Ctx)) {
    root(&mut Ctx {
        current_entity: world.spawn().id(),
        world,
    })
}

fn resource_change_track_system<T: Send + Sync + 'static>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ResUpdateFuncs<T>>,
    detector: Res<T>,
) {
    if detector.is_changed() {
        process_update_func_list(&mut update_funcs.0, &mut ui)
    }
}

fn component_change_track_system<T: Component>(
    mut ui: ResMut<UiScratchSpace>,
    mut update_funcs: ResMut<ComponentUpdateFuncs<T>>,
    detector: Query<Entity, Changed<T>>,
) {
    for entity in detector.iter() {
        if let Some(list) = update_funcs.0.get_mut(&entity) {
            process_update_func_list(list, &mut ui);
        }
    }
}

fn process_update_func_list(list: &mut Vec<UpdateFunc>, ui: &mut UiScratchSpace) {
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

fn primary_ui_system(world: &mut World) {
    world.resource_scope(|world, mut buttons: Mut<ButtonSystemState>| {
        let buttons = &mut *buttons;
        buttons
            .button_list
            .extend(buttons.query.iter_mut(world).filter_map(
                |(c, h, dc, dh, mut scratch, interaction)| {
                    let old = scratch.0;
                    scratch.0 = *interaction;
                    match interaction {
                        Interaction::Clicked => c.map(|x| &x.0).cloned(),
                        Interaction::Hovered => h.map(|x| &x.0).cloned(),
                        Interaction::None => match old {
                            Interaction::Clicked => dc.map(|x| &x.0).cloned(),
                            Interaction::Hovered => dh.map(|x| &x.0).cloned(),
                            Interaction::None => None,
                        },
                    }
                },
            ));

        for func in buttons.button_list.drain(..) {
            func.run(world);
        }
    });
    loop {
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            systems.0.run(world);
        });
        if world.resource_scope(|world, mut ui: Mut<UiScratchSpace>| {
            let ui = &mut *ui;
            if ui.update_hashset.is_empty() {
                return true;
            }
            for uf in ui.update_hashset.iter() {
                uf.run(world);
            }

            ui.update_hashset.clear();
            false
        }) {
            break;
        }
    }
}

pub struct Ctx<'a> {
    world: &'a mut World,
    current_entity: Entity,
}
pub struct McCtx<'a> {
    world: &'a mut World,
    get_new_child: &'a mut dyn FnMut(&mut World) -> Entity,
}
impl McCtx<'_> {
    // TODO: bikeshed name
    pub fn c(&mut self, f: impl FnOnce(&mut Ctx)) -> &mut Self {
        let new_child = (self.get_new_child)(self.world);
        f(&mut Ctx {
            current_entity: new_child,
            world: self.world,
        });
        self
    }
}

pub struct ComponentObserver<T: Send + Sync + 'static>(Entity, PhantomData<T>);

impl<T: Send + Sync + 'static> Clone for ComponentObserver<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T: Send + Sync + 'static> Copy for ComponentObserver<T> {}

impl Ctx<'_> {
    #[track_caller]
    pub fn with<T: Component, M>(&mut self, item: impl Insertable<T, M>) -> &mut Self {
        item.insert_ui_val(self);
        self
    }

    pub fn with_bundle(&mut self, bundle: impl Bundle) -> &mut Self {
        self.current_entity().insert_bundle(bundle);
        self
    }

    pub fn child(&mut self, f: impl FnOnce(&mut Ctx)) -> &mut Self {
        self.children(|ctx: &mut McCtx| {
            ctx.c(f);
        });
        self
    }

    pub fn children<M>(&mut self, children: impl Childable<M>) -> &mut Self {
        children.insert(self);
        self
    }

    pub fn component<T: Send + Sync + 'static>(&self) -> ComponentObserver<T> {
        ComponentObserver(self.current_entity, PhantomData)
    }

    pub fn this(&self) -> Entity {
        self.current_entity
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

fn get_index_from_cng_list(list: &[ChildNodeGroupKind], group_index: usize) -> usize {
    list[..group_index]
        .iter()
        .map(|node| match node {
            ChildNodeGroupKind::StaticChildren(len) => *len,
            ChildNodeGroupKind::Dynamic(entities, _) => entities.len(),
            ChildNodeGroupKind::List(entities, _) => entities.iter().map(|v| v.0.len()).sum(),
        })
        .sum()
}

struct ChildNodeUpdateFuncMarker;

enum ChildNodeGroupKind {
    StaticChildren(usize),
    Dynamic(Vec<Entity>, UfMarker<ChildNodeUpdateFuncMarker>),
    List(
        Vec<(Vec<Entity>, Entity)>,
        UfMarker<ChildNodeUpdateFuncMarker>,
    ),
}

#[derive(Default, Component)]
struct ManagedChildrenTracker {
    children: Vec<ChildNodeGroupKind>,
}

pub trait UninitObserver: Send + Sync + 'static {
    type Observer: Observer;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc;
}

pub trait Observer: Send + Sync + 'static {
    type Return<'w, 's>;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool);
}

#[derive(Clone)]
pub struct Map<O, F>(O, F);
impl<O, F> Observer for Map<O, F>
where
    O: Observer,
    F: for<'w, 's> Fn<(O::Return<'w, 's>,)> + Send + Sync + 'static,
{
    type Return<'w, 's> = <F as FnOnce<(O::Return<'w, 's>,)>>::Output;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (val, change) = self.0.get(world);
        ((self.1)(val), change)
    }
}
impl<O, MF> UninitObserver for Map<O, MF>
where
    O: UninitObserver,
    MF: for<'w, 's> Fn<(<<O as UninitObserver>::Observer as Observer>::Return<'w, 's>,)>,
    MF: Send + Sync + 'static,
{
    type Observer = Map<O::Observer, MF>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, move |obs, world| uf(Map(obs, self.1), world))
    }
}

#[derive(Clone)]
pub struct And<O1, O2>(O1, O2);
impl<O1: Observer, O2: Observer> Observer for And<O1, O2> {
    type Return<'w, 's> = (O1::Return<'w, 's>, O2::Return<'w, 's>);

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (v0, c0) = self.0.get(world);
        let (v1, c1) = self.1.get(world);
        ((v0, v1), c0 || c1)
    }
}

impl<O1: UninitObserver, O2: UninitObserver> UninitObserver for And<O1, O2> {
    type Observer = And<O1::Observer, O2::Observer>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0.register_self(world, |obs1, world| {
            self.1
                .register_self(world, move |obs2, world| (uf)(And(obs1, obs2), world))
        })
    }
}

#[derive(Clone)]
pub struct DedupTemplate<O>(O);

pub struct Dedup<O: Observer>(Option<O::Return<'static, 'static>>, O);

#[rustfmt::skip]
impl<UO, O, T> UninitObserver for DedupTemplate<UO>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Observer = Dedup<O>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        self.0
            .register_self(world, |obs, world| (uf)(Dedup(None, obs), world))
    }
}

#[rustfmt::skip]
impl<O, T> Observer for Dedup<O>
where
    O: for<'w, 's> Observer<Return<'w, 's> = T>,
    T: PartialEq + Send + Sync + 'static,
{
    type Return<'w, 's> = &'s T;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        let (maybe_new, changed) = self.1.get(world);
        if self.0.is_none() {
            self.0 = Some(maybe_new);
            return (self.0.as_ref().unwrap(), true);
        }
        if !changed || self.0.as_ref() == Some(&maybe_new) {
            (self.0.as_ref().unwrap(), false)
        } else {
            self.0 = Some(maybe_new);
            (self.0.as_ref().unwrap(), true)
        }
    }
}

pub struct ResObserverTemplate<R>(PhantomData<R>);

impl<R> Clone for ResObserverTemplate<R> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
pub struct ResObserver<R>(PhantomData<R>);

impl<R: Send + Sync + 'static> Observer for ResObserver<R> {
    type Return<'w, 's> = &'w R;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        // TODO: keep track of ticks in the observer & use it
        (world.get_resource::<R>().unwrap(), true)
    }
}
impl<R: Send + Sync + 'static> UninitObserver for ResObserverTemplate<R> {
    type Observer = ResObserver<R>;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = uf(ResObserver(PhantomData), world);
        let ufc = uf.clone();
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut list) = world.get_resource_mut::<ResUpdateFuncs<R>>() {
                list.0.push(uf);
            } else {
                systems.0.add_system(resource_change_track_system::<R>);
                world.insert_resource(ResUpdateFuncs::<R>(vec![uf], PhantomData));
            };
        });
        ufc
    }
}

pub trait ObserverExt: UninitObserver + Sized {
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        F: for<'w, 's> Fn<(<<Self as UninitObserver>::Observer as Observer>::Return<'w, 's>,)>,
        F: Send + Sync + 'static;
    fn and<O>(self, o: O) -> And<Self, O>
    where
        O: UninitObserver;
    fn dedup(self) -> DedupTemplate<Self>;
}

impl<T: UninitObserver> ObserverExt for T {
    fn map<F>(self, f: F) -> Map<Self, F> {
        Map(self, f)
    }

    fn and<O>(self, o: O) -> And<Self, O> {
        And(self, o)
    }

    fn dedup(self) -> DedupTemplate<Self> {
        DedupTemplate(self)
    }
}

pub fn res<R: Send + Sync + 'static>() -> ResObserverTemplate<R> {
    ResObserverTemplate(PhantomData)
}

#[derive(Clone, Debug)]
pub struct UpdateFunc(Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>);
struct UfInner<F: ?Sized> {
    flag: AtomicBool,
    created_at: &'static Location<'static>,
    func: Mutex<F>,
}

impl<F: ?Sized> Debug for UfInner<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UfInner")
            .field("created_at", &self.created_at)
            .finish()
    }
}

#[derive(Component)]
struct UfMarker<T>(
    Arc<UfInner<dyn FnMut(&mut World) + Send + Sync>>,
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
    #[track_caller]
    fn new<T, F: FnMut(&mut World) + Send + Sync + 'static>(func: F) -> (Self, UfMarker<T>) {
        let arc = Arc::new(UfInner {
            flag: AtomicBool::new(false),
            created_at: std::panic::Location::caller(),
            func: Mutex::new(func),
        });
        (Self(arc.clone()), UfMarker(arc, PhantomData))
    }
    fn run(&self, world: &mut World) {
        if !self.flagged() {
            (self.0.func.lock().unwrap())(world);
        }
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

pub trait Insertable<T, M>: Send + Sync + 'static {
    #[track_caller]
    fn insert_ui_val(self, ctx: &mut Ctx);
}

pub struct Static;
pub struct Dynamic;

#[derive(Clone)]
pub struct StaticObserver<T>(T);

impl<T: Send + Sync + 'static> Observer for StaticObserver<T> {
    type Return<'w, 's> = &'s T;

    fn get<'w, 's>(&'s mut self, _: &'w bevy::prelude::World) -> (Self::Return<'w, 's>, bool) {
        (&self.0, false)
    }
}

impl<T: Send + Sync + 'static> UninitObserver for StaticObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        uf(self, world)
    }
}

impl<T: Component> Insertable<T, Static> for T {
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        ctx.current_entity().insert(self);
    }
}

#[rustfmt::skip]
impl<T: Component, O, UO> Insertable<T, Dynamic> for UO
where
    for<'w, 's> O: Observer<Return<'w, 's> = T>,
    UO: UninitObserver<Observer = O>,
{
    #[track_caller]
    fn insert_ui_val(self, ctx: &mut Ctx<'_>) {
        let entity = ctx.current_entity;
        let uf = self.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<T, _>(move |world| {
                let (val, changed) = observer.get(world);
                if !changed {
                    return;
                }
                world.entity_mut(entity).insert(val);
            });
            world.entity_mut(entity).insert(marker);
            uf
        });
        uf.run(&mut ctx.world);
    }
}

pub trait Childable<M> {
    fn insert(self, ctx: &mut Ctx);
}

impl<Func> Childable<Static> for Func
where
    Func: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let mut new_child = |world: &mut World| {
            let nc = world.spawn().id();
            let mut parent = world.entity_mut(parent);
            parent.push_children(&[nc]);
            let list = get_marker_list(parent);
            match list.last_mut() {
                Some(ChildNodeGroupKind::StaticChildren(count)) => *count += 1,
                _ => list.push(ChildNodeGroupKind::StaticChildren(1)),
            }
            nc
        };
        (self)(&mut McCtx {
            world: &mut ctx.world,
            get_new_child: &mut new_child,
        });
    }
}

#[rustfmt::skip]
impl<UO, O, F> Childable<Dynamic> for UO
where
    for<'w, 's> O: Observer<Return<'w, 's> = F>,
    UO: UninitObserver<Observer = O>,
    F: FnOnce(&mut McCtx),
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let list = ctx.get_child_tracker_list();
        let group_index = list.len();

        let uf = self.register_self(ctx.world, |mut observer, world| {
            let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
                let (func, changed) = observer.get(world);
                if !changed {
                    return;
                }
                let list = get_marker_list(world.entity_mut(parent));
                let index = get_index_from_cng_list(list, group_index);
                let entities = match &mut list[group_index] {
                    ChildNodeGroupKind::Dynamic(entities, _) => entities,
                    _ => unreachable!(),
                };
                // TODO: find a way to somehow do double buffering or sth with these vecs
                let mut old_entities = std::mem::replace(entities, Vec::new());
                for &entity in old_entities.iter() {
                    world.entity_mut(entity).despawn_recursive();
                }
                old_entities.clear();
                let list = get_marker_list(world.entity_mut(parent));
                match &mut list[group_index] {
                    ChildNodeGroupKind::Dynamic(l, _) => *l = old_entities,
                    _ => unreachable!(),
                };
                let mut new_child_func = |world: &mut World| {
                    let nc = world.spawn().id();
                    let list = get_marker_list(world.entity_mut(parent));
                    let entities = match &mut list[group_index] {
                        ChildNodeGroupKind::Dynamic(entities, _) => entities,
                        _ => unreachable!(),
                    };
                    let len = entities.len();
                    entities.push(nc);
                    world.entity_mut(parent).insert_children(index + len, &[nc]);
                    nc
                };
                func(&mut McCtx {
                    world,
                    get_new_child: &mut new_child_func,
                });
            });

            get_marker_list(world.entity_mut(parent))
                .push(ChildNodeGroupKind::Dynamic(vec![], marker));
            uf
        });
        uf.run(&mut ctx.world);
    }
}

#[rustfmt::skip]
pub trait IntoObserver<T, M>: Send + Sync + 'static
{
    type UninitObserver: UninitObserver<Observer = Self::Observer>;
    type Observer: for<'w, 's> Observer<Return<'w, 's> = Self::ObserverReturn<'w, 's>>;
    type ObserverReturn<'w, 's>: Borrow<T>;
    fn into_observable(self) -> Self::UninitObserver;
}

impl<T: Send + Sync + 'static> IntoObserver<T, Static> for T {
    type UninitObserver = StaticObserver<T>;
    type Observer = StaticObserver<T>;
    type ObserverReturn<'w, 's> = &'s T;

    fn into_observable(self) -> Self::Observer {
        StaticObserver(self)
    }
}

#[rustfmt::skip]
impl<T, O: for<'w, 's> Observer<Return<'w, 's> = T>, UO: UninitObserver<Observer = O>> IntoObserver<T, Dynamic> for UO {
    type UninitObserver = Self;
    type Observer = O;
    type ObserverReturn<'w, 's> = O::Return<'w, 's>;

    fn into_observable(self) -> Self::UninitObserver {
        self
    }

}

impl<T: Component> Observer for ComponentObserver<T> {
    type Return<'w, 's> = &'w T;

    fn get<'w, 's>(&'s mut self, world: &'w World) -> (Self::Return<'w, 's>, bool) {
        // TODO: use change detection
        (world.get::<T>(self.0).unwrap(), true)
    }
}

impl<T: Component> UninitObserver for ComponentObserver<T> {
    type Observer = Self;

    fn register_self<F: FnOnce(Self::Observer, &mut World) -> UpdateFunc>(
        self,
        world: &mut World,
        uf: F,
    ) -> UpdateFunc {
        let uf = (uf)(self, world);
        let ufc = uf.clone();
        world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
            if let Some(mut lists) = world.get_resource_mut::<ComponentUpdateFuncs<T>>() {
                lists.0.entry(self.0).or_default().push(uf);
            } else {
                systems.0.add_system(component_change_track_system::<T>);
                world.insert_resource(ComponentUpdateFuncs::<T>(
                    [(self.0, vec![uf])].into_iter().collect(),
                    PhantomData,
                ));
            };
        });
        ufc
    }
}

trait TvecObserverExt: Sized {
    fn for_each<F, Ff>(self, f: F) -> TvecForeachTemplate<Self, F>;
}

pub struct TrackedVec<T> {
    inner: Vec<T>,
    update_out: Mutex<Vec<Sender<Diff<T>>>>,
}

enum Diff<T> {
    Init(Vec<T>),
    Push(T),
    Pop,
    Replace(T, usize),
    // To be supported when Children supports it
    // Switch(usize, usize),
    RemoveAt(usize),
    InsertAt(T, usize),
    Clear,
}

impl<T> TrackedVec<T> {}

struct TVecMarker;

#[rustfmt::skip]
impl<O, UO, T: 'static> TvecObserverExt for UO
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w TrackedVec<T>>,
{
    fn for_each<F, Ff>(self, f: F) -> TvecForeachTemplate<Self, F> {
        TvecForeachTemplate(self, f)
    }
}

struct TvecForeachTemplate<UO, F>(UO, F);

struct TvecItemObserverTemplate<T: Send + Sync + 'static>(
    ComponentObserver<TvecItemComponentWrapper<T>>,
);

impl<T: Send + Sync + 'static> Clone for TvecItemObserverTemplate<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Component)]
struct TvecItemComponentWrapper<T>(T);

#[rustfmt::skip]
impl<O, UO, T, F, Ff> Childable<TVecMarker> for TvecForeachTemplate<UO, F>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w TrackedVec<T>>,
    F: Fn(TvecItemObserverTemplate<T>) -> Ff + Send + Sync + 'static,
    Ff: FnOnce(&mut McCtx),
    T: Clone + Send + Sync + 'static,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let f = self.1;

        let list = ctx.get_child_tracker_list();
        let group_index = list.len();

        self.0.register_self(ctx.world, |mut obs, world| {
            let (tv, _) = obs.get(world);
            let (tx, rx) = crossbeam_channel::unbounded();

            tx.send(Diff::Init(tv.inner.clone())).unwrap();
            tv.update_out.lock().unwrap().push(tx);

            let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
                let insert = |world: &mut World, e, i: Option<usize>| {
                    let list_item_entity = world.spawn().insert(TvecItemComponentWrapper(e)).id();
                    let list = get_marker_list(world.entity_mut(parent));
                    // the +1 makes it also include this node in the calculation
                    let mut insert_index = get_index_from_cng_list(list, group_index + 1);
                    let entities = match &mut list[group_index] {
                        ChildNodeGroupKind::List(v, _) => v,
                        _ => unreachable!(),
                    };
                    let i = i.unwrap_or(entities.len());
                    entities.insert(i, (vec![], list_item_entity));
                    let observer = TvecItemObserverTemplate::<T>(ComponentObserver(
                        list_item_entity,
                        PhantomData,
                    ));
                    let mut get_new_child = |world: &mut World| {
                        let id = world.spawn().id();
                        let mut parent = world.entity_mut(parent);
                        parent.insert_children(insert_index, &[id]);
                        let list = get_marker_list(parent);
                        let entities = match &mut list[group_index] {
                            ChildNodeGroupKind::List(v, _) => v,
                            _ => unreachable!(),
                        };
                        entities[i].0.push(id);
                        insert_index += 1;
                        id
                    };
                    (f(observer))(&mut McCtx {
                        world,
                        get_new_child: &mut get_new_child,
                    });
                };
                let remove = |world: &mut World, i: Option<usize>| {
                    let list = get_marker_list(world.entity_mut(parent));
                    let entities = match &mut list[group_index] {
                        ChildNodeGroupKind::List(v, _) => v,
                        _ => unreachable!(),
                    };
                    let i = i.unwrap_or(entities.len() - 1);
                    let (list, tvec_item) = entities.remove(i);
                    for entity in list {
                        world.entity_mut(entity).despawn_recursive();
                    }
                    world.entity_mut(tvec_item).despawn();
                };
                for msg in rx.try_iter() {
                    match msg {
                        Diff::Init(list) => {
                            let l = get_marker_list(world.entity_mut(parent));
                            let mut insert_index = get_index_from_cng_list(l, group_index);
                            for e in list {
                                let list_item_entity =
                                    world.spawn().insert(TvecItemComponentWrapper(e)).id();
                                let list = get_marker_list(world.entity_mut(parent));
                                let entities = match &mut list[group_index] {
                                    ChildNodeGroupKind::List(v, _) => v,
                                    _ => unreachable!(),
                                };
                                entities.push((vec![], list_item_entity));
                                let observer = TvecItemObserverTemplate::<T>(ComponentObserver(
                                    list_item_entity,
                                    PhantomData,
                                ));

                                let mut get_new_child = |world: &mut World| {
                                    let id = world.spawn().id();
                                    let mut parent = world.entity_mut(parent);
                                    parent.insert_children(insert_index, &[id]);
                                    let list = get_marker_list(parent);
                                    let entities = match &mut list[group_index] {
                                        ChildNodeGroupKind::List(v, _) => v,
                                        _ => unreachable!(),
                                    };
                                    entities.last_mut().unwrap().0.push(id);
                                    insert_index += 1;
                                    id
                                };
                                (f(observer))(&mut McCtx {
                                    world,
                                    get_new_child: &mut get_new_child,
                                });
                            }
                        }
                        Diff::Push(e) => insert(world, e, None),
                        Diff::Pop => remove(world, None),
                        Diff::Replace(e, i) => {
                            let list = get_marker_list(world.entity_mut(parent));
                            let entities = match &mut list[group_index] {
                                ChildNodeGroupKind::List(v, _) => v,
                                _ => unreachable!(),
                            };
                            let item_entity = entities[i].1;
                            // TODO: maybe push a change or sth?
                            world
                                .entity_mut(item_entity)
                                .get_mut::<TvecItemComponentWrapper<T>>()
                                .unwrap()
                                .0 = e;
                        }
                        // Diff::Switch(_, _) => todo!(),
                        Diff::RemoveAt(i) => remove(world, Some(i)),
                        Diff::InsertAt(e, i) => insert(world, e, Some(i)),
                        Diff::Clear => {
                            let list = get_marker_list(world.entity_mut(parent));
                            let entities = match &mut list[group_index] {
                                ChildNodeGroupKind::List(v, _) => v,
                                _ => unreachable!(),
                            };
                            let mut entities = std::mem::replace(entities, Vec::new());
                            for (list, tvec_item) in entities.drain(..) {
                                for entity in list {
                                    world.entity_mut(entity).despawn_recursive();
                                }
                                world.entity_mut(tvec_item).despawn();
                            }
                            let list = get_marker_list(world.entity_mut(parent));
                            match &mut list[group_index] {
                                ChildNodeGroupKind::List(v, _) => *v = entities,
                                _ => unreachable!(),
                            };
                        }
                    }
                }
            });
            get_marker_list(world.entity_mut(parent))
                .push(ChildNodeGroupKind::List(vec![], marker));
            uf.run(world);
            uf
        });
    }
}
