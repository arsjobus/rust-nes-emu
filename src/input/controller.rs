use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};

use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Replay, Ticks};

use std::time::{Duration, Instant};

use super::mapping::{NesButton, NesButtons, map_button};

const AXIS_THRESHOLD: f32 = 0.5;

pub struct Controller {
    gilrs: Option<Gilrs>,
    gamepad_id: Option<GamepadId>,

    /*
     * Active rumble effects, kept alive until their playback
     * duration elapses. gilrs stops (erases) a force-feedback
     * effect as soon as its `Effect` handle is dropped, so these
     * must be held here rather than as locals in `rumble()`.
     */
    active_rumbles: Vec<(Effect, Instant, Duration)>,

    /*
     * USB/gamepad state.
     */
    usb_buttons: NesButtons,

    /*
     * Keyboard state.
     *
     * Kept separate from USB state so keyboard input does
     * not overwrite controller input.
     */
    keyboard_buttons: NesButtons,

    // Right trigger toggles turbo; A and B then pulse while held.
    turbo_enabled: bool,
    right_trigger_pressed: bool,
    turbo_frame: u8,

    /*
     * NES $4016 controller protocol.
     */
    strobe: bool,
    shift_register: u8,
}

impl Controller {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),

            Err(error) => {
                eprintln!("Warning: gamepad support unavailable: {}", error);

                None
            }
        };

        let gamepad_id = gilrs
            .as_ref()
            .and_then(|gilrs| gilrs.gamepads().next().map(|(id, _)| id));

        let controller = Self {
            gilrs,
            gamepad_id,
            active_rumbles: Vec::new(),
            usb_buttons: NesButtons::default(),
            keyboard_buttons: NesButtons::default(),
            turbo_enabled: false,
            right_trigger_pressed: false,
            turbo_frame: 0,
            strobe: false,
            shift_register: 0,
        };

        if let Some(id) = controller.gamepad_id {
            if let Some(gilrs) = controller.gilrs.as_ref() {
                let gamepad = gilrs.gamepad(id);

                println!("Controller connected: {}", gamepad.name());

                println!("  ID: {:?}", id);
            }
        } else {
            println!("No USB controller detected.");
        }

        controller
    }

    /*
     * ---------------------------------------------------------
     * Update USB controller
     * ---------------------------------------------------------
     */
    pub fn update(&mut self) {
        self.prune_rumbles();
        self.turbo_frame = (self.turbo_frame + 1) % 6;

        /*
         * First process gilrs events.
         *
         * We collect them into a Vec so that we don't keep a
         * mutable borrow of gilrs while modifying self.
         */
        let mut events: Vec<Event> = Vec::new();

        if let Some(gilrs) = self.gilrs.as_mut() {
            while let Some(event) = gilrs.next_event() {
                events.push(event);
            }
        }

        for event in events {
            let id = event.id;

            match event.event {
                EventType::Connected => {
                    self.handle_connected(id);
                }

                EventType::Disconnected => {
                    self.handle_disconnected(id);
                }

                EventType::ButtonPressed(button, _) => {
                    if Some(id) == self.gamepad_id {
                        self.handle_button(button, true);
                    }
                }

                EventType::ButtonReleased(button, _) => {
                    if Some(id) == self.gamepad_id {
                        self.handle_button(button, false);
                    }
                }

                EventType::AxisChanged(axis, value, _) => {
                    if Some(id) == self.gamepad_id {
                        self.handle_axis(axis, value);
                    }
                }

                _ => {}
            }
        }

        /*
         * -----------------------------------------------------
         * Poll the current controller state.
         *
         * This is important because some controllers/platforms
         * don't reliably deliver every button transition as an
         * event.
         * -----------------------------------------------------
         */
        self.poll_gamepad_state();
    }

    /*
     * ---------------------------------------------------------
     * Poll current gamepad state
     * ---------------------------------------------------------
     */
    fn poll_gamepad_state(&mut self) {
        let id = match self.gamepad_id {
            Some(id) => id,
            None => return,
        };

        /*
         * Read the current gamepad state without holding a
         * mutable borrow of self.
         */
        let state = match self.gilrs.as_ref() {
            Some(gilrs) => {
                let gamepad = gilrs.gamepad(id);

                let a = gamepad.is_pressed(Button::South);

                let b = gamepad.is_pressed(Button::East);

                let select = gamepad.is_pressed(Button::Select);

                let start = gamepad.is_pressed(Button::Start);
                let right_trigger = gamepad.is_pressed(Button::RightTrigger2)
                    || gamepad.is_pressed(Button::RightTrigger);

                let dpad_up = gamepad.is_pressed(Button::DPadUp);

                let dpad_down = gamepad.is_pressed(Button::DPadDown);

                let dpad_left = gamepad.is_pressed(Button::DPadLeft);

                let dpad_right = gamepad.is_pressed(Button::DPadRight);

                let stick_x = gamepad.value(Axis::LeftStickX);

                let stick_y = gamepad.value(Axis::LeftStickY);

                (
                    a,
                    b,
                    select,
                    start,
                    right_trigger,
                    dpad_up,
                    dpad_down,
                    dpad_left,
                    dpad_right,
                    stick_x,
                    stick_y,
                )
            }

            None => return,
        };

        let (
            a,
            b,
            select,
            start,
            right_trigger,
            dpad_up,
            dpad_down,
            dpad_left,
            dpad_right,
            stick_x,
            stick_y,
        ) = state;

        self.set_turbo_trigger(right_trigger);

        /*
         * Buttons.
         */
        self.usb_buttons.a = a;

        self.usb_buttons.b = b;

        self.usb_buttons.select = select;

        self.usb_buttons.start = start;

        /*
         * D-pad buttons.
         *
         * Analog stick is OR'd with the physical D-pad.
         */
        self.usb_buttons.up = dpad_up || stick_y < -AXIS_THRESHOLD;

        self.usb_buttons.down = dpad_down || stick_y > AXIS_THRESHOLD;

        self.usb_buttons.left = dpad_left || stick_x < -AXIS_THRESHOLD;

        self.usb_buttons.right = dpad_right || stick_x > AXIS_THRESHOLD;
    }

    /*
     * ---------------------------------------------------------
     * Keyboard input
     * ---------------------------------------------------------
     *
     * Keyboard state is stored separately from USB state.
     */
    pub fn set_button(&mut self, button: NesButton, pressed: bool) {
        self.keyboard_buttons.set(button, pressed);
    }

    /*
     * ---------------------------------------------------------
     * USB event button handling
     * ---------------------------------------------------------
     */
    fn handle_button(&mut self, button: Button, pressed: bool) {
        if matches!(button, Button::RightTrigger | Button::RightTrigger2) {
            // Poll both trigger variants together in poll_gamepad_state so
            // mappings exposing both cannot produce duplicate toggles.
            return;
        }
        if let Some(nes_button) = map_button(button) {
            self.usb_buttons.set(nes_button, pressed);
        }
    }

    fn set_turbo_trigger(&mut self, pressed: bool) {
        if pressed && !self.right_trigger_pressed {
            self.turbo_enabled = !self.turbo_enabled;
        }
        self.right_trigger_pressed = pressed;
    }

    /*
     * ---------------------------------------------------------
     * USB analog stick event handling
     * ---------------------------------------------------------
     */
    fn handle_axis(&mut self, axis: Axis, value: f32) {
        match axis {
            Axis::LeftStickX => {
                self.usb_buttons.left = value < -AXIS_THRESHOLD;

                self.usb_buttons.right = value > AXIS_THRESHOLD;
            }

            Axis::LeftStickY => {
                self.usb_buttons.up = value < -AXIS_THRESHOLD;

                self.usb_buttons.down = value > AXIS_THRESHOLD;
            }

            _ => {}
        }
    }

    /*
     * ---------------------------------------------------------
     * Controller connected
     * ---------------------------------------------------------
     */
    fn handle_connected(&mut self, id: GamepadId) {
        if self.gamepad_id.is_some() {
            return;
        }

        self.gamepad_id = Some(id);

        self.usb_buttons = NesButtons::default();

        if let Some(gilrs) = self.gilrs.as_ref() {
            let gamepad = gilrs.gamepad(id);

            println!("Controller connected: {}", gamepad.name());

            println!("  ID: {:?}", id);
        }
    }

    /*
     * ---------------------------------------------------------
     * Controller disconnected
     * ---------------------------------------------------------
     */
    fn handle_disconnected(&mut self, id: GamepadId) {
        if Some(id) != self.gamepad_id {
            return;
        }

        println!("Controller disconnected.");

        self.gamepad_id = None;

        self.usb_buttons = NesButtons::default();
    }

    /*
     * ---------------------------------------------------------
     * Combined controller state
     * ---------------------------------------------------------
     *
     * A button is considered pressed if either:
     *
     *     USB controller = pressed
     *
     * OR
     *
     *     Keyboard = pressed
     * ---------------------------------------------------------
     */
    pub fn buttons(&self) -> NesButtons {
        let turbo_pulse = !self.turbo_enabled || self.turbo_frame < 3;
        NesButtons {
            // Deliberately swap USB A/B for the preferred default gamepad layout.
            a: (self.usb_buttons.b || self.keyboard_buttons.a) && turbo_pulse,

            b: (self.usb_buttons.a || self.keyboard_buttons.b) && turbo_pulse,

            select: self.usb_buttons.select || self.keyboard_buttons.select,

            start: self.usb_buttons.start || self.keyboard_buttons.start,

            up: self.usb_buttons.up || self.keyboard_buttons.up,

            down: self.usb_buttons.down || self.keyboard_buttons.down,

            left: self.usb_buttons.left || self.keyboard_buttons.left,

            right: self.usb_buttons.right || self.keyboard_buttons.right,
        }
    }

    pub fn turbo_enabled(&self) -> bool {
        self.turbo_enabled
    }

    #[allow(dead_code)] // Useful to front ends that display controller status.
    pub fn is_connected(&self) -> bool {
        self.gamepad_id.is_some()
    }

    #[allow(dead_code)] // Useful to front ends that display controller status.
    pub fn controller_name(&self) -> Option<String> {
        let id = self.gamepad_id?;

        let gilrs = self.gilrs.as_ref()?;

        Some(gilrs.gamepad(id).name().to_string())
    }

    /*
     * ---------------------------------------------------------
     * Force feedback (rumble)
     * ---------------------------------------------------------
     *
     * Used by external hooks (e.g. a scripting layer watching
     * game RAM) to trigger haptic feedback on the connected
     * gamepad. `strength` is 0-65535 (gilrs' own scale); `duration_ms`
     * is how long the effect plays. Silently does nothing if no
     * gamepad, or a gamepad with no FF support, is connected.
     */
    #[allow(dead_code)] // Haptic hook for front ends and future game integrations.
    pub fn rumble(&mut self, strength: u16, duration_ms: u64) {
        let (Some(gilrs), Some(id)) = (self.gilrs.as_mut(), self.gamepad_id) else {
            return;
        };

        if !gilrs.gamepad(id).is_ff_supported() {
            return;
        }

        let duration = Ticks::from_ms(duration_ms as u32);

        let effect = EffectBuilder::new()
            .add_effect(BaseEffect {
                kind: BaseEffectType::Strong {
                    magnitude: strength,
                },
                scheduling: Replay {
                    play_for: duration,
                    ..Default::default()
                },
                envelope: Default::default(),
            })
            .add_effect(BaseEffect {
                kind: BaseEffectType::Weak {
                    magnitude: strength,
                },
                scheduling: Replay {
                    play_for: duration,
                    ..Default::default()
                },
                envelope: Default::default(),
            })
            .gamepads(&[id])
            .finish(gilrs);

        match effect {
            Ok(effect) => {
                if let Err(error) = effect.play() {
                    eprintln!("Warning: failed to play rumble effect: {}", error);

                    // Don't keep a handle that never played.
                    return;
                }

                // gilrs stops the effect as soon as its handle is
                // dropped, so it must be kept alive here until
                // `duration_ms` has elapsed - pruned in `update()`.
                self.active_rumbles.push((
                    effect,
                    Instant::now(),
                    Duration::from_millis(duration_ms),
                ));
            }

            Err(error) => {
                eprintln!("Warning: failed to build rumble effect: {}", error);
            }
        }
    }

    /*
     * Drops any rumble effect handles whose playback duration has
     * elapsed. Cheap to call every frame - called from `update()`.
     */
    fn prune_rumbles(&mut self) {
        let now = Instant::now();

        self.active_rumbles
            .retain(|(_, started, duration)| now.duration_since(*started) < *duration);
    }

    /*
     * ---------------------------------------------------------
     * NES controller protocol
     * ---------------------------------------------------------
     */

    fn latch(&mut self) {
        self.shift_register = self.buttons().to_byte();
    }

    /*
     * CPU writes to $4016.
     */
    pub fn write(&mut self, value: u8) {
        let new_strobe = value & 1 != 0;

        /*
         * Latch on 1 -> 0.
         */
        if self.strobe && !new_strobe {
            self.latch();
        }

        self.strobe = new_strobe;

        /*
         * While strobe is high, continually expose the
         * current A button.
         */
        if self.strobe {
            self.latch();
        }
    }

    /*
     * CPU reads from $4016.
     */
    pub fn read(&mut self) -> u8 {
        /*
         * While strobe is high, return A.
         */
        if self.strobe {
            return 0x40 | if self.buttons().a { 1 } else { 0 };
        }

        /*
         * Return next serial bit.
         */
        let value = 0x40 | (self.shift_register & 1);

        self.shift_register >>= 1;

        /*
         * NES controllers return 1 after the eight
         * controller bits have been consumed.
         */
        self.shift_register |= 0x80;

        value
    }
}
