use std::io::Write as _;

use tokio::{io::AsyncReadExt as _, process::Command};

#[tokio::main]
async fn main() {
    let mut child = Command::new("git")
    .args(["clone", "--depth", "1", "https://github.com/zeldaret/botw", "--progress"])
        .stderr(std::process::Stdio::piped())
        .spawn().unwrap();

    let mut stderr = child.stderr.take().unwrap();

    let child_handle = tokio::spawn(async move {
        child.wait().await
    });

    let mut buffer = [0u8; 1024];
    loop {
        match stderr.read(&mut buffer).await {
            Err(x) => {
                println!("read err: {x}");
                break;
            }
            Ok(n) => {
                if n == 0 {
                    println!("read end");
                    break;
                }
                println!("read got {n} bytes");
                // it's guaranteed ascii
                for b in &buffer[0..n] {
                    let c = *b as char;
                    if c == '\r' {
                        let _ = write!(std::io::stdout(), "\\r\n");
                    } else {
                        let _ = write!(std::io::stdout(), "{c}");
                    }
                }
                let _ = std::io::stdout().flush();
            }
        }
    }
    child_handle.await.unwrap().unwrap();
    println!("done clone");

    std::fs::remove_dir_all("botw").unwrap();


}
