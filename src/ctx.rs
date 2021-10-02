use std::marker::PhantomData;

use bevy::{
    ecs::{prelude::*, world::EntityMut},
    prelude::ControlBundle,
};

use crate::{
    childable::Childable,
    insertable::Insertable,
    observer::{ComponentExistsObserver, ComponentObserver},
};

pub struct Ctx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) current_entity: Entity,
}

pub struct McCtx<'a> {
    pub(crate) world: &'a mut World,
    pub(crate) get_new_child: &'a mut dyn FnMut(&mut World) -> Entity,
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

    pub fn dyn_group<M>(&mut self, children: impl Childable<M>) -> &mut Self {
        self.c(|ctx: &mut Ctx| {
            ctx.with_bundle(ControlBundle::default()).children(children);
        })
    }
}

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

    pub fn has_component<T: Send + Sync + 'static>(&self) -> ComponentExistsObserver<T> {
        ComponentExistsObserver(self.current_entity, PhantomData)
    }

    pub fn this(&self) -> Entity {
        self.current_entity
    }

    pub(crate) fn current_entity(&mut self) -> EntityMut<'_> {
        self.world.entity_mut(self.current_entity)
    }
}
