//! VialRGB protocol state shared between the Vial host service, storage, split link and the user's LED driver.

use core::cell::Cell;

use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::watch::Watch;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

/// VialRGB protocol version reported to the host.
pub const VIALRGB_PROTOCOL_VERSION: u16 = 1;

/// VialRGB effect ids (subset of vial-qmk `vialrgb_effects.inc`).
pub mod effect {
    pub const OFF: u16 = 0;
    pub const SOLID_COLOR: u16 = 2;
    pub const BREATHING: u16 = 6;
    pub const DUAL_BEACON: u16 = 21;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RgbConfig {
    pub mode: u16,
    pub speed: u8,
    pub hue: u8,
    pub sat: u8,
    pub val: u8,
}

/// Current RGB config. Senders: `init`, Vial, storage load (central), split link (peripheral).
pub static RGB_CONFIG: Watch<crate::RawMutex, RgbConfig, 4> = Watch::new();

/// (max brightness, supported effect ids), set once by `init`.
static INFO: Mutex<crate::RawMutex, Cell<(u8, &'static [u16])>> = Mutex::new(Cell::new((255, &[])));

/// Register the driver's capabilities and default config; call before the keyboard tasks run.
pub fn init(default: RgbConfig, max_brightness: u8, supported: &'static [u16]) {
    INFO.lock(|c| c.set((max_brightness, supported)));
    RGB_CONFIG.sender().send(clamp(default));
}

pub(crate) fn max_brightness() -> u8 {
    INFO.lock(|c| c.get().0)
}

pub(crate) fn supported() -> &'static [u16] {
    INFO.lock(|c| c.get().1)
}

pub(crate) fn clamp(mut cfg: RgbConfig) -> RgbConfig {
    cfg.val = cfg.val.min(max_brightness());
    cfg
}

/// Apply a config coming from the host or storage, ignoring unsupported modes.
pub(crate) fn apply(cfg: RgbConfig) {
    if supported().contains(&cfg.mode) {
        RGB_CONFIG.sender().send(clamp(cfg));
    }
}

/// Load the persisted config from storage (central only).
#[cfg(feature = "storage")]
pub async fn load_from_storage() {
    if let Some(cfg) = crate::storage::read_rgb_config().await {
        apply(cfg);
    }
}
