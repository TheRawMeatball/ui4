use std::{marker::PhantomData, ops::Deref, sync::Mutex};

use bevy::prelude::{BuildWorldChildren, DespawnRecursiveExt, World};
use crossbeam_channel::Sender;

use crate::childable::{
    get_index_from_cng_list, get_marker_list, ChildNodeGroupKind, ChildNodeUpdateFuncMarker,
};
use crate::runtime::{UiScratchSpace, UpdateFunc};
use crate::{
    childable::Childable,
    ctx::{Ctx, McCtx},
    observer::{Observer, UninitObserver},
};

use super::{Diff, TrackedAnyList, TrackedForeach, TrackedMarker, TrackedObserverExt};

#[rustfmt::skip]
impl<O, UO, T: 'static> TrackedObserverExt for UO
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w TrackedVec<T>>,
{
    fn for_each<F, Ff>(self, f: F) -> TrackedForeach<Self, F> {
        TrackedForeach(self, f)
    }
}

pub struct TrackedVec<T> {
    inner: Vec<T>,
    update_out: Mutex<Vec<Sender<Diff<T>>>>,
}

impl<T> Default for TrackedVec<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            update_out: Default::default(),
        }
    }
}

impl<T: Clone> TrackedVec<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn send_msg(&mut self, msg: Diff<T>) -> Option<T> {
        self.update_out
            .get_mut()
            .unwrap()
            .retain(|tx| tx.send(msg.clone()).is_ok());
        match msg {
            Diff::Push(val) => self.inner.push(val),
            Diff::Pop => return self.inner.pop(),
            Diff::Replace(val, i) => return Some(std::mem::replace(&mut self.inner[i], val)),
            Diff::Remove(i) => return Some(self.inner.remove(i)),
            Diff::Insert(val, i) => self.inner.insert(i, val),
            Diff::Clear => self.inner.clear(),
            Diff::Init(_) => unreachable!(),
        }
        None
    }

    pub fn push(&mut self, val: T) {
        let msg = Diff::Push(val);
        self.send_msg(msg);
    }

    pub fn pop(&mut self) -> Option<T> {
        let msg = Diff::Pop;
        self.send_msg(msg)
    }

    pub fn replace(&mut self, val: T, i: usize) -> T {
        let msg = Diff::Replace(val, i);
        self.send_msg(msg).unwrap()
    }

    pub fn remove(&mut self, i: usize) -> T {
        let msg = Diff::Remove(i);
        self.send_msg(msg).unwrap()
    }

    pub fn insert(&mut self, val: T, i: usize) {
        let msg = Diff::Insert(val, i);
        self.send_msg(msg);
    }

    pub fn clear(&mut self) {
        let msg = Diff::Clear;
        self.send_msg(msg);
    }
}

impl<T> Deref for TrackedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

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

#[rustfmt::skip]
impl<O, UO, T, F, Ff> Childable<TrackedMarker> for TrackedForeach<UO, F>
where
    UO: UninitObserver<Observer = O>,
    O: for<'w, 's> Observer<Return<'w, 's> = &'w TrackedVec<T>>,
    F: Fn(TrackedItemObserver<T>) -> Ff + Send + Sync + 'static,
    Ff: FnOnce(&mut McCtx),
    T: Clone + Send + Sync + 'static,
{
    fn insert(self, ctx: &mut Ctx) {
        let parent = ctx.current_entity;
        let f = self.1;

        let list = get_marker_list(ctx.current_entity());
        let group_index = list.len();

        self.0.register_self(ctx.world, |mut obs, world| {
            let (tv, _) = obs.get(world);
            let (tx, rx) = crossbeam_channel::unbounded();

            tx.send(Diff::Init(tv.inner.clone())).unwrap();
            tv.update_out.lock().unwrap().push(tx);

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
