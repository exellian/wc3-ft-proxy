use std::sync::{Arc, RwLock};
use std::net::{SocketAddr, IpAddr};
use std::sync::atomic::{AtomicBool, Ordering, AtomicU32};
use winapi::um::winuser;
use tokio::net::{UdpSocket, TcpStream};
use std::error::Error;
use tokio::task::JoinHandle;
use tokio::runtime::Runtime;
use std::time::{Duration, Instant};
use tokio::io;
use tokio::io::AsyncWriteExt;
use std::net;
use directories::ProjectDirs;
use std::fs::{OpenOptions};
use std::io::{Read, Write};

const DISCOVER_MESSAGE: &'static [u8] = b"\xf7\x2f\x10\x00\x50\x58\x33\x57\x18\x00\x00\x00\x00\x00\x00\x00";

fn get_default_addr() -> Option<IpAddr> {
    let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("1.1.1.1:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => Some(addr.ip()),
        Err(_) => None,
    }
}

fn show_error(message: &str, title: &str) {
    use std::ptr::null_mut as NULL;

    let mut msg = message.to_string();
    let mut title = title.to_string();
    msg.push('\0');
    title.push('\0');
    let l_msg: Vec<u16> = msg.encode_utf16().collect();
    let l_title: Vec<u16> = title.encode_utf16().collect();
    unsafe {
        winuser::MessageBoxW(NULL(), l_msg.as_ptr(), l_title.as_ptr(), winuser::MB_OK | winuser::MB_ICONERROR);
    }
}

fn get_saved_config() -> Option<SocketAddr> {

    if let Some(proj_dirs) = ProjectDirs::from("de", "wc3", "proxy") {
        let config = proj_dirs.config_dir();
        std::fs::create_dir_all(config).expect("Failed to create path structure");
        let config = config.join("config.txt");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config).expect("Failed to open config file!");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read the file");

        let parse = contents.parse::<SocketAddr>();
        if parse.is_err() {
            None
        } else {
            Some(parse.unwrap())
        }
    } else {
        None
    }
}

fn save_config(addr: SocketAddr) {
    if let Some(proj_dirs) = ProjectDirs::from("de", "wc3", "proxy") {
        let config = proj_dirs.config_dir();
        std::fs::create_dir_all(config).expect("Failed to create path structure");
        let config = config.join("config.txt");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config).unwrap();
        file.write_all(addr.to_string().as_bytes()).unwrap();
    }
}

pub struct Proxy {
    default_ip: IpAddr,
    old_udp: Arc<RwLock<Option<JoinHandle<()>>>>,
    old_tcp: Arc<RwLock<Option<JoinHandle<()>>>>,
    udp_runtime: tokio::runtime::Runtime,
    tcp_runtime: tokio::runtime::Runtime,
    udp_run: Arc<AtomicBool>,
    tcp_run: Arc<AtomicBool>,
    current_addr: Option<SocketAddr>
}

impl Proxy {

    pub fn new() -> Self {
        let config = get_saved_config();
        if let Some(default_ip) = get_default_addr() {
            Proxy {
                default_ip,
                old_udp: Arc::new(RwLock::new(None)),
                old_tcp: Arc::new(RwLock::new(None)),
                udp_runtime: tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
                tcp_runtime: tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
                udp_run: Arc::new(AtomicBool::new(false)),
                tcp_run: Arc::new(AtomicBool::new(false)),
                current_addr: config
            }
        } else {
            show_error("Failed to select a default network", "Error during network selection");
            panic!("Can't get default network interface!");
        }
    }

    pub fn get_current_addr(&self) -> Option<SocketAddr> {
        self.current_addr.clone()
    }


    pub fn stop_proxy(&mut self, mut logger: impl FnMut(String)) {

        let mut old_udp_guard = self.old_udp.write().unwrap();
        let mut old_tcp_guard = self.old_tcp.write().unwrap();

        if self.udp_run.load(Ordering::Relaxed) || self.tcp_run.load(Ordering::Relaxed) {
            logger(format!("[stop][udp]"));
            logger(format!("[stop][tcp]"));
            self.udp_run.swap(false, Ordering::Relaxed);
            self.tcp_run.swap(false, Ordering::Relaxed);

            let old_udp_handle = old_udp_guard.take();
            let old_tcp_handle = old_tcp_guard.take();

            if old_udp_handle.is_some() {
                futures::executor::block_on(old_udp_handle.unwrap()).unwrap();
            }
            if old_tcp_handle.is_some() {
                futures::executor::block_on(old_tcp_handle.unwrap()).unwrap();
            }
        }

    }

    pub fn on_address_change(&mut self, addr: SocketAddr, mut logger: impl FnMut(String)) {
        self.current_addr = Some(addr);
        logger(format!("[listen][udp][{}]", SocketAddr::new(self.default_ip, addr.port())));
        logger(format!("[listen][tcp][{}]", SocketAddr::new(self.default_ip, addr.port())));
        Self::udp_discoverer(&self.udp_runtime, self.old_udp.clone(), self.default_ip.clone(), addr, self.udp_run.clone()).unwrap();
        Self::tcp_proxy(&self.tcp_runtime, self.old_tcp.clone(), self.default_ip.clone(), addr, self.tcp_run.clone()).unwrap();
        save_config(addr);
        println!("Changed proxy config!");
    }

    fn tcp_proxy(runtime: &Runtime, old_handle: Arc<RwLock<Option<JoinHandle<()>>>>, default_ip: IpAddr, server_addr: SocketAddr, run: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
        let default_addr = SocketAddr::new(default_ip, server_addr.port());

        static IDS: AtomicU32 = AtomicU32::new(0);

        let mut handle_guard = old_handle.write().unwrap();

        let id= IDS.fetch_add(1, Ordering::Relaxed) + 1;
        run.swap(true, Ordering::Acquire);

        let mut handle = handle_guard.take();
        if handle.is_some() {
            futures::executor::block_on(handle.unwrap()).unwrap();
        }

        let task = runtime.spawn(async move {

            let outgoing= net::TcpListener::bind(default_addr).unwrap();
            outgoing.set_nonblocking(true).unwrap();
            let mut iter = outgoing.incoming();
            while IDS.load(Ordering::Relaxed) == id && run.load(Ordering::Relaxed) {
                match iter.next() {

                    Some(res) => match res {
                        Ok(stream) => {
                            let transfer = Self::tcp_transfer(TcpStream::from_std(stream).unwrap(), server_addr);
                            tokio::spawn(transfer);
                            continue;
                        }
                        Err(_) => {}
                    },
                    None => {}
                }
                tokio::time::sleep(Duration::from_millis(0)).await;
            }
        });

        handle = Some(task);
        std::mem::swap(&mut handle, &mut handle_guard);

        Ok(())
    }

    async fn tcp_transfer(mut inbound: TcpStream, proxy_addr: SocketAddr) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut outbound = TcpStream::connect(proxy_addr).await?;

        let (mut ri, mut wi) = inbound.split();
        let (mut ro, mut wo) = outbound.split();

        let client_to_server = async {
            io::copy(&mut ri, &mut wo).await?;
            wo.shutdown().await
        };

        let server_to_client = async {
            io::copy(&mut ro, &mut wi).await?;
            wi.shutdown().await
        };

        tokio::try_join!(client_to_server, server_to_client)?;

        Ok(())
    }

    fn udp_discoverer(runtime: &Runtime, old_handle: Arc<RwLock<Option<JoinHandle<()>>>>, default_ip: IpAddr, server_addr: SocketAddr, run: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {

        let default_addr = SocketAddr::new(default_ip, server_addr.port());

        static IDS: AtomicU32 = AtomicU32::new(0);

        let mut handle_guard = old_handle.write().unwrap();

        let id= IDS.fetch_add(1, Ordering::Relaxed) + 1;
        run.swap(true, Ordering::Acquire);

        let mut handle = handle_guard.take();
        if handle.is_some() {
            futures::executor::block_on(handle.unwrap()).unwrap();
        }

        let task = runtime.spawn(async move {

            let incoming = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            incoming.connect(server_addr).await.unwrap();

            let mut buf = [0; 1024];

            let mut now = Instant::now();

            while IDS.load(Ordering::Relaxed) == id && run.load(Ordering::Relaxed) {

                if now.elapsed().as_secs() > 2 {
                    incoming.try_send_to(DISCOVER_MESSAGE, server_addr).unwrap();
                    now = Instant::now();
                }

                let recv = incoming.try_recv_from(&mut buf);

                if recv.is_err() { // && recv_outgoing.is_err() {
                    tokio::time::sleep(Duration::from_millis(0)).await;
                } else {
                    if recv.is_ok() {
                        let (recv_len, addr)  = recv.unwrap();
                        println!("{:?} redirecting to client {:?} -> {:?}", recv_len, addr, default_addr);
                        incoming.try_send_to(&buf[..recv_len], default_addr).unwrap();
                    }
                }
            }
        });

        handle = Some(task);
        std::mem::swap(&mut handle, &mut handle_guard);

        Ok(())
    }
}
