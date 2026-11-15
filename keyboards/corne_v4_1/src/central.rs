#![no_main]
#![no_std]

mod rgb;

use rmk::macros::rmk_central;

#[rmk_central]
mod keyboard_central {
    // DMA_CH0 is free on the central; its IRQ is bound by the generated `Irqs`.
    #[register_processor(poll)]
    fn rgb() -> crate::rgb::RgbProcessor {
        crate::rgb::RgbProcessor::new(p.PIO1, p.PIN_10, p.DMA_CH0, Irqs, &crate::rgb::POINTS[0])
    }
}
