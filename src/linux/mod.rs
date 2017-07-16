use std::time::Duration;
use std::thread;

mod devicemap;
mod hidraw;
pub mod device;
pub mod monitor;

use runloop::RunLoop;
use util::{io_err, OnceCallback};
pub use self::device::Device;
use self::devicemap::DeviceMap;
pub use self::monitor::Monitor;

pub struct PlatformManager {
    /// Handle to the thread loop.
    thread: Option<RunLoop>,
}

impl PlatformManager {
    pub fn new() -> Self {
        Self { thread: None }
    }

    pub fn register(
        &mut self,
        timeout: u64,
        challenge: Vec<u8>,
        application: Vec<u8>,
        callback: OnceCallback,
    ) {
        // Abort any prior register/sign calls.
        self.cancel();

        let cbc = callback.clone();

        let thread = RunLoop::new(
            move |alive| {
                let mut devices = DeviceMap::new();
                let monitor = try_or!(Monitor::new(), |e| { callback.call(Err(e)); });

                // TODO check if dlopen() failed?

                while alive() {
                    // Add/remove devices.
                    for event in monitor.events() {
                        devices.process_event(event);
                    }

                    // Try to register each device.
                    for device in devices.values_mut() {
                        if let Ok(bytes) = super::u2f_register(device, &challenge, &application) {
                            callback.call(Ok(bytes));
                            return;
                        }
                    }

                    // Wait a little before trying again.
                    thread::sleep(Duration::from_millis(100));
                }

                callback.call(Err(io_err("cancelled or timed out")));
            },
            timeout,
        );

        self.thread = Some(try_or!(
            thread,
            |_| cbc.call(Err(io_err("couldn't create runloop")))
        ));
    }

    // TODO merge with register()?
    pub fn sign(
        &mut self,
        timeout: u64,
        challenge: Vec<u8>,
        application: Vec<u8>,
        key_handle: Vec<u8>,
        callback: OnceCallback,
    ) {
        // Abort any prior register/sign calls.
        self.cancel();

        let cbc = callback.clone();

        let thread = RunLoop::new(
            move |alive| {
                let mut devices = DeviceMap::new();
                let monitor = try_or!(Monitor::new(), |e| { callback.call(Err(e)); });

                while alive() {
                    // Add/remove devices.
                    for event in monitor.events() {
                        devices.process_event(event);
                    }

                    // Try signing with each device.
                    for device in devices.values_mut() {
                        // Check if they key handle belongs to the current device.
//                        let is_valid = match super::u2f_is_keyhandle_valid(
//                            device,
//                            &challenge,
//                            &application,
//                            &key_handle,
//                        ) {
//                            Ok(valid) => valid,
//                            Err(_) => continue,
//                        };
//
//                        if is_valid {
                            // If yes, try to sign.
                            if let Ok(bytes) = super::u2f_sign(
                                device,
                                &challenge,
                                &application,
                                &key_handle,
                            )
                            {
                                callback.call(Ok(bytes));
                                return;
                            }
//                        } else {
//                            // If no, keep registering and blinking with bogus data
//                            let blank = vec![0u8; PARAMETER_SIZE];
//                            if let Ok(_) = super::u2f_register(device, &blank, &blank) {
//                                callback.call(Err(io_err("invalid key")));
//                                return;
//                            }
//                        }
                    }

                    // Wait a little before trying again.
                    thread::sleep(Duration::from_millis(100));
                }

                callback.call(Err(io_err("cancelled or timed out")));
            },
            timeout,
        );

        self.thread = Some(try_or!(
            thread,
            |_| cbc.call(Err(io_err("couldn't create runloop")))
        ));
    }

    pub fn send_apdu(&mut self, timeout: u64, cla: u8,  cmd: u8, p1: u8, data: Vec<u8>, callback: OnceCallback) {
        self.cancel();

        let cbc = callback.clone();

        let thread = RunLoop::new(
            move |alive| {
                let mut devices = DeviceMap::new();
                let monitor = try_or!(Monitor::new(), |e| { callback.call(Err(e)); });

                while alive() {
                    // Add/remove devices.
                    for event in monitor.events() {
                        devices.process_event(event);
                    }

                    for device in devices.values_mut() {
                        println!(">> DEBUG: {:?}", device);
                        if let Ok(bytes) = super::send_apdu(device, cla, cmd, p1, &data) {
                            callback.call(Ok(bytes));
                            return;
                        }
                    }

                    thread::sleep(Duration::from_millis(100));
                }

                callback.call(Err(io_err("cancelled or timed out")));
            },
            timeout,
        );

        self.thread = Some(try_or!(
            thread,
            |_| cbc.call(Err(io_err("couldn't create runloop")))
        ));
    }

    // This blocks.
    pub fn cancel(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.cancel();
        }
    }
}
