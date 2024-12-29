use std::ptr::{null, null_mut};

use core_foundation::{
    base::TCFType,
    dictionary::CFDictionaryRef,
    propertylist::kCFPropertyListXMLFormat_v1_0,
    string::{CFString, CFStringRef},
};
use scopefn::Run;

use crate::{
    cfstr,
    ffi::{
        AMDServiceConnectionInvalidate, AMDServiceConnectionReceiveMessage,
        AMDServiceConnectionRef, AMDServiceConnectionSendMessage, AMDeviceConnect,
        AMDeviceCopyDeviceIdentifier, AMDeviceCopyValue, AMDeviceDisconnect,
        AMDeviceGetInterfaceType, AMDeviceIsPaired, AMDevicePair, AMDeviceRef,
        AMDeviceSecureStartService, AMDeviceStartSession, AMDeviceStopSession,
        AMDeviceValidatePairing, InterfaceType,
    },
};

pub struct ServiceConnection(pub AMDServiceConnectionRef);

unsafe impl Send for ServiceConnection {}
unsafe impl Sync for ServiceConnection {}

impl ServiceConnection {
    pub fn start(device: AMDeviceRef, service_name: &str) -> Self {
        unsafe {
            let service_name = cfstr!(service_name);
            let service_ptr: AMDServiceConnectionRef = null_mut();

            let result = AMDeviceSecureStartService(
                device,
                service_name.as_concrete_TypeRef(),
                null_mut(),
                &service_ptr,
            );

            if result != 0 {
                panic!("couldn't start service {}", result);
            }

            ServiceConnection(service_ptr)
        }
    }

    pub fn send(&self, message: CFDictionaryRef) -> Result<(), i32> {
        match unsafe {
            AMDServiceConnectionSendMessage(self.0, message, kCFPropertyListXMLFormat_v1_0)
        } {
            0 => Ok(()),
            e => Err(e),
        }
    }

    pub fn receive(&self) -> Result<CFDictionaryRef, i32> {
        unsafe {
            let response: CFDictionaryRef = null_mut();
            AMDServiceConnectionReceiveMessage(self.0, &response, null(), null(), null(), null())
                .run(|res| match res {
                    0 => Ok(response),
                    _ => Err(res),
                })
        }
    }
}

impl Drop for ServiceConnection {
    fn drop(&mut self) {
        unsafe { AMDServiceConnectionInvalidate(self.0) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Device {
    pub device: AMDeviceRef,
    pub udid: String,
    pub interface_type: InterfaceType,
    // name: String,
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("couldn't connect: {0}")]
    Connect(i32),

    #[error("pairing failed: {0}")]
    Pair(i32),

    #[error("pairing validation failed: {0}")]
    Validate(i32),

    #[error("session failed: {0}")]
    Session(i32),
}

impl Device {
    pub fn new(device: AMDeviceRef) -> Self {
        let udid =
            unsafe { CFString::wrap_under_create_rule(AMDeviceCopyDeviceIdentifier(device)) }
                .to_string();
        // let name = unsafe {
        //     CFString::wrap_under_create_rule(AMDeviceCopyValue(
        //         device,
        //         null(),
        //         cfstr!("DeviceName").as_concrete_TypeRef(),
        //     ) as CFStringRef)
        // }
        // .to_string();
        Self {
            device,
            udid,
            interface_type: unsafe { AMDeviceGetInterfaceType(device) },
            // name: "".to_string(),
        }
    }

    pub fn name(&self) -> String {
        let name = unsafe {
            AMDeviceCopyValue(
                self.device,
                null(),
                cfstr!("DeviceName").as_concrete_TypeRef(),
            )
        } as CFStringRef;

        unsafe { CFString::wrap_under_create_rule(name) }.to_string()
    }

    pub fn interface_type(&mut self) -> InterfaceType {
        let interface_type = unsafe { AMDeviceGetInterfaceType(self.device) };

        self.interface_type = interface_type;

        interface_type
    }

    pub fn connect(&self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceConnect(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Connect(err)),
        }
    }

    pub fn disconnect(&self) {
        unsafe { AMDeviceStopSession(self.device) };
        unsafe { AMDeviceDisconnect(self.device) };
    }

    pub fn is_paired(&self) -> bool {
        unsafe { AMDeviceIsPaired(self.device) == 1 }
    }

    pub fn pair(&self) -> Result<(), DeviceError> {
        match unsafe { AMDevicePair(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Pair(err)),
        }
    }

    pub fn validate_pairing(&self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceValidatePairing(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Validate(err)),
        }
    }

    pub fn start_session(&self) -> Result<(), DeviceError> {
        match unsafe { AMDeviceStartSession(self.device) } {
            0 => Ok(()),
            err => Err(DeviceError::Session(err)),
        }
    }

    pub fn stop_session(&self) {
        unsafe {
            AMDeviceStopSession(self.device);
        }
    }

    pub fn prepare_device(&self) -> Result<(), DeviceError> {
        self.connect()?;
        if !self.is_paired() {
            self.pair()?;
        }
        self.validate_pairing()?;
        self.start_session()?;
        Ok(())
    }

    pub fn start_service(&self, service_name: &str) -> ServiceConnection {
        ServiceConnection::start(self.device, service_name)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.stop_session();
        self.disconnect();
    }
}

// pub trait CFDictionaryExt<K, V> {
//     fn get_by_str(&self, key: &str) -> ItemRef<'_, V>;
// }

// impl<K, V> CFDictionaryExt<K, V> for CFDictionary<K, V>
// where
//     CFString: ToVoid<K>,
//     V: FromVoid,
//     // K: ToVoid<CFStringRef>,
//     K: ToVoid<K>,
// {
//     fn get_by_str(&self, key: &str) -> ItemRef<'_, V> {
//         self.get(CFString::new(key))
//     }
// }

// response

// let a = unsafe {
//     CFNumber::wrap_under_get_rule(
//         response
//             .get(cfstr!("Diagnostics"))
//             .get(cfstr!("IORegistry"))
//             .get(cfstr!("Amperage"))
//             .to_void() as CFNumberRef,
//     )
// };

// let v = unsafe {
//     CFNumber::wrap_under_get_rule(
//         response
//             .get(cfstr!("Diagnostics"))
//             .get(cfstr!("IORegistry"))
//             .get(cfstr!("Voltage"))
//             .to_void() as CFNumberRef,
//     )
// };

// println!(
//     "power: {}watts",
//     (a.to_i64()
//         .expect("couldn't get amperage")
//         .mul(v.to_i64().unwrap()) as f64)
//         .div(1_000_000.0)
// );

// #[allow(unused_variables)]
// extern "C" fn handle_am_device_notification(
//     target: *const AMDeviceNotificationCallbackInfo,
//     _args: *mut libc::c_void,
// ) {
//     let arc_ref = unsafe { &*(_args as *const RwLock<Vec<&AMDevice>>) };
//     let device = unsafe { &*((*target).device as *const AMDevice) };
// }

// pub fn get_devices(_timeout: f64) -> Vec<&'static AMDevice> {
//     let devices = Box::new(RwLock::new(Vec::<&AMDevice>::new()));
//     let devices_ptr = Box::into_raw(devices);
//     let mut not = MaybeUninit::uninit();
//     unsafe {
//         bridge::AMDeviceNotificationSubscribe(
//             handle_am_device_notification,
//             0,
//             0,
//             devices_ptr as *mut c_void,
//             not.as_mut_ptr(),
//         );
//         // CFRunLoopRunInMode(kCFRunLoopDefaultMode, timeout, false as Boolean);
//         CFRunLoopRun();
//     }

//     let devices = unsafe { Box::from_raw(devices_ptr) };

//     devices.into_inner().unwrap()
// }

// pub fn get_udid(device: AMDeviceRef) -> String {
//     let char_ptr = unsafe {
//         let ns_uuid = AMDeviceCopyDeviceIdentifier(device);
//         let c_str_ptr = CFStringGetCStringPtr(ns_uuid, kCFStringEncodingUTF8);
//         CFRelease(ns_uuid as CFTypeRef);
//         c_str_ptr
//     };
//     let c_str = unsafe { std::ffi::CStr::from_ptr(char_ptr) };
//     String::from(c_str.to_str().unwrap())
// }

// pub unsafe fn disconnect(device: AMDeviceRef) {
//     unsafe {
//         AMDeviceStopSession(device);
//         AMDeviceDisconnect(device);
//     };
// }
