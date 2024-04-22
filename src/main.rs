use anyhow::{anyhow, Result};
use eframe::egui;
use regex_lite::Regex;
use std::process::Command;

#[derive(Debug, Eq, PartialEq)]
struct WirelessNetwork {
    pub name: String,
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
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let wlan_interfaces = iw()?;
    let mut selected_wlan_interface = wlan_interfaces[0].name.clone();

    let mut name = "Arthur".to_owned();
    let mut age = 42;
    let mut wlan_on = true;

    eframe::run_simple_native("Swelfi", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Swelfi");
            egui::Grid::new("")
                .num_columns(2)
                .spacing([20.0, 20.0])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("WLAN Interface"));
                        egui::ComboBox::from_label("")
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
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new("On"));
                        ui.add(toggle(&mut wlan_on));
                        ui.add(egui::Label::new("Off"));
                    });
                });
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hello '{name}', age {age}"));
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
        return Ok(parse_nw(&output.stdout));
    }
    Err(anyhow!("getting wireless interfaces using 'iw' failed"))
}

// TODO: get essid, refactor
fn parse_nw(output: &[u8]) -> Vec<WirelessNetwork> {
    let re = Regex::new(r"Cell (\w+)").unwrap();
    if let Ok(str) = String::from_utf8(output.to_owned()) {
        return re
            .captures_iter(&str)
            .map(|cap| {
                let (_, [name]) = cap.extract();
                WirelessNetwork {
                    name: name.to_owned(),
                }
            })
            .collect::<Vec<WirelessNetwork>>();
    } else {
        vec![]
    }
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
        return Ok(parse_iw(&output.stdout));
    }
    Err(anyhow!("getting wireless interfaces using 'iw' failed"))
}

fn parse_iw(output: &[u8]) -> Vec<WirelessInterface> {
    let re = Regex::new(r"Interface (\w+)").unwrap();
    if let Ok(str) = String::from_utf8(output.to_owned()) {
        return re
            .captures_iter(&str)
            .map(|cap| {
                let (_, [name]) = cap.extract();
                WirelessInterface {
                    name: name.to_owned(),
                }
            })
            .collect::<Vec<WirelessInterface>>();
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
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
            parse_iw(input.as_bytes()),
            vec![WirelessInterface {
                name: String::from("wlp64s0")
            }]
        );
    }

    #[test]
    fn no_device() {
        let input = "phy#0
	Unnamed/non-netdev interface
		wdev 0x2
		addr 9c:fc:e8:b8:fa:61
		type P2P-device";

        assert_eq!(parse_iw(input.as_bytes()), vec![]);
    }

    #[test]
    fn invalid() {
        let input = "fdsfasdjlhflasjdfhklajshf kasdj";

        assert_eq!(parse_iw(input.as_bytes()), vec![]);
    }
}
