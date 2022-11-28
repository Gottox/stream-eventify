# stream-eventify

This crate adds a diff function to streams of Iterables. Each iterator is scanned and
added/removed elements are reported in contrast to the last iteration.

Removed elements are reported before Add events, but the order within a batch of added/removed elements is
undefined.

## example

```rust
# tokio_test::block_on(async {
use stream_eventify::{ StreamDiffExt, Action };
use tokio_stream::StreamExt;

let states = [
	vec!["Chocolate"],
	vec!["Chocolate", "Bonbon"],
	vec!["Chocolate", "Bonbon", "Cookie"],
	vec!["Cake", "Bonbon", "Cookie"]
];

let mut stream_diff = tokio_stream::iter(states).diff();
assert_eq!(stream_diff.next().await, Some(Action::Add("Chocolate")));
assert_eq!(stream_diff.next().await, Some(Action::Add("Bonbon")));
assert_eq!(stream_diff.next().await, Some(Action::Add("Cookie")));
assert_eq!(stream_diff.next().await, Some(Action::Remove("Chocolate")));
assert_eq!(stream_diff.next().await, Some(Action::Add("Cake")));
assert_eq!(stream_diff.next().await, None);
# })
```
