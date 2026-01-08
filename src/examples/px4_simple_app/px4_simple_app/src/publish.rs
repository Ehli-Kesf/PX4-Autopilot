use core::marker::PhantomData;

use pictorus_px4::message_impls::Topic;

use crate::subscribe::OrbError;

pub struct Publisher<T: Topic> {
    handle: *mut core::ffi::c_void, // orb_advert_t
    _marker: PhantomData<T>,
}

impl<T: Topic> Publisher<T> {
    pub fn new(initial_data: &T::Message) -> Result<Self, OrbError> {
        // İlk advertise işlemi
        let handle = unsafe {
            px4_msgs_sys::orb::orb_advertise(
                T::metadata(),
                initial_data as *const T::Message as *const core::ffi::c_void,
            )
        };

        if handle.is_null() {
            Err(OrbError::PublishFailed)
        } else {
            Ok(Self {
                handle,
                _marker: core::marker::PhantomData,
            })
        }
    }

    pub fn publish(&mut self, data: &T::Message) -> Result<(), OrbError> {
        let ret = unsafe {
            px4_msgs_sys::orb::orb_publish(
                T::metadata(),
                self.handle,
                data as *const T::Message as *const core::ffi::c_void,
            )
        };

        if ret == 0 {
            Ok(())
        } else {
            Err(OrbError::PublishFailed)
        }
    }
}
