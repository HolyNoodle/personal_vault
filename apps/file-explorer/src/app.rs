use egui;
use std::path::PathBuf;

#[derive(Default)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub struct FileExplorerApp {
    pub search_query: String,
    pub current_path: PathBuf,
    pub items: Vec<FileItem>,
    pub selected_index: Option<usize>,
    pub error_message: Option<String>,
}

impl Default for FileExplorerApp {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            current_path: PathBuf::from("/"),
            items: vec![
                FileItem {
                    name: "Documents".to_string(),
                    path: PathBuf::from("/Documents"),
                    is_dir: true,
                    size: 0,
                },
                FileItem {
                    name: "Pictures".to_string(),
                    path: PathBuf::from("/Pictures"),
                    is_dir: true,
                    size: 0,
                },
                FileItem {
                    name: "example.txt".to_string(),
                    path: PathBuf::from("/example.txt"),
                    is_dir: false,
                    size: 1234,
                },
                FileItem {
                    name: "readme.md".to_string(),
                    path: PathBuf::from("/readme.md"),
                    is_dir: false,
                    size: 567,
                },
            ],
            selected_index: None,
            error_message: None,
        }
    }
}

impl FileExplorerApp {
    pub fn show(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("File Explorer");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
            });
            ui.separator();
            ui.label(format!("Path: {}", self.current_path.display()));
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, item) in self.items.iter().enumerate() {
                    if !self.search_query.is_empty()
                        && !item
                            .name
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                    {
                        continue;
                    }
                    let is_selected = self.selected_index == Some(idx);
                    let icon = if item.is_dir { "D" } else { "F" };
                    let label = format!("[{}] {}", icon, item.name);
                    let response = ui.selectable_label(is_selected, &label);
                    if response.clicked() {
                        self.selected_index = Some(idx);
                    }
                }
            });
            ui.separator();
            if let Some(idx) = self.selected_index {
                if let Some(item) = self.items.get(idx) {
                    ui.label(format!("Selected: {} ({} bytes)", item.name, item.size));
                }
            }
            if let Some(ref err) = self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }
}

pub fn create_file_explorer_app() -> FileExplorerApp {
    FileExplorerApp::default()
}
