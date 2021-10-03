use std::sync::Arc;

use bevy::prelude::*;

#[derive(Clone)]
pub struct ButtonFunc(Arc<dyn Fn(&mut World) + Send + Sync>);
impl ButtonFunc {
    pub fn new(f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
    pub(crate) fn run(&self, world: &mut World) {
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
pub struct FuncScratch(pub(crate) Interaction);

pub(crate) struct ButtonSystemState {
    pub query: QueryState<
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
    pub button_list: Vec<ButtonFunc>,
}

impl FromWorld for ButtonSystemState {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
            button_list: vec![],
        }
    }
}

impl ButtonSystemState {
    pub(crate) fn run(&mut self, world: &mut World) {
        self.button_list
            .extend(self.query.iter_mut(world).filter_map(
                |(c, h, dc, dh, mut scratch, interaction)| {
                    let old = scratch.0;
                    scratch.0 = *interaction;
                    match interaction {
                        Interaction::Clicked => c.map(|x| &x.0).cloned(),
                        Interaction::Hovered => match old {
                            Interaction::Clicked => {
                                match (dc.map(|x| &x.0).cloned(), h.map(|x| &x.0).cloned()) {
                                    (Some(dc), Some(h)) => Some(ButtonFunc::new(move |w| {
                                        dc.run(w);
                                        h.run(w);
                                    })),
                                    (Some(dc), None) => Some(dc),
                                    (None, Some(h)) => Some(h),
                                    (None, None) => None,
                                }
                            }
                            Interaction::Hovered => h.map(|x| &x.0).cloned(),
                            Interaction::None => h.map(|x| &x.0).cloned(),
                        },
                        Interaction::None => match old {
                            Interaction::Clicked => dc.map(|x| &x.0).cloned(),
                            Interaction::Hovered => dh.map(|x| &x.0).cloned(),
                            Interaction::None => None,
                        },
                    }
                },
            ));

        for func in self.button_list.drain(..) {
            func.run(world);
        }
    }
}
