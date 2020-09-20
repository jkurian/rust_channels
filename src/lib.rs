use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders += 1;
        Sender {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders -= 1;
        let was_last = inner.senders == 0;
        drop(inner);
        if was_last {
            self.shared.available.notify_one();
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&mut self, value: T) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(value);
        drop(inner);
        self.shared.available.notify_one();
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            println!("{} {}", inner.senders, inner.queue.len());
            match inner.queue.pop_front() {
                Some(t) => return Some(t),
                None if inner.senders == 0 => return None,
                None => inner = self.shared.available.wait(inner).unwrap(),
            }
        }
    }
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Shared {
        inner: Mutex::new(Inner {
            queue: VecDeque::new(),
            senders: 1,
        }),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);

    (
        Sender {
            shared: shared.clone(),
        },
        Receiver {
            shared: shared.clone(),
        },
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn ping_pong() {
        let (mut tx, mut rx) = channel();
        tx.send(25);
        assert_eq!(Some(25), rx.recv())
    }

    #[test]
    pub fn send_drop() {
        let (tx, mut rx) = channel::<()>();
        drop(tx);

        assert_eq!(rx.recv(), None)
    }
}
