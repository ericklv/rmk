//! Per-half WS2812 driver running a port of QMK's RGB_MATRIX_DUAL_BEACON effect.

use embassy_rp::Peri;
use embassy_rp::dma::ChannelInstance;
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::peripherals::{PIN_10, PIO1};
use embassy_rp::pio::{Common, Pio};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program};
use embassy_rp::{bind_interrupts, dma, pio};
use embassy_time::Instant;
use rmk::event::LayerChangeEvent;
use rmk::macros::processor;
use smart_leds::RGB8;

/// LEDs per half (crkbd rev4_1: 21 per-key + 2 inner-column).
pub const NUM_LEDS: usize = 23;

/// QMK build settings (VialRGB): hue 250, sat 125 (#ff8291), speed 202; val lowered from 82 to 30.
const HUE: u8 = 250;
const SAT: u8 = 125;
const VAL: u8 = 30;
const SPEED: u8 = 202;

/// QMK `k_rgb_matrix_center`.
const CENTER: (i32, i32) = (112, 32);

/// LED (x, y) in chain order per half [left, right], from QMK crkbd/rev4_1/standard keyboard.json.
pub const POINTS: [[(u8, u8); NUM_LEDS]; 2] = [[
    (95, 63), (85, 39), (85, 21), (85, 4), (68, 2), (68, 19), (68, 37), (80, 58),
    (60, 55), (50, 35), (50, 13), (50, 0), (33, 3), (33, 20), (33, 37), (16, 42),
    (16, 24), (16, 7), (0, 7), (0, 24), (0, 41), (103, 17), (103, 24),
], [
    (129, 63), (139, 39), (139, 21), (139, 4), (156, 2), (156, 19), (156, 37), (144, 58),
    (164, 55), (174, 35), (174, 13), (174, 0), (191, 3), (191, 20), (191, 37), (208, 42),
    (208, 24), (208, 7), (224, 7), (224, 24), (224, 41), (122, 17), (122, 24),
]];

bind_interrupts!(pub struct PioIrqs {
    PIO1_IRQ_0 => pio::InterruptHandler<PIO1>;
});

// The processor macro needs a subscription; layer changes are ignored, like in QMK.
#[processor(subscribe = [LayerChangeEvent], poll_interval = 33)]
pub struct RgbProcessor {
    leds: PioWs2812<'static, PIO1, 0, NUM_LEDS, Grb>,
    // Dropping Common would release the PIO block.
    _common: Common<'static, PIO1>,
    points: &'static [(u8, u8); NUM_LEDS],
}

impl RgbProcessor {
    pub fn new<D: ChannelInstance>(
        pio: Peri<'static, PIO1>,
        pin: Peri<'static, PIN_10>,
        dma: Peri<'static, D>,
        dma_irq: impl Binding<D::Interrupt, dma::InterruptHandler<D>> + 'static,
        points: &'static [(u8, u8); NUM_LEDS],
    ) -> Self {
        let Pio { mut common, sm0, .. } = Pio::new(pio, PioIrqs);
        let program = PioWs2812Program::new(&mut common);
        let leds = PioWs2812::new(&mut common, sm0, dma, dma_irq, pin, &program);
        Self { leds, _common: common, points }
    }

    async fn on_layer_change_event(&mut self, _event: LayerChangeEvent) {}

    /// QMK effect_runner_sin_cos_i + DUAL_BEACON_math, same integer math.
    async fn poll(&mut self) {
        let timer = Instant::now().as_millis() as u16;
        let time = ((timer as u32 * (SPEED / 4) as u32) >> 8) as u8;
        let cos_v = sin8(time.wrapping_add(64)) as i32 - 128;
        let sin_v = sin8(time) as i32 - 128;

        let mut frame = [RGB8::default(); NUM_LEDS];
        for (led, &(x, y)) in frame.iter_mut().zip(self.points.iter()) {
            let (dx, dy) = (x as i32 - CENTER.0, y as i32 - CENTER.1);
            // QMK passes (cos, sin) into params named (sin, cos).
            let hue = (HUE as i32 + (dy * sin_v + dx * cos_v) / 128) as u8;
            *led = hsv_to_rgb(hue, SAT, VAL);
        }
        self.leds.write(&frame).await;
    }
}

/// FastLED/lib8tion sin8_C: 0..=255 maps to sin * 128 + 128.
fn sin8(theta: u8) -> u8 {
    const B_M16: [u8; 8] = [0, 49, 49, 41, 90, 27, 117, 10];
    let mut offset = if theta & 0x40 != 0 { 255 - theta } else { theta };
    offset &= 0x3F;
    let mut secoffset = offset & 0x0F;
    if theta & 0x40 != 0 {
        secoffset += 1;
    }
    let section = (offset >> 4) as usize;
    let (b, m16) = (B_M16[section * 2], B_M16[section * 2 + 1]);
    let mx = ((m16 as u16 * secoffset as u16) >> 4) as u8;
    let mut y = mx.wrapping_add(b) as i8;
    if theta & 0x80 != 0 {
        y = y.wrapping_neg();
    }
    (y as u8).wrapping_add(128)
}

/// QMK hsv_to_rgb_impl without the CIE curve.
fn hsv_to_rgb(h: u8, s: u8, v: u8) -> RGB8 {
    if s == 0 {
        return RGB8::new(v, v, v);
    }
    let (h, s, v) = (h as u16, s as u16, v as u16);
    let region = h * 6 / 255;
    let remainder = ((h * 2 - region * 85) * 3) as u8 as u16;
    let p = ((v * (255 - s)) >> 8) as u8;
    let q = ((v * (255 - ((s * remainder) >> 8))) >> 8) as u8;
    let t = ((v * (255 - ((s * (255 - remainder)) >> 8))) >> 8) as u8;
    let v = v as u8;
    match region {
        0 | 6 => RGB8::new(v, t, p),
        1 => RGB8::new(q, v, p),
        2 => RGB8::new(p, v, t),
        3 => RGB8::new(p, q, v),
        4 => RGB8::new(t, p, v),
        _ => RGB8::new(v, p, q),
    }
}
