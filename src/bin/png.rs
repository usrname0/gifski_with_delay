use crate::source::{Fps, Source};
use crate::BinResult;
use gifski::Collector;
use std::path::PathBuf;

pub struct Lodecoder {
    frames: Vec<PathBuf>,
    fps: f64,
    custom_delays: Option<Vec<Option<u32>>>,
}

impl Lodecoder {
    pub fn new(frames: Vec<PathBuf>, params: Fps) -> Self {
        Self {
            frames,
            fps: f64::from(params.fps) * f64::from(params.speed),
            custom_delays: None,
        }
    }

    pub fn new_with_delays(frames: Vec<PathBuf>, params: Fps, delays: Vec<Option<u32>>) -> Self {
        Self {
            frames,
            fps: f64::from(params.fps) * f64::from(params.speed),
            custom_delays: Some(delays),
        }
    }
}

impl Source for Lodecoder {
    fn total_frames(&self) -> Option<u64> {
        Some(self.frames.len() as u64)
    }

    #[inline(never)]
    fn collect(&mut self, dest: &mut Collector) -> BinResult<()> {
        let dest = &*dest;
        let f = std::mem::take(&mut self.frames);
        let delays = self.custom_delays.take();
        let frame_count = f.len();

        let default_frame_duration = 1.0 / self.fps;

        // gifski derives a frame's duration from the gap to the *next* frame's
        // presentation timestamp. The last frame has no next frame, so a custom
        // delay on it would otherwise be ignored (gifski falls back to the
        // previous inter-frame gap). gifski's workaround: a non-zero first-frame
        // pts is treated as a global shift *and* as the last frame's delay. So we
        // offset every frame by the last frame's intended delay; gifski shifts it
        // back to start at 0 and gives the final frame exactly that duration.
        // Other frames' durations are gaps, which are unaffected by the shift.
        // NB: the offset must be > 10ms (1/100s) to trigger gifski's rule.
        let last_frame_delay = delays.as_ref()
            .and_then(|d| d.get(frame_count.wrapping_sub(1)).copied())
            .flatten();
        let pts_offset = match last_frame_delay {
            Some(ms) => f64::from(ms) / 1000.0,
            None => 0.0,
        };

        let mut accumulated_time = pts_offset;

        for (i, frame) in f.into_iter().enumerate() {
            let presentation_timestamp = accumulated_time;
            
            // Calculate duration for this frame
            let frame_duration = if let Some(ref delays) = delays {
                if let Some(Some(custom_delay_ms)) = delays.get(i) {
                    // Use custom delay: convert milliseconds to seconds
                    (*custom_delay_ms as f64) / 1000.0
                } else {
                    // Use default FPS timing
                    default_frame_duration
                }
            } else {
                // No custom delays, use default FPS timing
                default_frame_duration
            };
            
            dest.add_frame_png_file(i, frame, presentation_timestamp)?;
            accumulated_time += frame_duration;
        }
        Ok(())
    }
}