use clap::{Parser, Subcommand};
#[cfg(target_os = "linux")]
use nix::mount::{mount, umount, MsFlags};
use nix::sys::stat::Mode;
use nix::{
    fcntl::{open, OFlag},
    unistd::setsid,
};
use std::os::fd::FromRawFd;
use std::os::unix::fs;
use std::path::Path;
use std::process::Command;
use std::{
    env::current_exe,
    io::{self, copy},
};
use std::{ffi::CStr, fs::File};
use std::{os::unix::process::CommandExt, process::Stdio};

/// dedock is not a container runtime.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Dedock {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run {
        /// Directory to use for container filesystem root.
        #[arg(default_value_t = String::from("./container"))]
        root: String,
    },
    Fork {
        /// File descriptor for tty.
        tty: String,
        /// Directory to use for container filesystem root.
        root: String,
        /// The command to execute after chroot.
        cmd: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dedock = Dedock::parse();
    match &dedock.command {
        Commands::Run { root } => {
            let pty_fd = unsafe { libc::posix_openpt(libc::O_RDWR) };

            if pty_fd < 0 {
                panic!("failed to open pty");
            }

            let err = unsafe { libc::grantpt(pty_fd) };
            if err < 0 {
                panic!("failed to grantpt");
            }
            let err = unsafe { libc::unlockpt(pty_fd) };
            if err < 0 {
                panic!("failed to grantpt");
            }

            // TODO(hasheddan): consider using ptsname_r on Linux systems.
            let ret = unsafe { libc::ptsname(pty_fd) };
            if ret.is_null() {
                panic!("failed to get ptsname");
            }
            let tty = unsafe { CStr::from_ptr(ret) }.to_string_lossy();

            // Get current executable.
            let exe_path = current_exe().unwrap();
            let exe = exe_path.to_str().unwrap();

            let mut child = Command::new(exe);
            child.args(["fork", &tty, root, "/bin/bash"]);

            let pty_fd_clone = unsafe { libc::fcntl(pty_fd, libc::F_DUPFD, pty_fd) };
            if pty_fd_clone < 0 {
                panic!("failed clone");
            }
            let mut pty_read = unsafe { File::from_raw_fd(pty_fd) };
            let mut pty_write = unsafe { File::from_raw_fd(pty_fd_clone) };
            // TODO(hasheddan): use a more robust reading / writing strategy.
            let writer = tokio::spawn(async move {
                copy(&mut io::stdin(), &mut pty_write);
            });

            // Mount /dev.
            let p = Path::new(root).join("dev");
            const NONE: Option<&'static [u8]> = None;

            // On macOS we call mount as a subcommand to mount devfs.
            #[cfg(target_os = "macos")]
            Command::new("mount")
                .args(["-t", "devfs", "devfs", p.to_str().unwrap()])
                .output()?;

            // On Linux we bind mount /dev.
            #[cfg(target_os = "linux")]
            mount(
                Some(Path::new("/dev")),
                p.as_path(),
                NONE,
                MsFlags::MS_BIND,
                NONE,
            )?;

            // Start runner. stdin, stdout, stderr are inherited by default.
            let mut runner = child.spawn().unwrap();

            let reader = tokio::spawn(async move {
                // Use our own stdout because io::stdout is buffered.
                let mut std = unsafe { File::from_raw_fd(1) };
                copy(&mut pty_read, &mut std);
            });

            let out = runner.wait()?;

            if !out.success() {
                println!("Container exited unsuccessfully.");
            }

            // Unmount /dev.
            #[cfg(target_os = "macos")]
            Command::new("unmount").arg(p.to_str().unwrap()).output()?;
            #[cfg(target_os = "linux")]
            umount(p.as_path())?;
            tokio::join!(writer, reader);
        }
        Commands::Fork { tty, root, cmd } => {
            let tty_fd = open(Path::new(&tty), OFlag::O_RDWR, Mode::empty())?;
            unsafe {
                let mut termios = core::mem::MaybeUninit::uninit();
                let res = libc::tcgetattr(tty_fd, termios.as_mut_ptr());
                if res < 0 {
                    panic!("failed to get tty attributes");
                }
                let mut termios = termios.assume_init();
                libc::cfmakeraw(&mut termios);
                libc::tcsetattr(tty_fd, libc::TCSANOW, &termios);
            };
            let mut cmd = Command::new(cmd);
            unsafe {
                cmd.pre_exec(move || {
                    setsid().unwrap();
                    #[cfg(target_os = "macos")]
                    let tiocsctty: u64 = libc::TIOCSCTTY.into();
                    #[cfg(target_os = "linux")]
                    let tiocsctty = libc::TIOCSCTTY;
                    let errno = libc::ioctl(tty_fd, tiocsctty, 1);
                    if errno == -1 {
                        panic!("failed to set controlling terminal")
                    }
                    Ok(())
                })
            };
            cmd.stdout(unsafe { Stdio::from_raw_fd(tty_fd) });
            cmd.stderr(unsafe { Stdio::from_raw_fd(tty_fd) });
            cmd.stdin(unsafe { Stdio::from_raw_fd(tty_fd) });
            fs::chroot(root)?;
            std::env::set_current_dir("/")?;
            cmd.output()?;
        }
    }
    Ok(())
}
