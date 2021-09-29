use std::collections::BTreeMap;
use std::{marker::PhantomData, sync::Mutex};

use crossbeam_channel::Sender;

use super::{Diff, Tracked};

pub struct TrackedMap<K, V> {
    inner: BTreeMap<K, V>,
    update_out: Mutex<Vec<Sender<Diff<(K, V)>>>>,
}

impl<K, V> Default for TrackedMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            update_out: Default::default(),
        }
    }
}

impl<K: Clone, V: Clone> TrackedMap<K, V> {
    pub fn new() -> Self {
        Self::default()
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

impl<K: Clone, V: Clone> Tracked for TrackedMap<K, V> {
    type Item = (K, V);

    fn register(&self, tx: Sender<Diff<Self::Item>>) {
        tx.send(Diff::Init(
            self.inner
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ))
        .unwrap();
        self.update_out.lock().unwrap().push(tx);
    }
}