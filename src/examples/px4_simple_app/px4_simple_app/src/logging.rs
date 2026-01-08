use core::fmt::Write;
use log::{Metadata, Record}; // Artık format! makrosunu kullanabilirsiniz

unsafe extern "C" {
    fn px4_log_modulename(level: i32, module: *const u8, fmt: *const u8, ...);
    fn px4_log_raw(level: i32, fmt: *const u8, ...);
}

#[doc(hidden)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    Panic = 4,
}

#[doc(hidden)]
pub fn log_raw(level: LogLevel, message: &str) {
    unsafe {
        px4_log_raw(
            level as i32,
            "%.*s\0".as_ptr(),
            message.len() as i32,
            message.as_ptr(),
        );
    }
}

struct Px4Logger<const T: usize> {}

impl<const T: usize> log::Log for Px4Logger<T> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = match record.level() {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            log::Level::Info => LogLevel::Info,
            log::Level::Debug => LogLevel::Debug,
            log::Level::Trace => LogLevel::Debug,
        };

        let mut s = heapless::String::<T>::new();
        match write!(s, "{}\0{}\0", record.target(), record.args()) {
            Ok(_) => {
                let (module, message) = s.as_bytes().split_at(record.target().len() + 1);

                unsafe {
                    px4_log_modulename(
                        level as i32,
                        module.as_ptr(),
                        "%s\0".as_ptr(),
                        message.as_ptr(),
                    );
                }
            }

            Err(hata) => unsafe {
                px4_log_modulename(
                    level as i32,
                    "log hata\0".as_ptr(),
                    "%s\0".as_ptr(),
                    "logger boyut yetmedi\0".as_ptr(),
                );
            },
        };
    }

    fn flush(&self) {}
}

static LOGGER: Px4Logger<128> = Px4Logger {};

pub unsafe fn init() {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
        //core::panic::set_hook(Box::new(move |info: &std::panic::PanicInfo| {
        //    let payload: &str = if let Some(s) = info.payload().downcast_ref::<&'static str>() {
        //        s
        //    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        //        &s
        //    } else {
        //        "[unknown]"
        //    };
        //    let mut message = String::new();
        //    let thread = std::thread::current();
        //    if let Some(name) = thread.name() {
        //        write!(message, "thread '{}' ", name).unwrap();
        //    }
        //    write!(message, "panicked at '{}'", payload).unwrap();
        //    if let Some(loc) = info.location() {
        //        write!(message, ", {}", loc).unwrap();
        //    }
        //    message.push('\0');
        //    px4_log_modulename(
        //        LogLevel::Panic as i32,
        //        modulename.as_ptr(),
        //        "%s\0".as_ptr(),
        //        message.as_ptr(),
        //    );
        //}));
    }
}
