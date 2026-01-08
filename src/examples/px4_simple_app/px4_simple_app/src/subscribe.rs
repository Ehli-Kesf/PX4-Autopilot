#[cfg(not(target_os = "none"))]
use core::ffi::c_ushort;

#[cfg(target_os = "none")] // Genelde gömülü sistemler "none" olarak geçer
use core::ffi::c_uint;

use core::ffi::{c_int, c_void};

use pictorus_px4::message_impls::Topic;
use px4_msgs_sys::orb::orb_copy;

// PX4'teki typedef'lere karşılık gelen tipler
// Eğer C tarafında short ise burayı u16 yapın, ama genelde u32'dir.
#[cfg(target_os = "none")] // Genelde gömülü sistemler "none" olarak geçer
pub type Px4PollEvent = c_uint;
#[cfg(not(target_os = "none"))] // SITL (Linux/Mac)
pub type Px4PollEvent = c_ushort;

const POLLIN: Px4PollEvent = 0x01;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PollFd {
    // --- POSIX benzeri kısım ---
    pub fd: c_int,
    pub events: Px4PollEvent,  // POLLIN vb.
    pub revents: Px4PollEvent, // Çıktı olayları

    // --- PX4/NuttX özel kısım ---
    // Rust tarafında bu semaforu yönetmeyeceğimiz için
    // opaque pointer (*mut c_void) olarak tutmak güvenlidir.
    pub sem: *mut c_void,       // px4_sem_t *sem
    pub priv_data: *mut c_void, // void *priv (priv rezerve kelime olabilir diye değiştirdik)
}

unsafe extern "C" {
    fn rust_px4_poll(fds: *mut PollFd, nfds: core::ffi::c_uint, timeout: core::ffi::c_int)
    -> c_int;
}

pub struct Subscriber<T: Topic> {
    fd: core::ffi::c_int,
    _marker: core::marker::PhantomData<T>,
}

// Hata tiplerimiz
#[derive(Debug)]
pub enum OrbError {
    SubscribeFailed,
    PublishFailed,
    CopyFailed,
    PollError(core::ffi::c_int),
    Timeout,
}

impl<T: Topic> Subscriber<T> {
    // Yeni bir abonelik başlatır
    pub fn new() -> Result<Self, OrbError> {
        let fd = unsafe { px4_msgs_sys::orb::orb_subscribe(T::metadata()) };
        if fd < 0 {
            return Err(OrbError::SubscribeFailed);
        }
        Ok(Self {
            fd,
            _marker: core::marker::PhantomData,
        })
    }

    // Blocking (Bloklayan) Okuma Fonksiyonu
    // timeout_ms: Bekleme süresi. Veri gelirse döner, gelmezse Timeout hatası verir.
    pub fn next_timeout(&mut self, timeout_ms: i32) -> Result<T::Message, OrbError> {
        let mut pfd = PollFd {
            fd: self.fd,
            events: POLLIN,
            revents: 0,
            ..Default::default()
        };

        // px4_poll ile işletim sistemine "beni beklet" diyoruz
        let ret = unsafe { rust_px4_poll(&mut pfd, 1, timeout_ms) };

        // let ret = 0;
        if ret == 0 {
            return Err(OrbError::Timeout);
        } else if ret < 0 {
            return Err(OrbError::PollError(ret));
        }

        // Veri geldiyse kopyalayalım
        if (pfd.revents & POLLIN) != 0 {
            let mut data = core::mem::MaybeUninit::<T::Message>::uninit();
            let copy_ret = unsafe {
                orb_copy(
                    T::metadata(),
                    self.fd,
                    data.as_mut_ptr() as *mut core::ffi::c_void,
                )
            };

            if copy_ret == 0 {
                Ok(unsafe { data.assume_init() })
            } else {
                Err(OrbError::CopyFailed)
            }
        } else {
            Err(OrbError::Timeout) // Poll döndü ama event yoksa (nadir durum)
        }
    }

    // Sonsuza kadar bekleyen versiyon
    pub fn next(&mut self) -> Result<T::Message, OrbError> {
        // -1 genelde sonsuz bekleme demektir (PX4 sürümüne göre değişebilir, genelde güvenli bir döngüde kullanmak iyidir)
        self.next_timeout(1000)
    }
}
