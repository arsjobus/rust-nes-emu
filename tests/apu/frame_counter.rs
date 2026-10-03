use super::*;
use crate::apu::{noise::Noise, pulse::Pulse, triangle::Triangle};

struct Channels {
    p1: Pulse,
    p2: Pulse,
    m1: Pulse,
    m2: Pulse,
    tri: Triangle,
    noise: Noise,
}

fn channels() -> Channels {
    Channels {
        p1: Pulse::new(true),
        p2: Pulse::new(false),
        m1: Pulse::new(true),
        m2: Pulse::new(false),
        tri: Triangle::new(),
        noise: Noise::new(),
    }
}

fn run(fc: &mut FrameCounter, ch: &mut Channels, cycles: u32) {
    for _ in 0..cycles {
        fc.clock(&mut ch.p1, &mut ch.p2, &mut ch.m1, &mut ch.m2, &mut ch.tri, &mut ch.noise);
    }
}

#[test]
fn four_step_mode_raises_irq_once_per_29830_cycles() {
    let mut fc = FrameCounter::new();
    let mut ch = channels();
    run(&mut fc, &mut ch, 29_827);
    assert!(!fc.irq_flag);
    run(&mut fc, &mut ch, 1);
    assert!(fc.irq_flag);
}

#[test]
fn irq_inhibit_and_five_step_mode_suppress_the_irq() {
    let mut fc = FrameCounter::new();
    let mut ch = channels();
    fc.write(0x40);
    run(&mut fc, &mut ch, 40_000);
    assert!(!fc.irq_flag);

    fc.write(0x80);
    run(&mut fc, &mut ch, 40_000);
    assert!(!fc.irq_flag);
}

#[test]
fn writing_inhibit_clears_a_pending_irq() {
    let mut fc = FrameCounter::new();
    let mut ch = channels();
    run(&mut fc, &mut ch, 29_829);
    assert!(fc.irq_flag);
    fc.write(0x40);
    assert!(!fc.irq_flag);
}

/// Length counters tick at ~120 Hz (every 14913 CPU cycles), not twice
/// as fast.
#[test]
fn length_counters_are_clocked_every_14913_cycles() {
    let mut fc = FrameCounter::new();
    let mut ch = channels();
    ch.noise.length = 10;
    run(&mut fc, &mut ch, 14_912);
    assert_eq!(ch.noise.length, 10);
    run(&mut fc, &mut ch, 1);
    assert_eq!(ch.noise.length, 9);
}
