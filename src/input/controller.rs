use gilrs::{
    Axis,
    Button,
    Event,
    EventType,
    GamepadId,
    Gilrs,
};

use super::mapping::{
    map_button,
    NesButton,
    NesButtons,
};

const AXIS_THRESHOLD: f32 = 0.5;

pub struct Controller {
    gilrs: Option<Gilrs>,
    gamepad_id: Option<GamepadId>,

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
                eprintln!(
                    "Warning: gamepad support unavailable: {}",
                    error
                );

                None
            }
        };

        let gamepad_id = gilrs
            .as_ref()
            .and_then(|gilrs| {
                gilrs
                    .gamepads()
                    .next()
                    .map(|(id, _)| id)
            });

        let controller = Self {
            gilrs,
            gamepad_id,
            usb_buttons: NesButtons::default(),
            keyboard_buttons: NesButtons::default(),
            strobe: false,
            shift_register: 0,
        };

        if let Some(id) =
            controller.gamepad_id
        {
            if let Some(gilrs) =
                controller.gilrs.as_ref()
            {
                let gamepad =
                    gilrs.gamepad(id);

                println!(
                    "Controller connected: {}",
                    gamepad.name()
                );

                println!(
                    "  ID: {:?}",
                    id
                );
            }
        } else {
            println!(
                "No USB controller detected."
            );
        }

        controller
    }

    /*
     * ---------------------------------------------------------
     * Update USB controller
     * ---------------------------------------------------------
     */
    pub fn update(&mut self) {
        /*
         * First process gilrs events.
         *
         * We collect them into a Vec so that we don't keep a
         * mutable borrow of gilrs while modifying self.
         */
        let mut events: Vec<Event> =
            Vec::new();

        if let Some(gilrs) =
            self.gilrs.as_mut()
        {
            while let Some(event) =
                gilrs.next_event()
            {
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

                EventType::ButtonPressed(
                    button,
                    _,
                ) => {
                    if Some(id) ==
                        self.gamepad_id
                    {
                        self.handle_button(
                            button,
                            true,
                        );
                    }
                }

                EventType::ButtonReleased(
                    button,
                    _,
                ) => {
                    if Some(id) ==
                        self.gamepad_id
                    {
                        self.handle_button(
                            button,
                            false,
                        );
                    }
                }

                EventType::AxisChanged(
                    axis,
                    value,
                    _,
                ) => {
                    if Some(id) ==
                        self.gamepad_id
                    {
                        self.handle_axis(
                            axis,
                            value,
                        );
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
        let id =
            match self.gamepad_id {
                Some(id) => id,
                None => return,
            };

        /*
         * Read the current gamepad state without holding a
         * mutable borrow of self.
         */
        let state =
            match self.gilrs.as_ref() {
                Some(gilrs) => {
                    let gamepad =
                        gilrs.gamepad(id);

                    let a =
                        gamepad.is_pressed(
                            Button::South
                        );

                    let b =
                        gamepad.is_pressed(
                            Button::East
                        );

                    let select =
                        gamepad.is_pressed(
                            Button::Select
                        );

                    let start =
                        gamepad.is_pressed(
                            Button::Start
                        );

                    let dpad_up =
                        gamepad.is_pressed(
                            Button::DPadUp
                        );

                    let dpad_down =
                        gamepad.is_pressed(
                            Button::DPadDown
                        );

                    let dpad_left =
                        gamepad.is_pressed(
                            Button::DPadLeft
                        );

                    let dpad_right =
                        gamepad.is_pressed(
                            Button::DPadRight
                        );

                    let stick_x =
                        gamepad.value(
                            Axis::LeftStickX
                        );

                    let stick_y =
                        gamepad.value(
                            Axis::LeftStickY
                        );

                    (
                        a,
                        b,
                        select,
                        start,
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
            dpad_up,
            dpad_down,
            dpad_left,
            dpad_right,
            stick_x,
            stick_y,
        ) = state;

        /*
         * Buttons.
         */
        self.usb_buttons.a =
            a;

        self.usb_buttons.b =
            b;

        self.usb_buttons.select =
            select;

        self.usb_buttons.start =
            start;

        /*
         * D-pad buttons.
         *
         * Analog stick is OR'd with the physical D-pad.
         */
        self.usb_buttons.up =
            dpad_up
            || stick_y < -AXIS_THRESHOLD;

        self.usb_buttons.down =
            dpad_down
            || stick_y > AXIS_THRESHOLD;

        self.usb_buttons.left =
            dpad_left
            || stick_x < -AXIS_THRESHOLD;

        self.usb_buttons.right =
            dpad_right
            || stick_x > AXIS_THRESHOLD;
    }

    /*
     * ---------------------------------------------------------
     * Keyboard input
     * ---------------------------------------------------------
     *
     * Keyboard state is stored separately from USB state.
     */
    pub fn set_button(
        &mut self,
        button: NesButton,
        pressed: bool,
    ) {
        self.keyboard_buttons.set(
            button,
            pressed,
        );
    }

    /*
     * ---------------------------------------------------------
     * USB event button handling
     * ---------------------------------------------------------
     */
    fn handle_button(
        &mut self,
        button: Button,
        pressed: bool,
    ) {
        if let Some(nes_button) =
            map_button(button)
        {
            self.usb_buttons.set(
                nes_button,
                pressed,
            );
        }
    }

    /*
     * ---------------------------------------------------------
     * USB analog stick event handling
     * ---------------------------------------------------------
     */
    fn handle_axis(
        &mut self,
        axis: Axis,
        value: f32,
    ) {
        match axis {
            Axis::LeftStickX => {
                self.usb_buttons.left =
                    value < -AXIS_THRESHOLD;

                self.usb_buttons.right =
                    value > AXIS_THRESHOLD;
            }

            Axis::LeftStickY => {
                self.usb_buttons.up =
                    value < -AXIS_THRESHOLD;

                self.usb_buttons.down =
                    value > AXIS_THRESHOLD;
            }

            _ => {}
        }
    }

    /*
     * ---------------------------------------------------------
     * Controller connected
     * ---------------------------------------------------------
     */
    fn handle_connected(
        &mut self,
        id: GamepadId,
    ) {
        if self.gamepad_id.is_some() {
            return;
        }

        self.gamepad_id =
            Some(id);

        self.usb_buttons =
            NesButtons::default();

        if let Some(gilrs) =
            self.gilrs.as_ref()
        {
            let gamepad =
                gilrs.gamepad(id);

            println!(
                "Controller connected: {}",
                gamepad.name()
            );

            println!(
                "  ID: {:?}",
                id
            );
        }
    }

    /*
     * ---------------------------------------------------------
     * Controller disconnected
     * ---------------------------------------------------------
     */
    fn handle_disconnected(
        &mut self,
        id: GamepadId,
    ) {
        if Some(id) !=
            self.gamepad_id
        {
            return;
        }

        println!(
            "Controller disconnected."
        );

        self.gamepad_id =
            None;

        self.usb_buttons =
            NesButtons::default();
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
    pub fn buttons(
        &self,
    ) -> NesButtons {
        NesButtons {
            a: self.usb_buttons.b
                || self.keyboard_buttons.a,

            b: self.usb_buttons.a
                || self.keyboard_buttons.b,

            select: self.usb_buttons.select
                || self.keyboard_buttons.select,

            start: self.usb_buttons.start
                || self.keyboard_buttons.start,

            up: self.usb_buttons.up
                || self.keyboard_buttons.up,

            down: self.usb_buttons.down
                || self.keyboard_buttons.down,

            left: self.usb_buttons.left
                || self.keyboard_buttons.left,

            right: self.usb_buttons.right
                || self.keyboard_buttons.right,
        }
    }

    pub fn is_connected(
        &self,
    ) -> bool {
        self.gamepad_id.is_some()
    }

    pub fn controller_name(
        &self,
    ) -> Option<String> {
        let id =
            self.gamepad_id?;

        let gilrs =
            self.gilrs.as_ref()?;

        Some(
            gilrs
                .gamepad(id)
                .name()
                .to_string()
        )
    }

    /*
     * ---------------------------------------------------------
     * NES controller protocol
     * ---------------------------------------------------------
     */

    fn latch(&mut self) {
        self.shift_register =
            self.buttons().to_byte();
    }

    /*
     * CPU writes to $4016.
     */
    pub fn write(
        &mut self,
        value: u8,
    ) {
        let new_strobe =
            value & 1 != 0;

        /*
         * Latch on 1 -> 0.
         */
        if self.strobe
            && !new_strobe
        {
            self.latch();
        }

        self.strobe =
            new_strobe;

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
    pub fn read(
        &mut self,
    ) -> u8 {
        /*
         * While strobe is high, return A.
         */
        if self.strobe {
            return if self.buttons().a {
                1
            } else {
                0
            };
        }

        /*
         * Return next serial bit.
         */
        let value =
            self.shift_register & 1;

        self.shift_register >>= 1;

        /*
         * NES controllers return 1 after the eight
         * controller bits have been consumed.
         */
        self.shift_register |= 0x80;

        value
    }
}
