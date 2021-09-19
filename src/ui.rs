use std::net::{SocketAddr, IpAddr};
use std::sync::{Arc, RwLock};

use druid::widget::{Label, TextBox, Flex};
use druid::{Color, Data, Env, Lens, LocalizedString, Widget, WidgetExt, WindowId, AppDelegate, Event, DelegateCtx};
use crate::wc3proxy::Proxy;

pub const WINDOW_TITLE: LocalizedString<AppState> = LocalizedString::new("Warcraft III Proxy");

#[derive(Clone, Data, Lens)]
pub struct AppState {
    ip: String,
    port: String,
    old_addr: Arc<Option<SocketAddr>>,
    log: Arc<Vec<String>>,
    state: Arc<RwLock<Proxy>>
}

impl AppState {
    pub fn new(ip: String, port: String, old_addr: Arc<Option<SocketAddr>>, log: Arc<Vec<String>>, state: Arc<RwLock<Proxy>>) -> Self {
        AppState {
            ip,
            port,
            old_addr,
            log,
            state
        }
    }
}

fn println_log(state: &Arc<Vec<String>>, str: &str) -> Vec<String> {
    let mut new_lines = Vec::new();
    let mut i = 0;
    for line in state.iter() {
        if i >= 2 {
            break;
        }
        new_lines.push(line.into());
        i += 1;
    }
    new_lines.insert(0, str.to_string());
    new_lines
}

pub struct AppEvents;
impl AppDelegate<AppState> for AppEvents {

    fn event(&mut self, _: &mut DelegateCtx<'_>, _: WindowId, event: Event, data: &mut AppState, _: &Env) -> Option<Event> {
        match &event {
            Event::WindowConnected => {
                let mut log = data.log.clone();
                let logger = |str: String| {
                    log = Arc::new(println_log(&log, &str));
                };
                {
                    let mut data_guard = data.state.write().unwrap();
                    let current = data_guard.get_current_addr();
                    if current.is_some() {
                        let current = current.unwrap();
                        data.ip = current.ip().to_string();
                        data.port = current.port().to_string();
                        data_guard.on_address_change(current, logger);
                    }
                }
                data.log = log;
            },
            Event::KeyUp(_) => {
                let port = data.port.parse::<u16>();
                let ip = data.ip.parse::<IpAddr>();
                let mut log = data.log.clone();
                let logger = |str: String| {
                    log = Arc::new(println_log(&log, &str));
                };

                if port.is_ok() && ip.is_ok() && port.as_ref().unwrap() > &1023 {
                    let sockt_addr = SocketAddr::new(ip.unwrap(), port.unwrap());

                    if data.old_addr.is_none() || data.old_addr.unwrap() != sockt_addr {
                        data.old_addr = Some(sockt_addr).into();
                        {
                            let mut data_guard = data.state.write().unwrap();
                            data_guard.on_address_change(sockt_addr, logger);
                        }
                    } else {
                        {
                            let mut data_guard = data.state.write().unwrap();
                            data_guard.stop_proxy(logger);
                        }
                    }

                } else {
                    if data.old_addr.is_some() {
                        {
                            let mut data_guard = data.state.write().unwrap();
                            data_guard.stop_proxy(logger);
                        }
                    }
                }
                data.log = log;
            },
            _ => {}
        }
        Some(event)
    }
}

pub fn build_root_widget() -> impl Widget<AppState> {

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(
            Label::new("Server-Address: "),
        )
        .with_spacer(5.0)
        .with_child(Flex::row()
            .with_flex_child(TextBox::new()
                                 .lens(AppState::ip)
                                 .padding((0.0, 0.0, 5.0, 0.0))
                                 .expand_width(),0.7)
            .with_flex_child(TextBox::new()
                                 .lens(AppState::port)
                                 .expand_width(), 0.3)
        )

        .with_spacer(6.0)
        .with_flex_child(Label::new(|data: &Arc<Vec<String>>, _env: &_| {
            let mut str = "".to_string();
            for line in data.iter() {
                str.push_str(&format!("{}\n", line))
            }
            str
        })
                             .with_line_break_mode(druid::widget::LineBreaking::WordWrap)
                             .lens(AppState::log)
                             .expand_width()
                             .expand_height()
                             .background(Color::grey(0.1))
                             .rounded(5.0)
                         ,
                         1.0)
        .padding(20.0)
}
