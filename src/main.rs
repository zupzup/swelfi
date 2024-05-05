use anyhow::{anyhow, Result};
use eframe::egui;
use nom::{
    bytes::complete::{tag, take_until},
    character::complete::multispace0,
    combinator::map,
    multi::many0,
    sequence::preceded,
    IResult,
};
use std::process::Command;

#[derive(Debug, Eq, PartialEq)]
struct WirelessNetwork {
    pub essid: String,
    pub security_type: String,
    pub frequency: String,
    pub quality: String,
    pub address: String,
}

#[derive(Debug, Eq, PartialEq)]
struct WirelessInterface {
    pub name: String,
}

#[derive(Debug)]
enum Switch {
    On,
    Off,
}

fn main() -> Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_always_on_top()
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    let wlan_interfaces = iw()?;
    // let wlan_interfaces: Vec<WirelessInterface> = vec![WirelessInterface {
    //     name: String::from("tstintf"),
    // }]
    let mut selected_wlan_interface = wlan_interfaces[0].name.clone();

    let wlan_networks = scan_for_networks(&selected_wlan_interface)?;
    let wlan_networks: Vec<WirelessNetwork> = vec![WirelessNetwork {
        essid: String::from("some network"),
        security_type: String::from("IEEE 802.11i/WPA2 Version 1"),
        frequency: String::from("5.18 GHz (Channel 36)"),
        quality: String::from("25/70  Signal level=-85 dBm"),
        address: String::from("AE:E2:D3:CC:59:F7"),
    }];
    // let mut selected_wlan_network = wlan_networks[0].name.clone();
    let mut selected_wlan_network = wlan_networks[0].essid.clone();

    let mut wlan_on = true;

    eframe::run_simple_native("Swelfi", options, move |ctx, _frame| {
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
                                .selected_text(&selected_wlan_interface)
                                .show_ui(ui, |ui| {
                                    wlan_interfaces.iter().for_each(|wi| {
                                        ui.selectable_value(
                                            &mut selected_wlan_interface,
                                            wi.name.clone(),
                                            wi.name.clone(),
                                        );
                                    });
                                });
                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new("On"));
                                ui.add(toggle(&mut wlan_on));
                                ui.add(egui::Label::new("Off"));
                            });
                            ui.end_row();

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                                ui.add(egui::Label::new("Networks"));
                            });
                            ui.vertical(|ui| {
                                wlan_networks.iter().for_each(|wn| {
                                    ui.selectable_value(
                                        &mut selected_wlan_network,
                                        wn.essid.clone(),
                                        wn.essid.clone(),
                                    );
                                });
                            });
                        });
                });
        });
    })
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
    let output = Command::new("iwlist").args([interface, "s"]).output()?;
    if output.status.success() {
        if let Ok(out_str) = std::str::from_utf8(&output.stdout) {
            if let Ok((_, wlan_networks)) = parse_nw(out_str) {
                return Ok(wlan_networks);
            }
        }
    }
    Err(anyhow!("getting wireless networks using 'iwlist' failed"))
}

// TODO: get essid, refactor
fn parse_nw(input: &str) -> IResult<&str, Vec<WirelessNetwork>> {
    many0(cell)(input)
}

fn cell(input: &str) -> IResult<&str, WirelessNetwork> {
    Ok((
        "",
        WirelessNetwork {
            essid: String::from("some network"),
            security_type: String::from("IEEE 802.11i/WPA2 Version 1"),
            frequency: String::from("5.18 GHz (Channel 36)"),
            quality: String::from("25/70  Signal level=-85 dBm"),
            address: String::from("AE:E2:D3:CC:59:F7"),
        },
    ))
}

fn switch_wlan_interface(interface: &str, switch: Switch) -> Result<()> {
    let on_off = match switch {
        Switch::On => "up",
        Switch::Off => "down",
    };

    Command::new("ip")
        .args(["link", "set", interface, on_off])
        .output()?;
    Ok(())
}

fn iw() -> Result<Vec<WirelessInterface>> {
    let output = Command::new("iw").args(["dev"]).output()?;
    if output.status.success() {
        if let Ok(out_str) = std::str::from_utf8(&output.stdout) {
            if let Ok((_, wlan_interfaces)) = parse_iw(out_str) {
                return Ok(wlan_interfaces);
            }
        }
    }
    Err(anyhow!("getting wireless interfaces using 'iw' failed"))
}

fn parse_iw(input: &str) -> IResult<&str, Vec<WirelessInterface>> {
    many0(interface)(input)
}

fn interface(input: &str) -> IResult<&str, WirelessInterface> {
    let (input, _) = take_until("Interface ")(input)?;
    let (input, interface) = preceded(tag("Interface "), take_until("\n"))(input)?;
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
                security_type: String::from("IEEE 802.11i/WPA2 Version 1"),
                frequency: String::from("5.18 GHz (Channel 36)"),
                quality: String::from("25/70  Signal level=-85 dBm"),
                address: String::from("AE:E2:D3:CC:59:F7"),
            }]
        );
    }
}
