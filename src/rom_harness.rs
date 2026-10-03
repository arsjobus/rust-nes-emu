//! Headless ROM runner used to run blargg-style test ROMs.
//! Usage: ROM=path/to/test.nes FRAMES=1500 cargo test rom_harness -- --nocapture --ignored
use crate::{cartridge::Cartridge, nes::Nes};

#[test]
#[ignore]
fn rom_harness() {
    let path = std::env::var("ROM").expect("set ROM=path");
    let frames: usize = std::env::var("FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(1500);
    let cart = Cartridge::load(&path).unwrap();
    let mut nes = Nes::new(cart);
    let mut last_status = 0xffu8;
    // Status $81 means "press the reset button now" (apu_reset, cpu_reset).
    let mut reset_requested = false;
    for f in 0..frames {
        nes.run_frame();
        let sig = [
            nes.bus.read(0x6001),
            nes.bus.read(0x6002),
            nes.bus.read(0x6003),
        ];
        if sig == [0xde, 0xb0, 0x61] {
            let st = nes.bus.read(0x6000);
            last_status = st;
            if st == 0x81 && !reset_requested {
                reset_requested = true;
                // The ROMs ask for the reset to happen within ~100 ms.
                for _ in 0..6 {
                    nes.run_frame();
                }
                nes.reset();
                continue;
            }
            if st != 0x81 {
                reset_requested = false;
            }
            if st != 0x80 && st != 0x81 {
                println!("frame {f}: status {st:#04x}");
                break;
            }
        }
    }
    if let Ok(out) = std::env::var("DUMP_PPM") {
        let mut buf = format!("P6\n256 240\n255\n").into_bytes();
        for px in nes.framebuffer() {
            buf.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]);
        }
        std::fs::write(out, buf).unwrap();
    }
    println!("PC={:#06x}", nes.cpu.pc);
    hang_report(&mut nes);
    let mut text = String::new();
    let mut a = 0x6004u16;
    loop {
        let b = nes.bus.read(a);
        if b == 0 || a > 0x7fff { break; }
        text.push(b as char);
        a += 1;
    }
    println!("RESULT status={last_status:#04x} zp_f8={:#04x}\n{text}", nes.bus.ram[0xf8]);
    assert_eq!(
        last_status, 0,
        "test ROM did not report success (0x00); status was {last_status:#04x}: {text}"
    );
}

/// Single-steps the CPU for a while and prints what the program is
/// looping on: visited addresses with their bytes, plus CPU/PPU/APU state.
fn hang_report(nes: &mut Nes) {
    use std::collections::BTreeSet;
    let mut visited = BTreeSet::new();
    let mut order: Vec<u16> = Vec::new();
    for _ in 0..2000 {
        let pc = nes.cpu.pc;
        if visited.insert(pc) {
            order.push(pc);
        }
        let cycles = nes.cpu.step(&mut nes.bus) + nes.bus.take_dma_stall();
        let already = nes.bus.take_ppu_advanced();
        nes.bus.ppu.catch_up((cycles * 3) as i32 - already);
        let apu_done = nes.bus.take_apu_advanced();
        nes.bus.clock_apu(cycles.saturating_sub(apu_done));
        if nes.bus.ppu.nmi_pending && !nes.bus.ppu.nmi_delay {
            nes.bus.ppu.nmi_pending = false;
            let c = nes.cpu.nmi(&mut nes.bus);
            nes.bus.ppu.catch_up((c * 3) as i32);
            nes.bus.clock_apu(c);
        }
    }
    println!("--- hang report ---");
    println!(
        "CPU a={:02x} x={:02x} y={:02x} sp={:02x} p={:02x}",
        nes.cpu.a, nes.cpu.x, nes.cpu.y, nes.cpu.sp, nes.cpu.status
    );
    let p = &nes.bus.ppu;
    println!(
        "PPU ctrl={:02x} mask={:02x} status={:02x} scanline={} dot={}",
        p.ctrl, p.mask, p.status, p.scanline, p.dot
    );
    println!(
        "IRQ apu={} mapper={}",
        nes.bus.apu.irq_line(),
        nes.bus.ppu.cart.irq_pending()
    );
    println!("distinct PCs in 2000 steps: {}", order.len());
    let mut sorted: Vec<u16> = order.clone();
    sorted.sort();
    for pc in sorted.iter().take(48) {
        let b: Vec<String> = (0..3)
            .map(|i| format!("{:02x}", nes.bus.ppu.cart.cpu_read(pc.wrapping_add(i))))
            .collect();
        println!("  {:04x}: {}", pc, b.join(" "));
    }
    print!("zero page 00-1f:");
    for i in 0..32 {
        print!(" {:02x}", nes.bus.ram[i]);
    }
    println!();
}
