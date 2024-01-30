use std::sync::{Arc, RwLock};
use crate::LogFileHandler;



pub struct LogInfoWindow {
    enabled: bool,
    DMC: String,
    report: Vec<String>,

    search_bar: String
}

impl LogInfoWindow {
    pub fn default() -> Self {
        Self {
            enabled: false,
            DMC: String::new(),
            report: Vec::new(),
            search_bar: String::new()
        }
    }

    pub fn open(&mut self, target_DMC: String, lfh: Arc<RwLock<LogFileHandler>>) {
        if let Some(report) = lfh.read().unwrap().get_report_for_DMC(&target_DMC) {
            self.enabled = true;
            self.DMC = target_DMC.clone();
            self.search_bar = target_DMC;
            self.report = report;
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn update(&mut self, ctx: &egui::Context, lfh: Arc<RwLock<LogFileHandler>>) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of(self.DMC.clone()),
            egui::ViewportBuilder::default()
                .with_title(self.DMC.clone())
                .with_inner_size([400.0, 400.0]),
            |ctx, class| {

                assert!(
                    class == egui::ViewportClass::Immediate,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().show(ctx, |ui| {

                    ui.horizontal( |ui| {
                        ui.monospace("DMC:");

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_bar).desired_width(300.0)
                        );

                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Some(report) = lfh.read().unwrap().get_report_for_DMC(&self.search_bar) {
                                self.DMC = self.search_bar.clone();
                                self.report = report;
                            }
                        }
                    });

                    ui.separator();


                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| {
                            for rpt in self.report.iter() {
                                //ui.label(rpt);
                                ui.text_edit_singleline( &mut rpt.as_str());
                            }
                        });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    self.enabled = false;
                }
            },
        );
    }
}