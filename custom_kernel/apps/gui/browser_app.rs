use crate::gui::app::App;
use crate::drivers::keyboard;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

pub struct BrowserApp {
    url: String,
    content: Vec<String>,
    scroll_y: i32,
    loading: bool,
}

impl BrowserApp {
    pub fn new() -> Self {
        let mut content = Vec::new();
        content.push(String::from("Welcome to Ainux Web Browser!"));
        content.push(String::from(""));
        content.push(String::from("Type a URL and press Enter to browse."));
        content.push(String::from("Currently supported protocols: file://"));
        
        Self {
            url: String::from("file://index.html"),
            content,
            scroll_y: 0,
            loading: false,
        }
    }

    fn fill(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for dy in 0..h {
            let sy = y + dy; if sy < 0 || sy >= bh as i32 { continue; }
            for dx in 0..w {
                let sx = x + dx; if sx < 0 || sx >= bw as i32 { continue; }
                buf[(sy * bw as i32 + sx) as usize] = c;
            }
        }
    }

    fn text(buf: &mut [u32], bw: usize, bh: usize, x: i32, y: i32, s: &str, c: u32) {
        crate::drivers::video::draw_text_to_buffer(buf, bw as i64, bh as i64, x as i64, y as i64, s, c);
    }
}

impl App for BrowserApp {
    fn update(&mut self) {}

    fn draw(&mut self, buf: &mut [u32], w: usize, h: usize) {
        // Toolbar background
        Self::fill(buf, w, h, 0, 0, w as i32, 40, 0xFFE0E0E0);
        Self::fill(buf, w, h, 0, 40, w as i32, 1, 0xFF808080);
        
        // URL Bar
        Self::fill(buf, w, h, 10, 10, w as i32 - 80, 20, 0xFFFFFFFF);
        Self::fill(buf, w, h, 10, 30, w as i32 - 80, 1, 0xFF808080);
        Self::fill(buf, w, h, 10, 10, 1, 20, 0xFF808080);
        
        let display_url = format!("{}|", self.url);
        Self::text(buf, w, h, 15, 12, &display_url, 0x000000);
        
        // Go Button
        Self::fill(buf, w, h, w as i32 - 60, 10, 50, 20, 0xFFC0C0C0);
        Self::text(buf, w, h, w as i32 - 45, 12, "Go", 0x000000);

        // Content Area
        Self::fill(buf, w, h, 0, 41, w as i32, h as i32 - 41, 0xFFFFFFFF);
        
        let mut text_y = 50 - self.scroll_y;
        for line in &self.content {
            if text_y > 40 && text_y < h as i32 {
                Self::text(buf, w, h, 10, text_y, line, 0x000000);
            }
            text_y += 20;
        }
    }

    fn on_mouse_event(&mut self, _x: i32, _y: i32, _buttons: u8) {}

    fn on_key_event(&mut self, c: char) {
        if c == '\n' {
            self.content.clear();
            self.content.push(format!("Loading {}...", self.url));
            
            let mut domain = self.url.as_str();
            if domain.starts_with("http://") {
                domain = &domain[7..];
            } else if domain.starts_with("https://") {
                domain = &domain[8..];
            }
            
            let mut path = "/";
            if let Some(slash_idx) = domain.find('/') {
                path = &domain[slash_idx..];
                domain = &domain[..slash_idx];
            }
            
            if let Some(ip) = crate::net::dns::resolve(domain) {
                let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, domain);
                
                let mut tcp_handle = None;
                crate::cpu::without_interrupts(|| {
                    let mut net_opt = crate::net::NET_STACK.lock();
                    if let Some(net) = net_opt.as_mut() {
                        let rx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                        let tx_buffer = smoltcp::socket::tcp::SocketBuffer::new(alloc::vec![0; 65535]);
                        let mut socket = smoltcp::socket::tcp::Socket::new(rx_buffer, tx_buffer);
                        let local_port = 49152 + (crate::process::scheduler::get_ticks() % 16384) as u16;
                        let _ = socket.connect(net.iface.context(), (ip, 80), local_port);
                        tcp_handle = Some(net.sockets.add(socket));
                    }
                });
                
                if let Some(handle) = tcp_handle {
                    self.content.push(String::from("Connecting..."));
                    
                    let mut timeout = 0;
                    let mut connected = false;
                    while timeout < 1_000_000 {
                        let mut state = smoltcp::socket::tcp::State::Closed;
                        crate::cpu::without_interrupts(|| {
                            let mut net_opt = crate::net::NET_STACK.lock();
                            if let Some(net) = net_opt.as_mut() {
                                let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                                state = socket.state();
                            }
                        });
                        
                        if state == smoltcp::socket::tcp::State::Established {
                            connected = true;
                            break;
                        }
                        crate::net::poll();
                        crate::process::scheduler::yield_now();
                        timeout += 1;
                    }
                    
                    if connected {
                        self.content.push(String::from("Connected! Requesting..."));
                        crate::cpu::without_interrupts(|| {
                            let mut net_opt = crate::net::NET_STACK.lock();
                            if let Some(net) = net_opt.as_mut() {
                                let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                                let _ = socket.send_slice(request.as_bytes());
                            }
                        });
                        
                        let mut response = Vec::new();
                        timeout = 0;
                        while timeout < 2_000_000 {
                            let mut recvd = 0;
                            let mut active = false;
                            crate::cpu::without_interrupts(|| {
                                let mut net_opt = crate::net::NET_STACK.lock();
                                if let Some(net) = net_opt.as_mut() {
                                    let socket = net.sockets.get_mut::<smoltcp::socket::tcp::Socket>(handle);
                                    active = socket.is_active();
                                    if socket.can_recv() {
                                        let mut buf = [0u8; 1024];
                                        if let Ok(n) = socket.recv_slice(&mut buf) {
                                            if n > 0 {
                                                response.extend_from_slice(&buf[..n]);
                                                recvd = n;
                                            }
                                        }
                                    }
                                }
                            });
                            
                            if recvd > 0 {
                                timeout = 0; // reset
                            } else if !active {
                                break;
                            }
                            crate::net::poll();
                            crate::process::scheduler::yield_now();
                            timeout += 1;
                        }
                        
                        if response.is_empty() {
                            self.content.push(String::from("Error: Empty response"));
                        } else {
                            if let Ok(s) = core::str::from_utf8(&response) {
                                let lines: Vec<&str> = s.split('\n').collect();
                                for l in lines.into_iter().take(200) {
                                    self.content.push(String::from(l.trim_end_matches('\r')));
                                }
                            } else {
                                self.content.push(format!("Received {} bytes of binary data.", response.len()));
                            }
                        }
                    } else {
                        self.content.push(String::from("Error: Connection timeout."));
                    }
                    
                    crate::cpu::without_interrupts(|| {
                        if let Some(net) = crate::net::NET_STACK.lock().as_mut() {
                            net.sockets.remove(handle);
                        }
                    });
                } else {
                    self.content.push(String::from("Error: Socket error"));
                }
            } else {
                self.content.push(String::from("Error: DNS failed"));
            }
        } else if c == '\x08' {
            self.url.pop();
        } else if c >= ' ' && c <= '~' {
            self.url.push(c);
        } else if c == keyboard::KEY_UP {
            self.scroll_y = (self.scroll_y - 20).max(0);
        } else if c == keyboard::KEY_DOWN {
            self.scroll_y += 20;
        }
    }
}
