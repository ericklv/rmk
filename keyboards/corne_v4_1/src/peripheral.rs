#![no_main]
#![no_std]

mod rgb;

use embassy_rp::peripherals::DMA_CH0;
use embassy_rp::{bind_interrupts, dma};
use rmk::macros::rmk_peripheral;

bind_interrupts!(struct DmaIrqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

#[rmk_peripheral(id = 0)]
mod keyboard_peripheral {
    #[register_processor(poll)]
    fn rgb() -> crate::rgb::RgbProcessor {
        crate::rgb::RgbProcessor::new(p.PIO1, p.PIN_10, p.DMA_CH0, crate::DmaIrqs)
    }
}
