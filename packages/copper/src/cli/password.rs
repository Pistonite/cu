// This file was taken from the rpassword and rtoolbox project and modified
// to remove unnecessary print dependencies
//
// See https://docs.rs/rtoolbox/latest/rtoolbox
//
// See original license information below
//
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) https://github.com/conradkleinespel/rpassword

#[cfg(unix)]
pub(crate) use unix::read_password;
#[cfg(windows)]
pub(crate) use windows::read_password;

#[cfg(unix)]
mod unix {
    use libc::{ECHO, ECHONL, TCSANOW, c_int, tcsetattr, termios};
    use std::io::{self, BufRead};
    use std::mem;
    use std::os::unix::io::AsRawFd;

    struct HiddenInput {
        fd: i32,
        term_orig: termios,
    }

    impl HiddenInput {
        fn new(fd: i32) -> io::Result<HiddenInput> {
            // Make two copies of the terminal settings. The first one will be modified
            // and the second one will act as a backup for when we want to set the
            // terminal back to its original state.
            let mut term = safe_tcgetattr(fd)?;
            let term_orig = safe_tcgetattr(fd)?;

            // Hide the password. This is what makes this function useful.
            term.c_lflag &= !ECHO;

            // But don't hide the NL character when the user hits ENTER.
            term.c_lflag |= ECHONL;

            // Save the settings for now.
            io_result(unsafe { tcsetattr(fd, TCSANOW, &term) })?;

            Ok(HiddenInput { fd, term_orig })
        }
    }

    impl Drop for HiddenInput {
        fn drop(&mut self) {
            // Set the mode back to normal
            unsafe {
                tcsetattr(self.fd, TCSANOW, &self.term_orig);
            }
        }
    }

    fn safe_tcgetattr(fd: c_int) -> io::Result<termios> {
        let mut term = mem::MaybeUninit::<termios>::uninit();
        io_result(unsafe { ::libc::tcgetattr(fd, term.as_mut_ptr()) })?;
        Ok(unsafe { term.assume_init() })
    }

    /// Reads a password from the TTY
    pub(crate) fn read_password() -> io::Result<crate::ZString> {
        let tty = std::fs::File::open("/dev/tty")?;
        let fd = tty.as_raw_fd();
        let mut reader = io::BufReader::new(tty);

        read_password_from_fd_with_hidden_input(&mut reader, fd)
    }

    /// Reads a password from a given file descriptor
    fn read_password_from_fd_with_hidden_input(
        reader: &mut impl BufRead,
        fd: i32,
    ) -> io::Result<crate::ZString> {
        let mut password = crate::ZString::default();
        {
            let _hidden_input = HiddenInput::new(fd)?;
            reader.read_line(&mut password)?;
        }
        Ok(password.trim().to_string().into())
    }
}

#[cfg(windows)]
mod windows {
    use std::io::{self, BufRead, BufReader};
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

    struct HiddenInput {
        mode: u32,
        handle: HANDLE,
    }

    impl HiddenInput {
        fn new(handle: HANDLE) -> io::Result<HiddenInput> {
            let mut mode = 0;

            // Get the old mode so we can reset back to it when we are done
            if unsafe { GetConsoleMode(handle, &mut mode as *mut CONSOLE_MODE) } == 0 {
                return Err(io::Error::last_os_error());
            }

            // We want to be able to read line by line, and we still want backspace to work
            let new_mode_flags = ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
            if unsafe { SetConsoleMode(handle, new_mode_flags) } == 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(HiddenInput { mode, handle })
        }
    }

    impl Drop for HiddenInput {
        fn drop(&mut self) {
            // Set the mode back to normal
            unsafe {
                SetConsoleMode(self.handle, self.mode);
            }
        }
    }

    /// Reads a password from the TTY
    pub fn read_password() -> io::Result<crate::ZString> {
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

        let mut stream = BufReader::new(unsafe { std::fs::File::from_raw_handle(handle as _) });
        read_password_from_handle_with_hidden_input(&mut stream, handle)
    }

    /// Reads a password from a given file handle
    fn read_password_from_handle_with_hidden_input(
        reader: &mut impl BufRead,
        handle: HANDLE,
    ) -> io::Result<crate::ZString> {
        let mut password = crate::ZString::default();
        {
            let _hidden_input = HiddenInput::new(handle)?;
            reader.read_line(&mut password)?;
        }
        Ok(password.trim().to_string().into())
    }
}
macro_rules! special_chars {
    ($c1:literal | $($c:literal)|* $(|)?) => {
        static LEGAL_PASSWORD_ERROR_MESSAGE: &str = concat!(
            "password contains illegal characters, only ascii alphanumeric characters and special characters in the following list are allowed: ",
            stringify!($c1),
            $( ", ", stringify!($c), )*
        );
        #[inline(always)]
        fn special_char_legal(c: char) -> bool {
            matches!(c, $c1 $( | $c )* )
        }
    }
}
special_chars! { '!' | '#' | '$' | '%' | '&' | '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/' | ':' | ';' | '<' | '=' | '>' | '?' | '@' | '[' | ']' | '^' | '_' | '`' | '{' | '|' | '}' | '~'}
/// Check if the password contains all "legal" characters (and is non-empty)
pub fn password_chars_legal(s: &str) -> crate::Result<()> {
    if s.is_empty() {
        crate::bail!("password cannot be empty");
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || special_char_legal(c))
    {
        return Ok(());
    }
    crate::bail!("{LEGAL_PASSWORD_ERROR_MESSAGE}");
}
