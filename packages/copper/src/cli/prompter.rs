use std::io;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;

use cu::pre::*;

static READ_STDIN: LazyLock<Mutex<Receiver<io::Result<String>>>> = LazyLock::new(|| {
    let (send, recv) = mpsc::sync_channel(0);

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut buf = String::new();
        loop {
            buf.clear();
            let result = stdin.read_line(&mut buf);
            let to_send = match result {
                Ok(0) => break, // EOF
                Ok(_) => Ok(buf.clone()),
                Err(e) => Err(e),
            };
            let _ = send.send(to_send);
        }
    });

    Mutex::new(recv)
});

/// Read a line of plaintext from stdin.
///
/// Uses a global background thread to read from stdin. Polls every 200ms
/// to check if the ctrlc signal has been triggered.
///
/// Returns `Ok(None)` if ctrlc is triggered or EOF is reached.
pub fn read_plaintext(ctrlc: cu::CtrlcSignal) -> cu::Result<Option<cu::ZString>> {
    loop {
        let Ok(recv) = READ_STDIN.lock() else {
            cu::bail!("failed to acquire stdin reader lock");
        };
        if ctrlc.should_abort() {
            return Ok(None);
        }
        match recv.try_recv() {
            Ok(Ok(x)) => return Ok(Some(x.into())),
            Ok(Err(e)) => cu::rethrow!(e, "error reading stdin"),
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(TryRecvError::Disconnected) => {
                // this may happen on EOF
                return Ok(None);
            }
        }
    }
}

/// Read a password from the terminal with echo disabled.
///
/// Spawns a dedicated thread for each call to read the password.
/// A global lock ensures only one password read can occur at a time.
/// Polls every 200ms to check if the ctrlc signal has been triggered.
///
/// Returns `Ok(None)` if ctrlc is triggered.
pub fn read_password(ctrlc: cu::CtrlcSignal) -> cu::Result<Option<cu::ZString>> {
    let (send, recv) = oneshot::channel();
    let (reader, _guard) = cu::check!(
        password::open_password_input(),
        "failed to open password input"
    )?;

    // prevent multiple callers from reading tty at the same time
    thread::spawn(move || {
        use std::io::BufRead as _;

        let mut reader = reader;
        let mut password = cu::ZString::default();
        let result = match reader.read_line(&mut password) {
            Err(e) => Err(e),
            Ok(_) => Ok(password),
        };
        let _ = send.send(result);
    });

    loop {
        if ctrlc.should_abort() {
            return Ok(None);
        }
        match recv.try_recv() {
            Ok(Ok(x)) => return Ok(Some(x)),
            Ok(Err(e)) => cu::rethrow!(e, "error reading password"),
            Err(oneshot::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(oneshot::TryRecvError::Disconnected) => {
                return Ok(None);
            }
        }
    }
}

/// This module was taken from the rpassword and rtoolbox project and modified
/// to remove unnecessary print dependencies
///
/// See https://docs.rs/rtoolbox/latest/rtoolbox
///
/// See original license information below
///
/// SPDX-License-Identifier: Apache-2.0
/// Copyright (c) https://github.com/conradkleinespel/rpassword
mod password {
    use std::fs::File;
    use std::io::{self, BufReader};
    use std::sync::{Mutex, MutexGuard};

    pub struct PasswordInputGuard {
        hidden_input_guard: imp::HiddenInputGuard,
        tty_guard: Option<MutexGuard<'static, ()>>,
    }

    pub fn open_password_input() -> io::Result<(BufReader<File>, PasswordInputGuard)> {
        static READ_PASSWORD: Mutex<()> = Mutex::new(());
        let tty_guard = READ_PASSWORD.lock().ok();
        let (reader, hidden_input_guard) = imp::open_console()?;
        Ok((
            reader,
            PasswordInputGuard {
                hidden_input_guard,
                tty_guard,
            },
        ))
    }
    #[cfg(unix)]
    mod imp {
        use libc::{ECHO, ECHONL, TCSANOW, c_int, tcsetattr, termios};
        use std::fs::File;
        use std::io::{self, BufReader};
        use std::os::unix::io::AsRawFd;

        pub fn open_console() -> io::Result<(BufReader<File>, HiddenInputGuard)> {
            // open tty as fd
            let tty = std::fs::File::open("/dev/tty")?;
            let fd = tty.as_raw_fd();
            let guard = HiddenInputGuard::try_new(fd)?;
            let reader = io::BufReader::new(tty);
            Ok((reader, guard))
        }
        pub struct HiddenInputGuard {
            fd: i32,
            original_attr: termios,
        }
        impl HiddenInputGuard {
            fn try_new(fd: i32) -> io::Result<Self> {
                // Make two copies of the terminal settings. The first one will be modified
                // and the second one will act as a backup for when we want to set the
                // terminal back to its original state.
                let mut term = safe_tcgetattr(fd)?;
                let original_attr = safe_tcgetattr(fd)?;
                // Hide the password. This is what makes this function useful.
                term.c_lflag &= !ECHO;
                // But don't hide the NL character when the user hits ENTER.
                term.c_lflag |= ECHONL;
                // Save the settings for now.
                io_result(unsafe { tcsetattr(fd, TCSANOW, &term) })?;
                Ok(Self { fd, original_attr })
            }
        }
        impl Drop for HiddenInputGuard {
            fn drop(&mut self) {
                // Set the mode back to normal
                unsafe {
                    tcsetattr(self.fd, TCSANOW, &self.original_attr);
                }
            }
        }
        fn safe_tcgetattr(fd: c_int) -> io::Result<termios> {
            let mut term = std::mem::MaybeUninit::<termios>::uninit();
            io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
            Ok(unsafe { term.assume_init() })
        }
        fn io_result(ret: c_int) -> io::Result<()> {
            match ret {
                0 => Ok(()),
                _ => Err(io::Error::last_os_error()),
            }
        }
    }

    #[cfg(windows)]
    mod imp {
        use std::fs::File;
        use std::io::{self, BufReader};
        use std::os::windows::io::FromRawHandle;
        use windows_sys::Win32::Foundation::{
            GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            CreateFileA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows_sys::Win32::System::Console::{
            CONSOLE_MODE, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, GetConsoleMode, SetConsoleMode,
        };
        use windows_sys::core::PCSTR;

        pub fn open_console() -> io::Result<(BufReader<File>, HiddenInputGuard)> {
            let handle = unsafe {
                CreateFileA(
                    c"CONIN$".as_ptr() as PCSTR,
                    GENERIC_READ | GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    INVALID_HANDLE_VALUE,
                )
            };

            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
            let guard = HiddenInputGuard::try_new(handle)?;
            let reader = BufReader::new(unsafe { std::fs::File::from_raw_handle(handle as _) });
            Ok((reader, guard))
        }

        pub struct HiddenInputGuard {
            handle: HANDLE,
            original_mode: u32,
        }
        impl HiddenInputGuard {
            fn try_new(handle: HANDLE) -> io::Result<Self> {
                let mut original_mode = 0u32;

                // Get the old mode so we can reset back to it when we are done
                if unsafe { GetConsoleMode(handle, &mut original_mode as *mut CONSOLE_MODE) } == 0 {
                    return Err(io::Error::last_os_error());
                }

                // We want to be able to read line by line, and we still want backspace to work
                let new_mode_flags = ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
                if unsafe { SetConsoleMode(handle, new_mode_flags) } == 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(Self {
                    handle,
                    original_mode,
                })
            }
        }
        impl Drop for HiddenInputGuard {
            fn drop(&mut self) {
                // Set the mode back to normal
                unsafe {
                    SetConsoleMode(self.handle, self.original_mode);
                }
            }
        }
    }
}
