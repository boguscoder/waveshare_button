#![no_std]

//! Spawns an embassy task that polls the active-low button and announces a
//! debounced [`ButtonEvent`] on [`BUTTON_EVENTS`]: exactly one event per
//! press, on release — [`ButtonEvent::ShortPress`] for holds under 1 s,
//! [`ButtonEvent::LongPress`] for longer ones.

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Ticker};
use esp_hal::gpio::{AnyPin, Input, InputConfig, Pull};

#[derive(Clone, Copy)]
pub enum ButtonEvent {
    /// Press released after less than 1 s.
    ShortPress,
    /// Press held for 1 s or more.
    LongPress,
}

pub static BUTTON_EVENTS: Signal<CriticalSectionRawMutex, ButtonEvent> = Signal::new();

const POLL_INTERVAL_MS: u64 = 20;
const DEBOUNCE_MS: u64 = 30;
const LONG_PRESS_MS: u64 = 1000;

#[embassy_executor::task]
pub async fn button_task(button_pin: AnyPin<'static>) {
    let button = Input::new(button_pin, InputConfig::default().with_pull(Pull::Up));
    let mut ticker = Ticker::every(Duration::from_millis(POLL_INTERVAL_MS));
    let mut press_start: Option<Instant> = None;

    loop {
        let is_pressed = button.is_low();

        match (is_pressed, press_start) {
            // Button just pressed
            (true, None) => {
                press_start = Some(Instant::now());
            }
            // Button released after being pressed
            (false, Some(start)) => {
                let duration = start.elapsed().as_millis();
                if duration >= LONG_PRESS_MS {
                    BUTTON_EVENTS.signal(ButtonEvent::LongPress);
                } else if duration >= DEBOUNCE_MS {
                    BUTTON_EVENTS.signal(ButtonEvent::ShortPress);
                }
                press_start = None;
            }
            // Still pressed or still released
            _ => {}
        }

        ticker.next().await;
    }
}
