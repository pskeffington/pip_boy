//! Source-derived Rust-Boy CRT operator sprite atlas.
//!
//! The approved image supplies the face and uniform. This module owns only
//! deterministic frame selection and atlas geometry; it does not query sensors,
//! create timers, or mount a STATUS screen. The launcher may load it later.

use std::time::Duration;

pub const FRAME_WIDTH: u32 = 196;
pub const FRAME_HEIGHT: u32 = 300;
pub const ATLAS_COLUMNS: u32 = 3;
pub const WALK_FRAMES: u32 = 8;
pub const IDLE_FRAME: u32 = 8;
pub const WALK_CYCLE_MS: u64 = 800;

/// Embedded transparent PNG compiled into the loopback server executable.
pub const ATLAS_PNG: &[u8] = include_bytes!("../assets/avatar-approved-atlas.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Idle,
    Walk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniformPanel {
    FrontRight,
    FrontLeft,
    BackLeft,
    BackRight,
}

/// The user's four-quadrant garment contract, relative to the wearer.
/// Never horizontally mirror an atlas frame: the rod and wrist device are asymmetric.
pub const ROD_PANEL: UniformPanel = UniformPanel::FrontLeft;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteFrame {
    pub index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Pure, deterministic frame sampler: native toolkits may decode `ATLAS_PNG`
/// once, then use this rectangle to draw the selected subimage in their viewport.
#[allow(dead_code)] // The native STATUS/host consumer is a separate integration gate.
pub fn frame_at(motion: Motion, elapsed: Duration) -> SpriteFrame {
    let index = if motion == Motion::Idle {
        IDLE_FRAME
    } else {
        ((elapsed.as_millis() % u128::from(WALK_CYCLE_MS)) * u128::from(WALK_FRAMES)
            / u128::from(WALK_CYCLE_MS)) as u32
    };
    SpriteFrame {
        index,
        x: (index % ATLAS_COLUMNS) * FRAME_WIDTH,
        y: (index / ATLAS_COLUMNS) * FRAME_HEIGHT,
        width: FRAME_WIDTH,
        height: FRAME_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_is_rgba_png_of_expected_size() {
        assert!(ATLAS_PNG.len() > 10_000);
        assert_eq!(&ATLAS_PNG[..8], b"\x89PNG\r\n\x1a\n");
        let width = u32::from_be_bytes(ATLAS_PNG[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(ATLAS_PNG[20..24].try_into().unwrap());
        assert_eq!(width, ATLAS_COLUMNS * FRAME_WIDTH);
        assert_eq!(height, ATLAS_COLUMNS * FRAME_HEIGHT);
        assert_eq!(ATLAS_PNG[25], 6, "RGBA color type expected");
    }

    #[test]
    fn exact_walk_loop_with_stationary_idle() {
        for index in 0..WALK_FRAMES {
            let f = frame_at(Motion::Walk, Duration::from_millis(index as u64 * 100));
            assert_eq!(f.index, index);
            assert_eq!(f.width, FRAME_WIDTH);
            assert_eq!(f.height, FRAME_HEIGHT);
            assert!(f.x + f.width <= FRAME_WIDTH * ATLAS_COLUMNS);
            assert!(f.y + f.height <= FRAME_HEIGHT * ATLAS_COLUMNS);
        }
        assert_eq!(frame_at(Motion::Walk, Duration::from_millis(799)).index, 7);
        assert_eq!(frame_at(Motion::Walk, Duration::from_millis(800)).index, 0);
        assert_eq!(frame_at(Motion::Walk, Duration::from_millis(1_000_800)).index, 0);
        assert_eq!(frame_at(Motion::Idle, Duration::ZERO).index, IDLE_FRAME);
        assert_eq!(frame_at(Motion::Idle, Duration::from_secs(2)).index, IDLE_FRAME);
    }

    #[test]
    fn only_front_left_is_marked() {
        let quadrants = [
            UniformPanel::FrontRight,
            UniformPanel::FrontLeft,
            UniformPanel::BackLeft,
            UniformPanel::BackRight,
        ];
        assert_eq!(quadrants.into_iter().filter(|panel| *panel == ROD_PANEL).count(), 1);
        assert_eq!(ROD_PANEL, UniformPanel::FrontLeft);
    }
}
