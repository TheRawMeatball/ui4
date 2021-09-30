use std::collections::BTreeMap;
use std::sync::Mutex;

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

impl<K: Eq + Ord + Clone, V: Clone> TrackedMap<K, V> {
    fn send_msg(&mut self, msg: &Diff<(K, V)>) {
        self.update_out
            .get_mut()
            .unwrap()
            .retain(|tx| tx.send(msg.clone()).is_ok());
    }

    fn index_of_key(&mut self, k: &K) -> usize {
        let mut split = self.inner.split_off(k);
        let i = self.inner.len();
        self.inner.append(&mut split);
        i
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, k: K, v: V) {
        let old = self.inner.insert(k.clone(), v.clone());
        let index = self.index_of_key(&k);

        if old.is_some() {
            self.send_msg(&Diff::Replace((k, v), index));
        } else {
            self.send_msg(&Diff::Insert((k, v), index));
        }
    }

    pub fn remove(&mut self, k: K) -> Option<V> {
        if !self.inner.contains_key(&k) {
            return None;
        }
        let index = self.index_of_key(&k);
        self.send_msg(&Diff::Remove(index));
        self.inner.remove(&k)
    }
}

impl<K: Clone + 'static, V: Clone + 'static> Tracked for TrackedMap<K, V> {
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
