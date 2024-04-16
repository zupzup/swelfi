use eframe::egui;
use std::process::Command;

#[derive(Debug)]
struct WirelessInterface {}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;
    let wlan_interfaces = ["wlan0", "wlan1", "wlan2"];
    let mut selected_wlan_interface = wlan_interfaces[0];

    eframe::run_simple_native("Swelfie", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Swelfie");
            ui.horizontal(|ui| {
                ui.add(egui::Label::new("Select WLAN Interface"));
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", selected_wlan_interface))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut selected_wlan_interface,
                            wlan_interfaces[0],
                            wlan_interfaces[0],
                        );
                        ui.selectable_value(
                            &mut selected_wlan_interface,
                            wlan_interfaces[1],
                            wlan_interfaces[1],
                        );
                        ui.selectable_value(
                            &mut selected_wlan_interface,
                            wlan_interfaces[2],
                            wlan_interfaces[2],
                        );
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
            ui.vertical(|ui| {
                ui.selectable_value(
                    &mut selected_wlan_interface,
                    wlan_interfaces[0],
                    wlan_interfaces[0],
                );
                ui.selectable_value(
                    &mut selected_wlan_interface,
                    wlan_interfaces[1],
                    wlan_interfaces[1],
                );
                ui.selectable_value(
                    &mut selected_wlan_interface,
                    wlan_interfaces[2],
                    wlan_interfaces[2],
                );
            });
        });
    })
}

fn get_wireless_interfaces() -> Vec<WirelessInterface> {
    vec![]
}
