use std::ops::Deref;

use crossbeam_channel::{Receiver, Sender};

use super::{Diff, Tracked, TrackedId};

pub struct TrackedVec<T> {
    inner: Vec<T>,
    id: TrackedId,
    update_out: Vec<Sender<Diff>>,
}

impl<T> Default for TrackedVec<T> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            update_out: Default::default(),
            id: TrackedId::new(),
        }
    }
}

impl<T> TrackedVec<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn send_msg(&mut self, msg: Diff) {
        self.update_out.retain(|tx| tx.send(msg.clone()).is_ok());
    }

    pub fn push(&mut self, val: T) {
        self.send_msg(Diff::Insert(self.inner.len()));
        self.inner.push(val);
    }

    pub fn pop(&mut self) -> Option<T> {
        let out = self.inner.pop();
        self.send_msg(Diff::Remove(self.inner.len()));
        out
    }

    pub fn get_mut(&mut self, i: usize) -> &mut T {
        self.send_msg(Diff::Modify(i));
        &mut self.inner[i]
    }

    pub fn remove(&mut self, i: usize) -> T {
        self.send_msg(Diff::Remove(i));
        self.inner.remove(i)
    }

    pub fn insert(&mut self, val: T, i: usize) {
        self.send_msg(Diff::Insert(i));
        self.inner.insert(i, val)
    }

    pub fn clear(&mut self) {
        self.send_msg(Diff::Clear);
        self.inner.clear();
    }
}

impl<T> Deref for TrackedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: 'static> Tracked for TrackedVec<T> {
    type Item = T;

    fn register(&mut self) -> Receiver<Diff> {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.update_out.push(tx);
        rx
    }

    fn id(&self) -> TrackedId {
        self.id
    }

    fn get(&self, index: usize) -> &Self::Item {
        &self.inner[index]
    }

    fn get_mut(&mut self, index: usize) -> &mut Self::Item {
        self.get_mut(index)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}
