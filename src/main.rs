use anyhow::{anyhow, Result};
use eframe::egui;
use nom::{
    bytes::complete::{tag, take_until, take_while},
    character::complete::{digit1, not_line_ending},
    multi::many0,
    number::complete::double,
    sequence::{delimited, tuple},
    IResult,
};
use std::{process::Command, sync::mpsc::Receiver};
use std::{
    sync::mpsc::{channel, Sender},
    time::Duration,
};

mod fps;

const INTERFACE: &str = "Interface ";
const SSID: &str = "ssid ";
const CELL: &str = "Cell ";
const FREQUENCY: &str = "Frequency:";
const QUALITY: &str = "Quality=";
const ESSID: &str = "ESSID:";
const IEEE: &str = "IEEE 802.11";

#[derive(Debug, Eq, PartialEq)]
enum SecurityType {
    Wpa2,
    Wpa3,
    Wpa,
    Invalid,
}

impl From<&str> for SecurityType {
    fn from(value: &str) -> Self {
        match value {
            v if v.contains("WPA2") => SecurityType::Wpa2,
            v if v.contains("WPA3") => SecurityType::Wpa3,
            v if v.contains("WPA") => SecurityType::Wpa,
            _ => SecurityType::Invalid,
        }
    }
}

#[derive(Debug, PartialEq)]
struct WirelessNetwork {
    pub address: String,
    pub quality: Quality,
    pub frequency: f64,
    pub essid: String,
    pub security_type: SecurityType,
}

impl WirelessNetwork {
    pub fn id(&self) -> String {
        format!("{} - ({})", self.essid, self.address)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Quality {
    pub value: u64,
    pub limit: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct WirelessInterface {
    pub name: String,
    pub connected_ssid: Option<String>,
}

enum Event {
    RefreshNetworks(egui::Context, String, Option<Duration>),
    UpdateNetworks(Option<Vec<WirelessNetwork>>),
}

struct SwelfiApp {
    app_state: AppState,
    background_event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
}

struct AppState {
    wlan_interfaces: Vec<WirelessInterface>,
    selected_wlan_interface: String,
    wlan_networks: Option<Vec<WirelessNetwork>>,
    selected_wlan_network: String,
    connected_wlan_network: Option<String>,
    wlan_on: bool,
    frame_history: fps::FrameHistory,
}

impl SwelfiApp {
    fn new(
        context: &eframe::CreationContext<'_>,
        app_state: AppState,
        background_event_sender: Sender<Event>,
        event_receiver: Receiver<Event>,
    ) -> Self {
        background_event_sender
            .send(Event::RefreshNetworks(
                context.egui_ctx.clone(),
                app_state.selected_wlan_interface.clone(),
                None,
            ))
            .expect("can send on channel");
        log::info!("sent event...waiting");
        Self {
            app_state,
            background_event_sender,
            event_receiver,
        }
    }
}

impl eframe::App for SwelfiApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app_state
            .frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
        while let Ok(event) = self.event_receiver.try_recv() {
            if let Event::UpdateNetworks(networks) = event {
                if let Ok(connected_wlan_network) =
                    get_connected_network_ssid(&self.app_state.selected_wlan_interface)
                {
                    self.app_state.connected_wlan_network = connected_wlan_network;
                }
                if let Some(ref networks) = networks {
                    if !networks.is_empty() {
                        self.app_state.selected_wlan_network = networks[0].id()
                    };
                }
                self.app_state.wlan_networks = networks;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Swelfi");
            ui.label(format!("FPS: {:.1}", self.app_state.frame_history.fps()));
            ui.label(format!(
                "Mean CPU usage: {:.2} ms / frame",
                1e3 * self.app_state.frame_history.mean_frame_time()
            ));
            egui::Grid::new("structure")
                .num_columns(2)
                .spacing([20.0, 20.0])
                .show(ui, |ui| {
                    egui::Grid::new("interfaces and networks")
                        .num_columns(2)
                        .spacing([20.0, 20.0])
                        .min_col_width(80.0)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new("WLAN Interface"));
                            egui::ComboBox::from_id_source("wlan interfaces")
                                .selected_text(&self.app_state.selected_wlan_interface)
                                .show_ui(ui, |ui| {
                                    self.app_state.wlan_interfaces.iter().for_each(|wi| {
                                        ui.selectable_value(
                                            &mut self.app_state.selected_wlan_interface,
                                            wi.name.clone(),
                                            wi.name.clone(),
                                        );
                                    });
                                });
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new("On"));
                                ui.add(toggle(
                                    &mut self.app_state,
                                    self.background_event_sender.clone(),
                                    ctx.clone(),
                                ));
                                ui.add(egui::Label::new("Off"));
                            });
                            ui.end_row();

                            ui.with_layout(egui::Layout::top_down(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new("Networks"));
                                if ui.button("refresh").clicked() {
                                    self.background_event_sender
                                        .send(Event::RefreshNetworks(
                                            ctx.clone(),
                                            self.app_state.selected_wlan_interface.clone(),
                                            None,
                                        ))
                                        .expect("can send on channel");
                                }
                            });
                            ui.vertical(|ui| {
                                ui.set_width(250.0);
                                let connected_wlan_network = &self.app_state.connected_wlan_network;
                                if let Some(ref networks) = self.app_state.wlan_networks {
                                    networks.iter().for_each(|wn| {
                                        let mut id = wn.id();
                                        if let Some(connected_wlan_network) =
                                            &connected_wlan_network
                                        {
                                            if wn.essid == *connected_wlan_network {
                                                id = format!("{} - connected", id);
                                            }
                                        }
                                        ui.selectable_value(
                                            &mut self.app_state.selected_wlan_network,
                                            id.clone(),
                                            id,
                                        );
                                    });
                                } else {
                                    ui.spinner();
                                }
                            });
                            ui.end_row();
                        });
                });
        });
    }
}

fn main() -> Result<()> {
    env_logger::init();

    // spawn thread for background actions
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_always_on_top()
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    let (background_event_sender, background_event_receiver) = channel::<Event>();
    let (event_sender, event_receiver) = channel::<Event>();

    std::thread::spawn(move || {
        while let Ok(event) = background_event_receiver.recv() {
            if let Event::RefreshNetworks(ctx, selected_wlan_interface, wait_time) = event {
                event_sender
                    .send(Event::UpdateNetworks(None))
                    .expect("can send on channel");

                if let Some(wait) = wait_time {
                    std::thread::sleep(wait);
                }

                match scan_for_networks(&selected_wlan_interface) {
                    Ok(networks) => {
                        event_sender
                            .send(Event::UpdateNetworks(Some(networks)))
                            .expect("can send on channel");
                        ctx.request_repaint();
                    }
                    Err(e) => log::error!("Error while scanning for networks: {}", e),
                }
            }
        }
    });

    let wlan_interfaces = iw()?;
    if wlan_interfaces.is_empty() {
        panic!("There is no wlan interface")
    }
    let selected_wlan_interface = wlan_interfaces[0].name.clone(); // this is safe, because we
                                                                   // check for at least one
                                                                   // interface above

    let selected_wlan_network = String::new();
    let connected_wlan_network = get_connected_network_ssid(&selected_wlan_interface)?;
    let app_state = AppState {
        wlan_interfaces,
        selected_wlan_interface,
        wlan_networks: None,
        selected_wlan_network,
        connected_wlan_network,
        wlan_on: true,
        frame_history: fps::FrameHistory::default(),
    };

    eframe::run_native(
        "Swelfi",
        options,
        Box::new(|context| {
            Box::new(SwelfiApp::new(
                context,
                app_state,
                background_event_sender,
                event_receiver,
            ))
        }),
    )
    .map_err(|e| anyhow!("eframe error: {}", e))
}

fn toggle(
    app_state: &mut AppState,
    background_sender: Sender<Event>,
    ctx: egui::Context,
) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, app_state, background_sender, ctx)
}

// custom toggle from egui examples
fn toggle_ui(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    background_sender: Sender<Event>,
    ctx: egui::Context,
) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        // set new value
        app_state.wlan_on = !app_state.wlan_on;

        // if we set the interface to off, we clear the list
        if !app_state.wlan_on {
            app_state.wlan_networks = Some(vec![]);
        }
        response.mark_changed();
        match switch_wlan_interface(&app_state.selected_wlan_interface, app_state.wlan_on) {
            Ok(_) => {
                if app_state.wlan_on {
                    background_sender
                        .send(Event::RefreshNetworks(
                            ctx,
                            app_state.selected_wlan_interface.to_owned(),
                            Some(Duration::from_millis(1000)), // wait for interface to come up
                                                               // before scanning
                        ))
                        .expect("can send on channel");
                }
            }
            Err(e) => log::error!("Error while switching wifi on, or off: {}", e),
        }
    }
    let on = app_state.wlan_on;

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, on);
        let visuals = ui.style().interact_selectable(&response, on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter()
            .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

fn scan_for_networks(interface: &str) -> Result<Vec<WirelessNetwork>> {
    let output = Command::new("sudo")
        .args(["iwlist", interface, "s"])
        .output()?;
    if !output.status.success() {
        return Err(anyhow!("getting wireless interfaces using 'iwlist' failed"));
    }

    std::str::from_utf8(&output.stdout)
        .map(|out_str| {
            parse_nw(out_str)
                .map(|(_, wlan_networks)| wlan_networks)
                .map_err(|e| anyhow!("parsing 'iwlist' output failed: {}", e))
        })
        .map_err(|_| anyhow!("output of 'iwlist' wasn't valid utf-8"))?
}

fn get_connected_network_ssid(selected_interface: &str) -> Result<Option<String>> {
    let output = Command::new("iw")
        .args(["dev", selected_interface, "info"])
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "getting wireless interface info for {} using 'iw' failed",
            selected_interface
        ));
    }

    std::str::from_utf8(&output.stdout)
        .map(|out_str| {
            interface(out_str)
                .map(|(_, wlan_interface)| wlan_interface.connected_ssid)
                .map_err(|e| anyhow!("parsing 'iw' output failed: {}", e))
        })
        .map_err(|_| anyhow!("output of 'iw' wasn't valid utf-8"))?
}

fn switch_wlan_interface(interface: &str, on: bool) -> Result<()> {
    let on_off = if on { "up" } else { "down" };

    Command::new("sudo")
        .args(["ip", "link", "set", interface, on_off])
        .output()?;
    Ok(())
}

fn parse_nw(input: &str) -> IResult<&str, Vec<WirelessNetwork>> {
    many0(cell)(input)
}

fn cell(input: &str) -> IResult<&str, WirelessNetwork> {
    let (input, (address, frequency, quality, essid, security_type)) = tuple((
        network_address,
        network_frequency,
        network_quality,
        network_essid,
        network_security_type,
    ))(input)?;

    log::info!(
        "########## address: {}, frequency: {}, quality: {:?}, essid: {}, security_type: {:?} ##########",
        address,
        frequency,
        quality,
        essid,
        security_type
    );

    Ok((
        input,
        WirelessNetwork {
            essid: essid.to_owned(),
            address: address.to_owned(),
            security_type,
            frequency,
            quality,
        },
    ))
}

fn network_address(input: &str) -> IResult<&str, &str> {
    tuple((
        take_until::<_, _, nom::error::Error<_>>(CELL),
        tag(CELL),
        digit1,
        tag(" - Address: "),
        take_until("\n"),
    ))(input)
    .map(|(inp, (_, _, _, _, address))| Ok((inp, address)))?
}

fn network_frequency(input: &str) -> IResult<&str, f64> {
    tuple((
        take_until::<_, _, nom::error::Error<_>>(FREQUENCY),
        tag(FREQUENCY),
        double,
    ))(input)
    .map(|(inp, (_, _, frequency))| Ok((inp, frequency)))?
}

fn network_quality(input: &str) -> IResult<&str, Quality> {
    tuple((
        take_until::<_, _, nom::error::Error<_>>(QUALITY),
        tag(QUALITY),
        digit1,
        tag("/"),
        digit1,
    ))(input)
    .map(|(inp, (_, _, quality_value, _, quality_limit))| {
        Ok((
            inp,
            Quality {
                value: quality_value
                    .parse::<u64>()
                    .expect("quality value is a number"),
                limit: quality_limit
                    .parse::<u64>()
                    .expect("quality value is a number"),
            },
        ))
    })?
}

fn network_essid(input: &str) -> IResult<&str, &str> {
    tuple((
        take_until::<_, _, nom::error::Error<_>>(ESSID),
        tag(ESSID),
        delimited(tag("\""), take_while(|c| c != '"'), tag("\"")),
    ))(input)
    .map(|(inp, (_, _, essid))| Ok((inp, essid)))?
}

fn network_security_type(input: &str) -> IResult<&str, SecurityType> {
    tuple((
        take_until::<_, _, nom::error::Error<_>>(IEEE),
        tag(IEEE),
        not_line_ending,
    ))(input)
    .map(|(inp, (_, _, security_type_line))| Ok((inp, SecurityType::from(security_type_line))))?
}

fn iw() -> Result<Vec<WirelessInterface>> {
    let output = Command::new("iw").args(["dev"]).output()?;
    if !output.status.success() {
        return Err(anyhow!("getting wireless interfaces using 'iw' failed"));
    }

    std::str::from_utf8(&output.stdout)
        .map(|out_str| {
            parse_iw(out_str)
                .map(|(_, wlan_interfaces)| wlan_interfaces)
                .map_err(|e| anyhow!("parsing 'iw' output failed: {}", e))
        })
        .map_err(|_| anyhow!("output of 'iw' wasn't valid utf-8"))?
}

fn parse_iw(input: &str) -> IResult<&str, Vec<WirelessInterface>> {
    many0(interface)(input)
}

fn interface(input: &str) -> IResult<&str, WirelessInterface> {
    let (input, (_, _, interface)) =
        tuple((take_until(INTERFACE), tag(INTERFACE), take_until("\n")))(input)?;
    let (input, (_, _, connected_ssid)) =
        tuple((take_until(SSID), tag(SSID), take_until("\n")))(input)?; // TODO: make optional
    Ok((
        input,
        WirelessInterface {
            name: interface.to_owned(),
            connected_ssid: Some(connected_ssid.to_owned()),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_interface() {
        let input = "phy#0
	Interface wlp64s0
		ifindex 3
		wdev 0x1
		addr 9c:fc:e8:b8:fa:60
		ssid whatever
		type managed
		";

        assert_eq!(
            parse_iw(input).unwrap().1,
            vec![WirelessInterface {
                name: String::from("wlp64s0"),
                connected_ssid: Some(String::from("whatever"))
            }]
        );
    }
    // TODO: test without ssid

    #[test]
    fn basic_interface() {
        let input = "phy#0
	Unnamed/non-netdev interface
		wdev 0x2
		addr 9c:fc:e8:b8:fa:61
		type P2P-device
	Interface wlp64s0
		ifindex 3
		wdev 0x1
		addr 9c:fc:e8:b8:fa:60
		ssid whatever
		type managed
		channel 100 (5500 MHz), width: 80 MHz, center1: 5530 MHz
		txpower 22.00 dBm
		multicast TXQ:
			qsz-byt	qsz-pkt	flows	drops	marks	overlmt	hashcol	tx-bytes	tx-packets
			0	0	0	0	0	0	0	0		0
            ";

        assert_eq!(
            parse_iw(input).unwrap().1,
            vec![WirelessInterface {
                name: String::from("wlp64s0"),
                connected_ssid: Some(String::from("whatever")),
            }]
        );
    }

    #[test]
    fn two_interfaces() {
        let input = "phy#0
	Unnamed/non-netdev interface
		wdev 0x2
		addr 9c:fc:e8:b8:fa:61
		type P2P-device
	Interface wlp64s0
		ifindex 3
		wdev 0x1
		addr 9c:fc:e8:b8:fa:60
		ssid whatever
		type managed
		channel 100 (5500 MHz), width: 80 MHz, center1: 5530 MHz
		txpower 22.00 dBm
		multicast TXQ:
			qsz-byt	qsz-pkt	flows	drops	marks	overlmt	hashcol	tx-bytes	tx-packets
			0	0	0	0	0	0	0	0		0
	Interface second
		ifindex 3
		wdev 0x1
		addr 9c:fc:e8:b8:fa:60
		ssid whatever
		type managed
		channel 100 (5500 MHz), width: 80 MHz, center1: 5530 MHz
		txpower 22.00 dBm
		multicast TXQ:
			qsz-byt	qsz-pkt	flows	drops	marks	overlmt	hashcol	tx-bytes	tx-packets
			0	0	0	0	0	0	0	0		0
            ";

        assert_eq!(
            parse_iw(input).unwrap().1,
            vec![
                WirelessInterface {
                    name: String::from("wlp64s0"),
                    connected_ssid: Some(String::from("whatever")),
                },
                WirelessInterface {
                    name: String::from("second"),
                    connected_ssid: Some(String::from("whatever")),
                }
            ]
        );
    }

    #[test]
    fn no_device_interface() {
        let input = "phy#0
	Unnamed/non-netdev interface
		wdev 0x2
		addr 9c:fc:e8:b8:fa:61
		type P2P-device";

        assert_eq!(parse_iw(input).unwrap().1, vec![]);
    }

    #[test]
    fn invalid_interface() {
        let input = "fdsfasdjlhflasjdfhklajshf kasdj";

        assert_eq!(parse_iw(input).unwrap().1, vec![]);
    }

    #[test]
    fn valid_network() {
        let input = "Cell 09 - Address: D4:1A:D1:51:67:F2
                    Channel:6
                    Frequency:2.437 GHz (Channel 6)
                    Quality=42/70  Signal level=-68 dBm
                    Encryption key:on
                    ESSID:\"some network\"
                    Bit Rates:1 Mb/s; 2 Mb/s; 5.5 Mb/s; 11 Mb/s; 18 Mb/s
                              24 Mb/s; 36 Mb/s; 54 Mb/s
                    Bit Rates:6 Mb/s; 9 Mb/s; 12 Mb/s; 48 Mb/s
                    Mode:Master
                    Extra:tsf=00000052cabe36b9
                    Extra: Last beacon: 2216ms ago
                    IE: Unknown: 00086D696E6B616E6574
                    IE: Unknown: 010882848B962430486C
                    IE: Unknown: 030106
                    IE: Unknown: 0706415420010D14
                    IE: Unknown: 200100
                    IE: Unknown: 23021000
                    IE: Unknown: 2A0104
                    IE: Unknown: 32040C121860
                    IE: IEEE 802.11i/WPA2 Version 1
                        Group Cipher : CCMP
                        Pairwise Ciphers (1) : CCMP
                        Authentication Suites (1) : PSK
                    IE: Unknown: 0B050000130000
                    IE: Unknown: 2D1ABC091BFFFF000000000000000000000000000000000000000000
                    IE: Unknown: 3D1606080000000000000000000000000000000000000000
                    IE: Unknown: 7F080400080000000040
                    IE: Unknown: DD880050F204104A0001101044000102103B00010310470010F1C8F0ECA8220A216584CCEC11054672102100055A5958454C102300094458333130312D4230102400094458333130312D423010420004313233341054000800060050F2040001101100114458333130312D4230205A7958454C4150100800022008103C0001031049000600372A000120
                    IE: Unknown: DD090010180200000C0000
                    IE: Unknown: DD180050F2020101840003A4000027A4000042435E0062322F00";

        assert_eq!(
            parse_nw(input).unwrap().1,
            vec![WirelessNetwork {
                essid: String::from("some network"),
                security_type: SecurityType::Wpa2,
                frequency: 2.437,
                quality: Quality {
                    value: 42,
                    limit: 70,
                },
                address: String::from("D4:1A:D1:51:67:F2"),
            }]
        );
    }
}
