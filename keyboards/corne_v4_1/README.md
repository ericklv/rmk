# Corne v4.1 — RMK

RMK v0.9 port of the [`cornev4`](https://github.com/ericklv/qmk-vial/tree/main/cornev4) QMK/Vial keymap for the foostan Corne v4.1 (RP2040, direct-pin, half-duplex split on GP12).

- Left half = central (connect USB here), right half = peripheral.
- Layers 0–5 identical to QMK; RGB matrix keys on layer 3 map to `No` (RMK has no RGB matrix support). Encoders not configured.
- Debounce 8 ms, split serial 115200 baud (PIO half-duplex), Vial unlock Q+P, Vial UID kept from QMK.

## Build

```bash
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/central firmware/corne_v4_1_central.uf2
elf2uf2-rs target/thumbv6m-none-eabi/release/peripheral firmware/corne_v4_1_peripheral.uf2
```

## Flash

Hold **Q** (left) / **P** (right) while plugging USB into that half, then copy the matching `.uf2` to `RPI-RP2`. Never hot-plug the TRRS cable.
