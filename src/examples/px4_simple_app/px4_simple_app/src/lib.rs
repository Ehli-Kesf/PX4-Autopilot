#![no_std]
#![no_main]

use log::logger;
use macros::px4_message;
use publish::Publish;
use subscribe::Subscribe;

/// A message which can be published and/or subscribed to.
///
/// This trait is automatically implemented for all messages imported using
/// `#[px4_message]`.
pub mod c;

pub unsafe trait Message {
    /// Get the metadata of this type of message.
    fn metadata() -> &'static c::Metadata;
}
pub mod logging;
pub mod publish;
pub mod subscribe;

#[px4_message(
    "/home/abdulmelik/dev/ehlikesf_ws/PX4-Autopilot/msg/versioned/VehicleGlobalPosition.msg"
)]
pub struct VehicleGlobalPosition;

#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> f32 {
    logging::log_raw(logging::LogLevel::Info, "Hello world from rust\n");
    if let Ok(sub) = VehicleGlobalPosition::subscribe() {
        for _ in 0..10 {
            if let Ok(res) = sub.get() {
                return res.alt as f32;
            }
        }
        2.0
    } else {
        1.0
    }
    // let sub = VehicleStatus::subscribe().unwrap();
    // let metadata = VehicleAttitude::metadata();
    // a + b
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
