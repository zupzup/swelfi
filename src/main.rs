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
use std::sync::mpsc::{channel, Sender};
use std::{process::Command, sync::mpsc::Receiver};

const INTERFACE: &str = "Interface ";
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
}

enum Event {
    RefreshNetworks(egui::Context),
}

#[derive(Debug)]
enum Switch {
    On,
    Off,
}

#[derive(Debug)]
struct SwelfiApp {
    app_state: AppState,
    background_event_sender: Sender<Event>,
    event_receiver: Receiver<Event>,
}

#[derive(Debug)]
struct AppState {
    wlan_interfaces: Vec<WirelessInterface>,
    selected_wlan_interface: String,
    wlan_networks: Vec<WirelessNetwork>,
    selected_wlan_network: String,
    wlan_on: bool,
}

impl SwelfiApp {
    fn new(
        context: &eframe::CreationContext<'_>,
        app_state: AppState,
        background_event_sender: Sender<Event>,
        event_receiver: Receiver<Event>,
    ) -> Self {
        background_event_sender
            .send(Event::RefreshNetworks(context.egui_ctx.clone()))
            .unwrap();
        Self {
            app_state,
            background_event_sender,
            event_receiver,
        }
    }
}

impl eframe::App for SwelfiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.event_receiver.try_recv() {
            // TODO: handle event - update app state
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Swelfi");
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
                                ui.add(toggle(&mut self.app_state.wlan_on));
                                ui.add(egui::Label::new("Off"));
                            });
                            ui.end_row();

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new("Networks"));
                            });
                            ui.vertical(|ui| {
                                self.app_state.wlan_networks.iter().for_each(|wn| {
                                    ui.selectable_value(
                                        &mut self.app_state.selected_wlan_network,
                                        wn.id(),
                                        wn.id(),
                                    );
                                });
                            });
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
            match event {
                // TODO: handle event
                Event::RefreshNetworks(ctx) => ctx.request_repaint(),
            }
        }
    });

    let wlan_interfaces = iw()?;
    // let wlan_interfaces: Vec<WirelessInterface> = vec![WirelessInterface {
    //     name: String::from("tstintf"),
    // }]
    let selected_wlan_interface = wlan_interfaces[0].name.clone();

    let wlan_networks = scan_for_networks(&selected_wlan_interface)?;

    // let wlan_networks: Vec<WirelessNetwork> = vec![WirelessNetwork {
    //     essid: String::from("some network"),
    //     security_type: SecurityType::WPA2,
    //     frequency: 5.18,
    //     quality: Quality {
    //         value: 25,
    //         limit: 70,
    //     },
    //     address: String::from("AE:E2:D3:CC:59:F7"),
    // }];
    // let mut selected_wlan_network = wlan_networks[0].name.clone();
    let selected_wlan_network = wlan_networks[0].id();
    let app_state = AppState {
        wlan_interfaces,
        selected_wlan_interface,
        wlan_networks,
        selected_wlan_network,
        wlan_on: true,
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

pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}

// custom toggle from egui examples
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
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

fn _switch_wlan_interface(interface: &str, switch: Switch) -> Result<()> {
    let on_off = match switch {
        Switch::On => "up",
        Switch::Off => "down",
    };

    Command::new("ip")
        .args(["link", "set", interface, on_off])
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
    Ok((
        input,
        WirelessInterface {
            name: interface.to_owned(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_interface() {
        let input = " Interface wlp64s0\n";

        assert_eq!(
            parse_iw(input).unwrap().1,
            vec![WirelessInterface {
                name: String::from("wlp64s0")
            }]
        );
    }

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
                name: String::from("wlp64s0")
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
                    name: String::from("wlp64s0")
                },
                WirelessInterface {
                    name: String::from("second")
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
