//! Per-half WS2812 driver: layer color, caps lock override, breathing on the base layer.

use embassy_rp::Peri;
use embassy_rp::dma::ChannelInstance;
use embassy_rp::interrupt::typelevel::Binding;
use embassy_rp::peripherals::{PIN_10, PIO1};
use embassy_rp::pio::{Common, Pio};
use embassy_rp::pio_programs::ws2812::{Grb, PioWs2812, PioWs2812Program};
use embassy_rp::{bind_interrupts, dma, pio};
use embassy_time::Instant;
use rmk::event::{LayerChangeEvent, LedIndicatorEvent};
use rmk::macros::processor;
use smart_leds::RGB8;

/// LEDs per half (crkbd rev4_1: 21 per-key + 2 inner-column).
pub const NUM_LEDS: usize = 23;
/// Same ceiling as the QMK build (RGB_MATRIX_MAXIMUM_BRIGHTNESS 120).
const MAX_BRIGHTNESS: u8 = 120;
/// Base-layer breathing period.
const BREATH_PERIOD_MS: u64 = 4000;

bind_interrupts!(pub struct PioIrqs {
    PIO1_IRQ_0 => pio::InterruptHandler<PIO1>;
});

/// Base color per layer; unknown layers fall back to the last entry.
const LAYER_COLORS: [RGB8; 4] = [
    RGB8::new(0, 90, 255),  // 0 base: blue (breathing)
    RGB8::new(0, 255, 80),  // 1 num: green
    RGB8::new(255, 110, 0), // 2 fn: orange
    RGB8::new(255, 0, 40),  // 3 sys: red
];
const CAPS_COLOR: RGB8 = RGB8::new(255, 255, 255);

#[processor(subscribe = [LayerChangeEvent, LedIndicatorEvent], poll_interval = 33)]
pub struct RgbProcessor {
    leds: PioWs2812<'static, PIO1, 0, NUM_LEDS, Grb>,
    // Dropping Common would release the PIO block.
    _common: Common<'static, PIO1>,
    layer: u8,
    caps: bool,
    last: Option<[RGB8; NUM_LEDS]>,
}

impl RgbProcessor {
    pub fn new<D: ChannelInstance>(
        pio: Peri<'static, PIO1>,
        pin: Peri<'static, PIN_10>,
        dma: Peri<'static, D>,
        dma_irq: impl Binding<D::Interrupt, dma::InterruptHandler<D>> + 'static,
    ) -> Self {
        let Pio { mut common, sm0, .. } = Pio::new(pio, PioIrqs);
        let program = PioWs2812Program::new(&mut common);
        let leds = PioWs2812::new(&mut common, sm0, dma, dma_irq, pin, &program);
        Self { leds, _common: common, layer: 0, caps: false, last: None }
    }

    async fn on_layer_change_event(&mut self, event: LayerChangeEvent) {
        self.layer = event.0;
    }

    async fn on_led_indicator_event(&mut self, event: LedIndicatorEvent) {
        self.caps = event.0.caps_lock();
    }

    async fn poll(&mut self) {
        let frame = [self.color(); NUM_LEDS];
        // Skip identical frames to keep the data line quiet.
        if self.last != Some(frame) {
            self.leds.write(&frame).await;
            self.last = Some(frame);
        }
    }

    fn color(&self) -> RGB8 {
        if self.caps {
            return scale(CAPS_COLOR, MAX_BRIGHTNESS);
        }
        let idx = (self.layer as usize).min(LAYER_COLORS.len() - 1);
        let base = LAYER_COLORS[idx];
        if self.layer != 0 {
            return scale(base, MAX_BRIGHTNESS);
        }
        // Triangle wave 15%..100% of MAX_BRIGHTNESS, quantized to limit LED writes.
        let t = Instant::now().as_millis() % BREATH_PERIOD_MS;
        let half = BREATH_PERIOD_MS / 2;
        let tri = if t < half { t } else { BREATH_PERIOD_MS - t };
        let min = MAX_BRIGHTNESS as u64 * 15 / 100;
        let level = min + (MAX_BRIGHTNESS as u64 - min) * tri / half;
        scale(base, (level & !0x3) as u8)
    }
}

fn scale(c: RGB8, level: u8) -> RGB8 {
    let s = |v: u8| ((v as u16 * level as u16) / 255) as u8;
    RGB8::new(s(c.r), s(c.g), s(c.b))
}
