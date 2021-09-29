use std::{ops::Deref, sync::Mutex};

use crossbeam_channel::Sender;

use super::{Diff, Tracked};

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

impl<T: Clone + 'static> Tracked for TrackedVec<T> {
    type Item = T;

    fn register(&self, tx: Sender<Diff<Self::Item>>) {
        tx.send(Diff::Init(self.inner.clone())).unwrap();
        self.update_out.lock().unwrap().push(tx);
    }
}
