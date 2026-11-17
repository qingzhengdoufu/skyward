//! Bounded CPU-only transport benchmark: never opens GPU, DRM or partitions.
use std::{hint::black_box, time::Instant};
fn main() -> anyhow::Result<()> {
    let frame: Vec<u8> = (0..skyward_browser::protocol::FRAME_BYTES).map(|i| (i % 251) as u8).collect();
    for (width, height) in [(1080, 2100), (1440, 2436), (1440, 3200)] {
        let start = Instant::now();
        for _ in 0..12 {
            black_box(skyward_browser::session::rgba_frame(black_box(&frame), width, height)?);
        }
        println!(
            "{width}x{height}: {:.2} ms/frame (12 CPU conversions, not webpage FPS)",
            start.elapsed().as_secs_f64() * 1000.0 / 12.0
        );
    }
    Ok(())
}
