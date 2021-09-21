use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bevy::ecs::system::CommandQueue;
use bevy::ecs::world::EntityMut;
use bevy::ecs::{component::Component, prelude::*};
use bevy::prelude::{BuildWorldChildren, Children};

pub struct Ui {
    update_hashset: lockfree::set::Set<UpdateFunc>,
    command_queue: CommandQueue,
}

struct ResUpdateFuncs<T>(Vec<UpdateFunc>, PhantomData<T>);
struct UiManagedSystems(SystemStage);

fn change_track_system<T: Send + Sync + 'static>(
    ui: Res<Ui>,
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
                ui.update_hashset.insert(relevant_uf.clone()).ok();
                i += 1;
            }
        }
    }
}

pub fn primary_ui_system(world: &mut World) {
    world.resource_scope(|world, mut systems: Mut<UiManagedSystems>| {
        systems.0.run(world);
        world.resource_scope(|world, mut ui: Mut<Ui>| {
            let ui = &mut *ui;
            for uf in ui.update_hashset.iter() {
                uf.run(world);
            }

            ui.command_queue.apply(world);
            ui.update_hashset.clear();
        });
    });
}

pub struct Ctx<'a> {
    world: &'a mut World,
    ui: &'a mut Ui,
    systems: &'a mut UiManagedSystems,
    current_entity: Entity,
}

impl Ctx<'_> {
    pub fn insert(&mut self, item: impl Component) -> &mut Self {
        self.current_entity().insert(item);
        self
    }

    pub fn insert_dynamic<O, R>(&mut self, item: O) -> &mut Self
    where
        for<'a> O: Observable<'a, Return = R>,
        R: Component,
    {
        let entity = self.current_entity;
        let observer = Arc::new(item);
        let observer_clone = observer.clone();
        let (uf, marker) = UpdateFunc::new::<R, _>(move |world| {
            let val = observer_clone.get(world);
            world.entity_mut(entity).insert(val);
        });
        self.current_entity().insert(marker);
        observer.register_self(self.world, uf);
        self
    }

    pub fn static_child(&mut self, f: impl Fn(&mut Ctx) + Send + Sync + 'static) -> &mut Self {
        let new_entity = self.world.spawn().id();
        f(&mut Ctx {
            current_entity: new_entity,
            ui: self.ui,
            world: self.world,
            systems: self.systems,
        });
        self.current_entity().push_children(&[new_entity]); // This will need to change once dynamic children exist
        self
    }

    pub fn dyn_child<O>(&mut self, f: impl Fn(&mut Ctx) + Send + Sync + 'static, o: O)
    where
        for<'a> O: Observable<'a, Return = bool>,
    {
        let observer = Arc::new(o);
        let observer_clone = observer.clone();
        let insert_point = self
            .current_entity()
            .get::<Children>()
            .map(|c| c.len())
            .unwrap_or(0);
        let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
            let val = observer_clone.get(world);
        });
        observer.register_self(self.world, uf);
    }

    fn current_entity(&mut self) -> EntityMut<'_> {
        self.world.entity_mut(self.current_entity)
    }
}

struct ChildNodeUpdateFuncMarker;

enum ChildNodeGroupKind {
    StaticChildren(usize),
    Optional(bool, UfMarker<ChildNodeUpdateFuncMarker>),
    List(UfMarker<ChildNodeUpdateFuncMarker>),
}

struct ManagedChildrenTracker {
    children: Vec<ChildNodeGroupKind>,
}

pub trait Observable<'a>: Send + Sync + 'static {
    type Return;

    fn get(&self, world: &'a World) -> Self::Return;
    fn register_self(&self, world: &mut World, uf: UpdateFunc);
}

struct Map<O, F>(O, F);
impl<'a, O, F, R> Observable<'a> for Map<O, F>
where
    O: Observable<'a>,
    F: Fn(O::Return) -> R + Send + Sync + 'static,
{
    type Return = R;

    fn get(&self, world: &'a World) -> Self::Return {
        (self.1)(self.0.get(world))
    }

    fn register_self(&self, world: &mut World, uf: UpdateFunc) {
        self.0.register_self(world, uf);
    }
}

struct And<O1, O2>(O1, O2);
impl<'a, O1, O2> Observable<'a> for And<O1, O2>
where
    O1: Observable<'a>,
    O2: Observable<'a>,
{
    type Return = (O1::Return, O2::Return);

    fn get(&self, world: &'a World) -> Self::Return {
        (self.0.get(world), self.1.get(world))
    }

    fn register_self(&self, world: &mut World, uf: UpdateFunc) {
        self.0.register_self(world, uf.clone());
        self.1.register_self(world, uf);
    }
}

struct ResObserver<R>(PhantomData<R>);

impl<'a, R: Send + Sync + 'static> Observable<'a> for ResObserver<R> {
    type Return = &'a R;

    fn get(&self, world: &'a World) -> Self::Return {
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

fn root(ctx: &mut Ctx) {}
