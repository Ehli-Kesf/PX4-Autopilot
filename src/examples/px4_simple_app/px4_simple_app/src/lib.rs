#![no_std]
#![no_main]

// lib.rs veya main.rs başında
// extern crate alloc;

use pictorus_px4::message_impls::VehicleLocalPosition;

use panic_halt as _;

use crate::subscribe::Subscriber;

// #[global_allocator]
// static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

pub mod logging;
pub mod publish;
pub mod subscribe;

// 1. Dışarıdaki C fonksiyonunu tanımlıyoruz
unsafe extern "C" {
    // PX4'ün sistem sleep fonksiyonu (mikrosaniye cinsinden)
    fn px4_usleep(usec: core::ffi::c_uint);
}

// 2. Kullanımı kolaylaştıran güvenli bir wrapper yazıyoruz
pub fn sleep_ms(ms: u32) {
    unsafe {
        // Milisaniyeyi mikrosaniyeye çevirip C fonksiyonunu çağırıyoruz
        px4_usleep(ms * 1000);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> isize {
    // const HEAP_SIZE: usize = 10_240;
    // static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    // unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    unsafe {
        logging::init();
    };
    log::info!("merhaba dunya form rust");

    let mut sub = Subscriber::<VehicleLocalPosition>::new().unwrap();
    for _ in 0..1000 {
        if let Ok(msg) = sub.next_timeout(1) {
            log::info!("local pos geldi {}", msg.timestamp);
        } else {
            log::info!("gelmedi");
        };
    }
    0
}

// #[panic_handler]
// fn panic(_info: &core::panic::PanicInfo) -> ! {
//     loop {}
// }
