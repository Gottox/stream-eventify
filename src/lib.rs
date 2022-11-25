use pin_project_lite::pin_project;
use std::{collections::HashSet, hash::Hash, task::Poll};
use tokio_stream::Stream;

#[derive(Debug, PartialEq)]
pub enum Action<T> {
    Add(T),
    Remove(T),
}

#[derive(Debug)]
struct Inner<T> {
    last: HashSet<T>,
    added_queue: Vec<T>,
    removed_queue: Vec<T>,
}

impl<T: Hash + Eq + Clone> Inner<T> {
    fn new() -> Self {
        Self {
            last: HashSet::new(),
            added_queue: Vec::new(),
            removed_queue: Vec::new(),
        }
    }

    fn next_from_queue(&mut self) -> Option<Action<T>> {
        if let Some(added) = self.added_queue.pop() {
            return Some(Action::Add(added));
        }
        if let Some(removed) = self.removed_queue.pop() {
            return Some(Action::Remove(removed));
        }
        None
    }

    fn update<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let new_state: HashSet<T> = HashSet::from_iter(iter);

        self.added_queue
            .extend(new_state.difference(&self.last).cloned());
        self.removed_queue
            .extend(self.last.difference(&new_state).cloned());

        self.last = new_state;
    }
}

pin_project! {
pub struct StreamDiff<S, I, T>
where
    S: Stream<Item = I>,
    I: IntoIterator<Item = T>,
{
    #[pin]
    stream: S,
    inner: Inner<T>,
}
}

impl<S, I, T> StreamDiff<S, I, T>
where
    S: Stream<Item = I>,
    I: IntoIterator<Item = T>,
    T: Clone + Hash + Eq,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            inner: Inner::new(),
        }
    }
}

impl<S, I, T> Stream for StreamDiff<S, I, T>
where
    S: Stream<Item = I>,
    I: IntoIterator<Item = T>,
    T: Hash + Eq + Sized + Clone,
{
    type Item = Action<T>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        loop {
            let me = self.as_mut().project();

            if let Some(item) = me.inner.next_from_queue() {
                return Poll::Ready(Some(item));
            }

            let item = match me.stream.poll_next(cx) {
                Poll::Ready(Some(item)) => item,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            me.inner.update(item);
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

trait StreamDiffExt<I, T>
where
    Self: Stream<Item = I> + Sized,
    I: IntoIterator<Item = T>,
    T: Clone + Hash + Eq,
{
    fn diff(self) -> StreamDiff<Self, I, T>;
}

impl<S, I, T> StreamDiffExt<I, T> for S
where
    S: Stream<Item = I> + Sized,
    I: IntoIterator<Item = T>,
    T: Clone + Hash + Eq,
{
    fn diff(self) -> StreamDiff<Self, I, T> {
        StreamDiff::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test() {
        let states = [
            vec![1],
            vec![1],
            vec![1, 2],
            vec![1, 2, 3],
            vec![1, 3],
            vec![1, 2, 3, 4],
            vec![],
        ];
        let mut stream_diff = tokio_stream::iter(states).diff();

        assert_eq!(stream_diff.next().await, Some(Action::Add(1)));
        assert_eq!(stream_diff.next().await, Some(Action::Add(2)));
        assert_eq!(stream_diff.next().await, Some(Action::Add(3)));
        assert_eq!(stream_diff.next().await, Some(Action::Remove(2)));

        let take_2 = [stream_diff.next().await, stream_diff.next().await];
        assert_eq!(take_2.len(), 2);
        assert!(take_2.contains(&Some(Action::Add(2))));
        assert!(take_2.contains(&Some(Action::Add(4))));

        let take_4 = [
            stream_diff.next().await,
            stream_diff.next().await,
            stream_diff.next().await,
            stream_diff.next().await,
        ];
        assert_eq!(take_4.len(), 4);
        assert!(take_4.contains(&Some(Action::Remove(1))));
        assert!(take_4.contains(&Some(Action::Remove(2))));
        assert!(take_4.contains(&Some(Action::Remove(3))));
        assert!(take_4.contains(&Some(Action::Remove(4))));

        assert_eq!(stream_diff.next().await, None);
    }
}
