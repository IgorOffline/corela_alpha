use alloc::vec::Vec;
use core::ffi::c_int;
use core::net::SocketAddrV4;
use core::sync::atomic::{AtomicI32, Ordering};

const AF_INET: c_int = 2;
const SOCK_STREAM: c_int = 1;
const MSG_NOSIGNAL: c_int = 0x4000;
const OPCODE_TEXT: u8 = 0x1;
const OPCODE_CLOSE: u8 = 0x8;
const CLOSE_NORMAL: u16 = 1000;

#[repr(C)]
struct SockaddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: [u8; 4],
    sin_zero: [u8; 8],
}

unsafe extern "C" {
    fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int;
    fn connect(fd: c_int, addr: *const SockaddrIn, len: u32) -> c_int;
    fn send(fd: c_int, buf: *const u8, len: usize, flags: c_int) -> isize;
    fn recv(fd: c_int, buf: *mut u8, len: usize, flags: c_int) -> isize;
    fn close(fd: c_int) -> c_int;
    fn getrandom(buf: *mut u8, len: usize, flags: u32) -> isize;
}

static FD: AtomicI32 = AtomicI32::new(-1);

fn random<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    unsafe { getrandom(buf.as_mut_ptr(), N, 0) };
    buf
}

fn base64(bytes: &[u8]) -> Vec<u8> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                TABLE[(n >> (18 - 6 * i) & 0x3f) as usize]
            } else {
                b'='
            });
        }
    }
    out
}

fn send_all(fd: c_int, mut buf: &[u8]) -> bool {
    while !buf.is_empty() {
        let n = unsafe { send(fd, buf.as_ptr(), buf.len(), MSG_NOSIGNAL) };
        if n <= 0 {
            return false;
        }
        buf = &buf[n as usize..];
    }
    true
}

fn upgraded(fd: c_int) -> bool {
    let mut response = Vec::new();
    let mut byte = 0u8;
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() > 4096 || unsafe { recv(fd, &mut byte, 1, 0) } != 1 {
            return false;
        }
        response.push(byte);
    }
    response.starts_with(b"HTTP/1.1 101")
}

pub fn open(addr: SocketAddrV4) -> bool {
    let fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
    if fd < 0 {
        return false;
    }
    let sockaddr = SockaddrIn {
        sin_family: AF_INET as u16,
        sin_port: addr.port().to_be(),
        sin_addr: addr.ip().octets(),
        sin_zero: [0; 8],
    };
    let mut request = Vec::new();
    request.extend_from_slice(b"GET / HTTP/1.1\r\nHost: ");
    request.extend_from_slice(alloc::format!("{addr}").as_bytes());
    request
        .extend_from_slice(b"\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: ");
    request.extend_from_slice(&base64(&random::<16>()));
    request.extend_from_slice(b"\r\nSec-WebSocket-Version: 13\r\n\r\n");
    let ok = unsafe { connect(fd, &sockaddr, size_of::<SockaddrIn>() as u32) } == 0
        && send_all(fd, &request)
        && upgraded(fd);
    if ok {
        FD.store(fd, Ordering::Relaxed);
    } else {
        unsafe { close(fd) };
    }
    ok
}

fn send_frame(opcode: u8, payload: &[u8]) {
    let fd = FD.load(Ordering::Relaxed);
    if fd < 0 {
        return;
    }
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x80 | opcode);
    match payload.len() {
        n @ 0..=125 => frame.push(0x80 | n as u8),
        n @ 126..=0xffff => {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    let mask = random::<4>();
    frame.extend_from_slice(&mask);
    frame.extend(payload.iter().zip(mask.iter().cycle()).map(|(b, m)| b ^ m));
    if !send_all(fd, &frame) {
        shut();
    }
}

fn shut() {
    let fd = FD.swap(-1, Ordering::Relaxed);
    if fd >= 0 {
        unsafe { close(fd) };
    }
}

pub fn send_text(text: &str) {
    send_frame(OPCODE_TEXT, text.as_bytes());
}

pub fn disconnect() {
    send_frame(OPCODE_CLOSE, &CLOSE_NORMAL.to_be_bytes());
    shut();
}
