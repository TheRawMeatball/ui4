mod map;
mod vec;

use std::marker::PhantomData;

use bevy::{
    ecs::prelude::*,
    prelude::{BuildWorldChildren, DespawnRecursiveExt},
};
use crossbeam_channel::Sender;
pub use map::TrackedMap;
pub use vec::TrackedVec;

use crate::{
    childable::{get_index_from_cng_list, ChildNodeGroupKind, ChildNodeUpdateFuncMarker},
    observer::{Observer, UninitObserver},
    prelude::{Ctx, McCtx},
    runtime::{UiScratchSpace, UpdateFunc},
};

use super::{get_marker_list, Childable};

trait TrackedObserverExt: Sized {
    fn for_each<F, Ff>(self, f: F) -> TrackedForeach<Self, F>;
}

#[derive(Clone)]
enum Diff<T> {
    Init(Vec<T>),
    Push(T),
    Pop,
    Replace(T, usize),
    // To be supported when Children supports it
    // Switch(usize, usize),
    Remove(usize),
    Insert(T, usize),
    Clear,
}

struct TrackedForeach<UO, F>(UO, F);

type TrackedAnyList<T> = Vec<(T, Vec<UpdateFunc>)>;

pub struct TrackedItemObserver<T: Send + Sync + 'static> {
    _marker: PhantomData<T>,
    group_index: usize,
    index: usize,
}

impl<T: Send + Sync + 'static> Clone for TrackedItemObserver<T> {
    fn clone(&self) -> Self {
        Self {
            _marker: PhantomData,
            group_index: self.group_index,
            index: self.index,
        }
    }
}

trait Tracked {
    type Item;
    fn register(&self, sender: Sender<Diff<Self::Item>>);
}

struct TrackedMarker;

#[rustfmt::skip]
impl<O, UO, T, F, Ff, Tt> Childable<(TrackedMarker, Tt)> for TrackedForeach<UO, F>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w Tt>,
    F: Fn(TrackedItemObserver<T>) -> Ff + Send + Sync + 'static,
    Ff: FnOnce(&mut McCtx),
    T: Send + Sync + 'static,
    Tt: Tracked<Item = T>,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let f = self.1;

        let list = get_marker_list(ctx.current_entity());
        let group_index = list.len();

        self.0.register_self(ctx.world, |mut obs, world| {
            let (tv, _) = obs.get(world);
            let (tx, rx) = crossbeam_channel::unbounded();

            tv.register(tx);

            let (uf, marker) = UpdateFunc::new::<ChildNodeUpdateFuncMarker, _>(move |world| {
                let insert = |world: &mut World, e, i: Option<usize>| {
                    let list = get_marker_list(world.entity_mut(parent));
                    // the +1 makes it also include this node in the calculation
                    let mut insert_index = get_index_from_cng_list(list, group_index + 1);
                    let (entities, items) = match &mut list[group_index] {
                        ChildNodeGroupKind::List(v, i, _) => {
                            (v, (&mut **i).downcast_mut::<TrackedAnyList<T>>().unwrap())
                        }
                        _ => unreachable!(),
                    };
                    let i = i.unwrap_or(entities.len());
                    entities.insert(i, vec![]);
                    items.insert(i, (e, vec![]));
                    let observer = TrackedItemObserver::<T> {
                        _marker: PhantomData,
                        group_index,
                        index: i,
                    };
                    let mut get_new_child = |world: &mut World| {
                        let id = world.spawn().id();
                        let mut parent = world.entity_mut(parent);
                        parent.insert_children(insert_index, &[id]);
                        let list = get_marker_list(parent);
                        let entities = match &mut list[group_index] {
                            ChildNodeGroupKind::List(v, _, _) => v,
                            _ => unreachable!(),
                        };
                        entities[i].push(id);
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
                    let (entities, items) = match &mut list[group_index] {
                        ChildNodeGroupKind::List(v, i, _) => {
                            (v, (&mut **i).downcast_mut::<TrackedAnyList<T>>().unwrap())
                        }
                        _ => unreachable!(),
                    };
                    let i = i.unwrap_or(entities.len() - 1);
                    let list = entities.remove(i);
                    items.remove(i);
                    for entity in list {
                        world.entity_mut(entity).despawn_recursive();
                    }
                };
                for msg in rx.try_iter() {
                    match msg {
                        Diff::Init(list) => {
                            let l = get_marker_list(world.entity_mut(parent));
                            let mut insert_index = get_index_from_cng_list(l, group_index);
                            for e in list {
                                let list = get_marker_list(world.entity_mut(parent));
                                let (entities, items) = match &mut list[group_index] {
                                    ChildNodeGroupKind::List(v, i, _) => {
                                        (v, (&mut **i).downcast_mut::<TrackedAnyList<T>>().unwrap())
                                    }
                                    _ => unreachable!(),
                                };
                                entities.push(vec![]);
                                items.push((e, vec![]));
                                let observer = TrackedItemObserver::<T> {
                                    _marker: PhantomData,
                                    group_index,
                                    index: entities.len() - 1,
                                };
                                let mut get_new_child = |world: &mut World| {
                                    let id = world.spawn().id();
                                    let mut parent = world.entity_mut(parent);
                                    parent.insert_children(insert_index, &[id]);
                                    let list = get_marker_list(parent);
                                    let entities = match &mut list[group_index] {
                                        ChildNodeGroupKind::List(v, _, _) => v,
                                        _ => unreachable!(),
                                    };
                                    entities.last_mut().unwrap().push(id);
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
                            let items = match &mut list[group_index] {
                                ChildNodeGroupKind::List(_, i, _) => {
                                    (&mut **i).downcast_mut::<TrackedAnyList<T>>().unwrap()
                                }
                                _ => unreachable!(),
                            };
                            let (item, ufs) = &mut items[i];
                            *item = e;
                            let ufs: Vec<_> = ufs.iter().cloned().collect();
                            world
                                .get_resource_mut::<UiScratchSpace>()
                                .unwrap()
                                .register_update_funcs(ufs);
                        }
                        // Diff::Switch(_, _) => todo!(),
                        Diff::Remove(i) => remove(world, Some(i)),
                        Diff::Insert(e, i) => insert(world, e, Some(i)),
                        Diff::Clear => {
                            let list = get_marker_list(world.entity_mut(parent));
                            let (entities, items) = match &mut list[group_index] {
                                ChildNodeGroupKind::List(v, i, _) => {
                                    (v, (&mut **i).downcast_mut::<TrackedAnyList<T>>().unwrap())
                                }
                                _ => unreachable!(),
                            };
                            items.clear();
                            let mut entities = std::mem::replace(entities, Vec::new());
                            for entity in entities.drain(..).flatten() {
                                world.entity_mut(entity).despawn_recursive();
                            }

                            let list = get_marker_list(world.entity_mut(parent));
                            match &mut list[group_index] {
                                ChildNodeGroupKind::List(v, _, _) => *v = entities,
                                _ => unreachable!(),
                            };
                        }
                    }
                }
            });
            get_marker_list(world.entity_mut(parent)).push(ChildNodeGroupKind::List(
                vec![],
                Box::new(TrackedAnyList::<T>::new()),
                marker,
            ));
            uf.run(world);
            uf
        });
    }
}
