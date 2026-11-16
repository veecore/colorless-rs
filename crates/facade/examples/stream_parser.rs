//! A streaming parser demonstrating real sources of color with callbacks.
//!
//! Run with:
//! ```sh
//! cargo run -p colorless --example stream_parser --features relaxed-drop
//! ```

use colorless::function::{from_async_fn, from_fn, ColoredFnMut};
use colorless::{colorless, AsyncExt, IdentityExt};
use colorless_core::test_util::block_on_local_test;

/// A key-value record parsed from the stream.
#[derive(Debug, PartialEq, Eq)]
pub struct Record<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

/// A streaming parser that pulls chunks from `source` and pushes parsed
/// records to `sink`.
///
/// Notice:
/// - In synchronous mode, `source` and `sink` are direct closures.
/// - In asynchronous mode, `source` and `sink` are async closures awaiting I/O.
/// - The parser algorithm is written ONCE.
#[colorless(C)]
pub fn parse_records<'a, S, D>(mut source: S, mut sink: D) -> Result<usize, ()>
where
    S: ColoredFnMut<(), C, Output = Option<&'a str>>,
    D: ColoredFnMut<Record<'a>, C, Output = Result<(), ()>>,
{
    let mut count = 0;

    pin_local!(source);
    pin_local!(sink);
    borrow_mut!(source, sink);
    #[colorless::loop_state(mut source, mut sink, pub mut count)]
    while let Some(line) = reborrow!(source).call_mut(()).resolve {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let record = Record {
                key: key.trim(),
                value: value.trim(),
            };
            reborrow!(sink).call_mut(record).resolve?;
            count += 1;
        }
    }

    Ok(count)
}

fn main() {
    println!("=== 1. Synchronous Execution ===");
    // In sync mode, source is a standard closure pulling from an iterator,
    // and sink is a standard closure pushing to a Vec.
    let lines = ["server=localhost", "port=8080", "# comment", "timeout=30"];
    let mut iter = lines.iter().copied();
    let mut sync_records = Vec::new();

    let sync_source = from_fn(move |()| iter.next());
    let sync_sink = from_fn(|rec: Record<'_>| {
        println!("  [Sync Sink] Parsed: {} => {}", rec.key, rec.value);
        sync_records.push((rec.key.to_string(), rec.value.to_string()));
        Ok(())
    });

    let sync_count = parse_records(sync_source, sync_sink).sync_().unwrap();
    println!("Total sync records: {}\n", sync_count);

    println!("=== 2. Asynchronous Execution ===");
    // In async mode, source is an async closure (e.g. simulating network/disk I/O)
    // and sink is an async closure (e.g. pushing to an async channel/database).
    let async_lines = ["host=127.0.0.1", "worker=4", "debug=true"];
    let mut async_iter = async_lines.iter().copied();
    let mut async_records = Vec::new();

    let async_source = from_async_fn(async move |()| {
        // In real code: socket.read_line().await
        async_iter.next()
    });

    let async_sink = from_async_fn(async |rec: Record<'_>| {
        // In real code: channel.send(rec).await
        println!("  [Async Sink] Streamed: {} => {}", rec.key, rec.value);
        async_records.push((rec.key.to_string(), rec.value.to_string()));
        Ok(())
    });

    let async_future = parse_records(async_source, async_sink).async_();
    let async_count = block_on_local_test(async_future).unwrap();
    println!("Total async records: {}", async_count);

    assert_eq!(sync_count, 3);
    assert_eq!(async_count, 3);
}
