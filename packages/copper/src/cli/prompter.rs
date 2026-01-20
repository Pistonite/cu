use std::io::{self, IsTerminal};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::Duration;

use cu::pre::*;

static READ_STDIN: LazyLock<Mutex<Receiver<io::Result<cu::ZString>>>> = LazyLock::new(|| {
    let (send, recv) = mpsc::sync_channel(0);

    thread::spawn(move || {
        use std::io::{BufRead as _, IsTerminal as _};
        let stdin = io::stdin();

        if !stdin.is_terminal() {
            // just process by lines - this also locks the stdin forever
            // while we are waiting for receiver
            for line in stdin.lines() {
                // lines iterator are guaranteed to not containing the CRLF/LF at the end
                // note we do not check error because the static receiver will never be dropped
                let _ = send.send(line.map(cu::ZString::from));
            }
            return;
        }

        // when reading from terminal, what we want:
        // Ctrl-D: the behavior is not consistent.
        // - some terminal will flush inputs to stdin
        // - some terminal will treat it as a normal character \u{4}
        // Ctrl-C: in my testing, most terminals behave the same, which is to discard
        //   unflushed portion of stdin and start over
        //
        // for example, for the following input sequence:
        //   asdf^Dtest<CR>
        //   FIRST BEHAVIOR         SECOND BEHAVIOR
        //   "asdf", "test\n"       "asdf\u{4}test\r\n"
        //
        // another example
        //   asdf^Dtest^Chello<CR>
        //   FIRST BEHAVIOR         SECOND BEHAVIOR
        //   "asdf", "hello\n"      "hello\r\n"
        //
        // we have to know if stdin would block to reliably detect this. This is
        // not worth the effort to look into right now.
        // So, we will depend on the terminal behavior for this.
        // Ctrl-C is also handled by terminal itself. we will not receive the bytes
        // from read_line if Ctrl-C is pressed

        // we will lock stdin so no one else in the universe can read,
        // since multi-threaded read from stdin can have issues
        // all prompting within cu is driven by the printer thread, so no issues there
        let mut stdin = stdin.lock();

        let mut buf = cu::ZString::default();
        // we will loop forever, when the program exits, the thread will silently be destroyed
        // by OS - unlike the printer thread needs to be joined to ensure printer animations
        // are processed properly before program ends
        loop {
            // insecure clear, just to be safe that we are not reading into existing data
            buf.clear();
            let to_send = loop {
                match stdin.read_line(&mut buf) {
                    Ok(0) => {
                        // can happen if ^D is pressed without any input
                        continue;
                    }
                    Ok(_) => {
                        // remove line ending - note we do trim trailing whitespaces here
                        // - that will be done by prompt configuration
                        if buf.as_bytes().last() == Some(&b'\n') {
                            let l = buf.len() - 1;
                            buf.truncate(l);
                            if buf.as_bytes().last() == Some(&b'\r') {
                                let l = buf.len() - 1;
                                buf.truncate(l);
                            }
                        }
                        // this should reset buf to default without a buffer allocated,
                        // meaning the buffer with sensitive user input will be transferred
                        // to receiver
                        break Ok(std::mem::take(&mut buf));
                    }
                    Err(e) => {
                        break Err(e);
                    }
                }
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
            Ok(Ok(x)) => return Ok(Some(x)),
            Ok(Err(e)) => cu::rethrow!(e, "error reading stdin"),
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(TryRecvError::Disconnected) => {
                cu::bail!("reached end of input (EOF)");
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
    //
    let handle = cu::check!(
        password::Handle::open(),
        "error opening terminal for reading password"
    )?;
    // if stdin is terminal, then we just hide the input and read from that
    if io::stdin().is_terminal() {
        let _guard = cu::check!(
            handle.into_guard(),
            "error setting hidden input in terminal for reading password"
        )?;
        return read_plaintext(ctrlc);
    }
    // if stdin is NOT terminal, then we open the terminal file to read from it
    let reader = cu::check!(
        handle.into_reader(),
        "error opening terminal reader for reading password"
    )?;
    reader.poll_read(ctrlc)
}
mod password {
    use std::fs::File;
    use std::io;
    use std::thread;
    use std::time::Duration;

    use cu::Context as _;

    pub struct Handle {
        inner: imp::HandleType,
        file: File,
    }
    pub struct Reader {
        // order matters here - we must drop the guard first
        // because the guard stores the FD of the file
        #[allow(unused)]
        guard: imp::HiddenInputGuard,
        inner: io::BufReader<File>,
    }
    pub struct Guard {
        // order matters here - we must drop the guard first
        // because the guard stores the FD of the file
        #[allow(unused)]
        guard: imp::HiddenInputGuard,
        #[allow(unused)]
        inner: File,
    }
    impl Reader {
        pub fn poll_read(self, ctrlc: cu::CtrlcSignal) -> cu::Result<Option<cu::ZString>> {
            let (send, recv) = oneshot::channel();
            let mut reader = self.inner;
            let guard = self.guard;

            thread::spawn(move || {
                use std::io::BufRead as _;

                let result = loop {
                    let mut password = cu::ZString::default();
                    match reader.read_line(&mut password) {
                        Ok(0) => continue,
                        Ok(_) => {
                            // remove line ending - note we do trim trailing whitespaces here
                            // - that will be done by prompt configuration
                            if password.as_bytes().last() == Some(&b'\n') {
                                let l = password.len() - 1;
                                password.truncate(l);
                                if password.as_bytes().last() == Some(&b'\r') {
                                    let l = password.len() - 1;
                                    password.truncate(l);
                                }
                            }
                            break Ok(password);
                        }
                        Err(e) => break Err(e),
                    }
                };
                // must send the reader and guard to enforce drop order
                // to restore the terminal
                if let Err(e) = send.send((result, reader, guard)) {
                    let (_, reader, guard) = e.into_inner();
                    drop(guard);
                    drop(reader);
                }
            });

            loop {
                if ctrlc.should_abort() {
                    return Ok(None);
                }
                match recv.try_recv() {
                    Ok((Ok(x), reader, guard)) => {
                        drop(guard);
                        drop(reader);
                        return Ok(Some(x));
                    }
                    Ok((Err(e), reader, guard)) => {
                        drop(guard);
                        drop(reader);
                        cu::rethrow!(e, "error reading password");
                    }
                    Err(oneshot::TryRecvError::Empty) => {
                        thread::sleep(Duration::from_millis(200));
                    }
                    Err(oneshot::TryRecvError::Disconnected) => {
                        cu::bail!("reached end of input (EOF)");
                    }
                }
            }
        }
    }

    // implementation was taken from the rpassword and rtoolbox project and modified
    // to remove unnecessary print dependencies
    //
    // See https://docs.rs/rtoolbox/latest/rtoolbox
    //
    // See original license information below
    //
    // SPDX-License-Identifier: Apache-2.0
    // Copyright (c) https://github.com/conradkleinespel/rpassword
    #[cfg(unix)]
    mod imp {
        use libc::{ECHO, ECHONL, TCSANOW, c_int, tcsetattr, termios};
        use std::fs::File;
        use std::io;
        use std::os::unix::io::AsRawFd;

        pub type HandleType = i32; // fd

        impl super::Handle {
            pub fn open() -> io::Result<Self> {
                let tty = File::open("/dev/tty")?;
                let fd = tty.as_raw_fd();
                Ok(Self {
                    inner: fd,
                    file: tty,
                })
            }
            pub fn into_reader(self) -> io::Result<super::Reader> {
                let reader = io::BufReader::new(self.file);
                let guard = HiddenInputGuard::try_new(self.inner)?;
                Ok(super::Reader {
                    inner: reader,
                    guard,
                })
            }
            pub fn into_guard(self) -> io::Result<super::Guard> {
                let guard = HiddenInputGuard::try_new(self.inner)?;
                Ok(super::Guard {
                    inner: self.file,
                    guard,
                })
            }
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
