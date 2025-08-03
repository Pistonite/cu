use tokio::io::AsyncReadExt as _;
use tokio::process::{ChildStderr, ChildStdout};

/// Drive that takes the out stream and err stream, and produces lines
pub(crate) struct Driver {
    out: Option<ChildStdout>,
    out_buf: String,
    err: Option<ChildStderr>,
    err_buf: String,
    out_i: usize,
    err_i: usize,
    buffer: Box<[u8; 1024]>,
    line_buf: String
}
pub(crate) enum DriverOutput<'a> {
    /// A non-empty line ending with a line break
    Line(&'a str),
    /// A non-empty progress line, typically because a `\r` was detected
    Progress(&'a str),
    /// No output yet, call me again
    Pending,
    /// An empty line was detected
    Empty,
    /// No more to be read from us
    Done,
}
impl<'a> DriverOutput<'a> {
    fn new(line: &'a str, lf: bool) -> Self {
        let line = line.trim();
        if line.is_empty() {
            Self::Empty
        }else if lf {
            Self::Line(line)
        } else {
            Self::Progress(line)
        }
    }
}
impl Driver {
    pub fn new(out: Option<ChildStdout>, err: Option<ChildStderr>) -> Self {
        Self {
            out,
            out_buf: String::new(),
            err,
            err_buf: String::new(),
            out_i: 0,
            err_i: 0,
            buffer: Box::new([0u8; 1024]),
            line_buf: String::new()
        }
    }
    /// Return the next line and whether it has a line break
    /// If the line is empty after trimming, returns 
    pub async fn next(&mut self) -> DriverOutput<'_> {
        match (self.out.as_mut(), self.err.as_mut()) {
            (None, None) => DriverOutput::Done,
            (None, Some(s)) => {
                match s.read(&mut self.buffer.as_mut()[self.err_i..]).await {
                    // probably finished reading
                    Err(_) | Ok(0) => DriverOutput::Done,
                    Ok(n) => {
                        let end = self.err_i +n;
                        let slice = &self.buffer.as_ref()[..end];
                        let (b, lf) = Self::process(slice, &mut self.err_buf, &mut self.line_buf);
                        let buf_mut = self.buffer.as_mut();
                        // shift the remaining section of buf
                        for i in b..end {
                            buf_mut[i-b] = buf_mut[i];
                        }
                        self.err_i = end-b;
                        DriverOutput::new(&self.line_buf, lf)
                    }
                }
            }
            (Some(s), None) => {
                match s.read(&mut self.buffer.as_mut()[self.out_i..]).await {
                    // probably finished reading
                    Err(_) | Ok(0) => DriverOutput::Done,
                    Ok(n) => {
                        let end = self.out_i +n;
                        let slice = &self.buffer.as_ref()[..end];
                        let (b, lf) = Self::process(slice, &mut self.out_buf, &mut self.line_buf);
                        let buf_mut = self.buffer.as_mut();
                        // shift the remaining section of buf
                        for i in b..end {
                            buf_mut[i-b] = buf_mut[i];
                        }
                        self.out_i = end-b;
                        DriverOutput::new(&self.line_buf, lf)
                    }
                }
            }
            (Some(so), Some(se)) => {
                let mid = self.buffer.len()/2;
                let (buf_o, buf_e) = self.buffer.as_mut().split_at_mut(mid);
                // read is cancel safe - if canceled, nothing will be read
                tokio::select! {
                    x = so.read(&mut buf_o[self.out_i..]) => {match x {
                        Err(_) | Ok(0) => {
                            let buf_mut = self.buffer.as_mut();
                            for i in 0..self.err_i {
                                buf_mut[i] = buf_mut[mid+i];
                            }
                            self.out = None;
                            DriverOutput::Pending
                        }
                        Ok(n) => {
                            let end = self.out_i +n;
                            let slice = &buf_o[..end];
                            let (b, lf) = Self::process(slice, &mut self.out_buf, &mut self.line_buf);
                            // shift the remaining section of buf
                            for i in b..end {
                                buf_o[i-b] = buf_o[i];
                            }
                            self.out_i = end-b;
                            DriverOutput::new(&self.line_buf, lf)
                        }
                    }}
                        x = se.read(&mut buf_e[self.err_i..]) => {match x{
                            Err(_) | Ok(0) => {
                                self.err = None;
                                DriverOutput::Pending
                            }
                            Ok(n) => {
                                let end = self.err_i +n;
                                let slice = &buf_e[..end];
                                let (b, lf) = Self::process(slice, &mut self.err_buf, &mut self.line_buf);
                                // shift the remaining section of buf
                                for i in b..end {
                                    buf_e[i-b] = buf_e[i];
                                }
                                self.err_i = end-b;
                                DriverOutput::new(&self.line_buf, lf)
                            }
                        }}
                }
            }
        }
    }

    // out will contain the remaining characters that's not a line,
    // and line will contain the last non-empty line after stripping
    // return how many bytes from buf are used and if the line is ends with `\n`
    fn process(buf: &[u8], out: &mut String, line: &mut String) -> (usize, bool) {
        use crate::print::{utf8, ansi};
        let mut i = 0;
        let mut is_escaping = false;
        line.clear();
        let mut full_line = false;
        
        let mut last: char = '\0';
        while i < buf.len() {
            match utf8::decode_char(&buf[i..]) {
                Err(true) => {
                    // invalid, skip one
                    i += 1
                }
                Err(false) => {
                    // not enough bytes
                    break;
                }
                Ok((c, l)) => {
                    i += l;
                    let prev = last;
                    last = c;
                    if is_escaping {
                        if ansi::is_ansi_end_char(c) {
                            is_escaping = false;
                        }
                        continue;
                    }
                    if c == '\x1b' {
                        is_escaping = true;
                        continue;
                    }
                    if c == '\r' || c == '\n' {
                        if c == '\n' && prev == '\r' {
                            full_line = true;
                        } else {
                            line.clear();
                            line.push_str(out);
                            full_line = c == '\n';
                        }
                        out.clear();
                        continue;
                    }
                    if c.is_control() {
                        continue;
                    }
                    out.push(c)
                }
            }
        }
        (i, full_line)
    }
}
