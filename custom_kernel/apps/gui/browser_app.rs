
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

pub fn browser_main() {
    let id = 7;
    let width = 640;
    let height = 480;
    
    crate::gui::wm::send_message(crate::gui::wm::GuiMessage::CreateWindow {
        id,
        title: alloc::string::String::from("Browser"),
        x: 100,
        y: 100,
        w: width as i32,
        h: height as i32,
    });
    
    let mut buffer = alloc::vec![0xFFFFFFFF; width * height];
    let mut app = BrowserApp::new();
    let mut needs_redraw = true;
    
    loop {
        for event in crate::gui::wm::pop_events(id) {
            match event {
                crate::gui::wm::GuiEvent::MouseClick { .. } => {}
                crate::gui::wm::GuiEvent::KeyPress { key: c } => {
                    if c == '\n' {
                        app.content.clear();
                        app.content.push(format!("Loading {}...", app.url));
                        needs_redraw = true;
                        
                        let mut domain = app.url.as_str();
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
                                app.content.push(String::from("Connecting..."));
                                
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
                                    app.content.push(String::from("Connected! Requesting..."));
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
                                        app.content.push(String::from("Error: Empty response"));
                                    } else {
                                        if let Ok(s) = core::str::from_utf8(&response) {
                                            let lines: Vec<&str> = s.split('\n').collect();
                                            for l in lines.into_iter().take(200) {
                                                app.content.push(String::from(l.trim_end_matches('\r')));
                                            }
                                        } else {
                                            app.content.push(format!("Received {} bytes of binary data.", response.len()));
                                        }
                                    }
                                } else {
                                    app.content.push(String::from("Error: Connection timeout."));
                                }
                                
                                crate::cpu::without_interrupts(|| {
                                    if let Some(net) = crate::net::NET_STACK.lock().as_mut() {
                                        net.sockets.remove(handle);
                                    }
                                });
                            } else {
                                app.content.push(String::from("Error: Socket error"));
                            }
                        } else {
                            app.content.push(String::from("Error: DNS failed"));
                        }
                    } else if c == '\x08' {
                        app.url.pop();
                        needs_redraw = true;
                    } else if c >= ' ' && c <= '~' {
                        app.url.push(c);
                        needs_redraw = true;
                    } else if c == keyboard::KEY_UP {
                        app.scroll_y = (app.scroll_y - 20).max(0);
                        needs_redraw = true;
                    } else if c == keyboard::KEY_DOWN {
                        app.scroll_y += 20;
                        needs_redraw = true;
                    }
                }
            }
        }
        
        if needs_redraw {
            // Toolbar background
            BrowserApp::fill(&mut buffer, width, height, 0, 0, width as i32, 40, 0xFFE0E0E0);
            BrowserApp::fill(&mut buffer, width, height, 0, 40, width as i32, 1, 0xFF808080);
            
            // URL Bar
            BrowserApp::fill(&mut buffer, width, height, 10, 10, width as i32 - 80, 20, 0xFFFFFFFF);
            BrowserApp::fill(&mut buffer, width, height, 10, 30, width as i32 - 80, 1, 0xFF808080);
            BrowserApp::fill(&mut buffer, width, height, 10, 10, 1, 20, 0xFF808080);
            
            let display_url = format!("{}|", app.url);
            BrowserApp::text(&mut buffer, width, height, 15, 12, &display_url, 0x000000);
            
            // Go Button
            BrowserApp::fill(&mut buffer, width, height, width as i32 - 60, 10, 50, 20, 0xFFC0C0C0);
            BrowserApp::text(&mut buffer, width, height, width as i32 - 45, 12, "Go", 0x000000);

            // Content Area
            BrowserApp::fill(&mut buffer, width, height, 0, 41, width as i32, height as i32 - 41, 0xFFFFFFFF);
            
            let mut text_y = 50 - app.scroll_y;
            for line in &app.content {
                if text_y > 40 && text_y < height as i32 {
                    BrowserApp::text(&mut buffer, width, height, 10, text_y, line, 0x000000);
                }
                text_y += 20;
            }

            needs_redraw = false;
            
            crate::gui::wm::send_message(crate::gui::wm::GuiMessage::UpdateBuffer {
                id,
                buffer_ptr: buffer.as_ptr() as u64,
            });
        }
        
        crate::process::scheduler::yield_now();
    }
}
