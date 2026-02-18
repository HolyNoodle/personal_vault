use eframe::egui;
use std::fs;
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
        let current_path = PathBuf::from("/");
        let (items, error_message) = load_directory(&current_path);
        Self {
            search_query: String::new(),
            current_path,
            items,
            selected_index: None,
            error_message,
        }
    }
}

fn load_directory(path: &PathBuf) -> (Vec<FileItem>, Option<String>) {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut items: Vec<FileItem> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    FileItem {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        path: entry.path(),
                        is_dir,
                        size,
                    }
                })
                .collect();
            // Directories first, then files, both alphabetically
            items.sort_by(|a, b| {
                b.is_dir
                    .cmp(&a.is_dir)
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            (items, None)
        }
        Err(e) => (Vec::new(), Some(format!("Error reading {}: {}", path.display(), e))),
    }
}

impl eframe::App for FileExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                let mut navigate_to: Option<PathBuf> = None;

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
                    let icon = if item.is_dir { "[D]" } else { "[F]" };
                    let label = format!("{} {}", icon, item.name);
                    let response = ui.selectable_label(is_selected, &label);
                    if response.clicked() {
                        self.selected_index = Some(idx);
                    }
                    if response.double_clicked() && item.is_dir {
                        navigate_to = Some(item.path.clone());
                    }
                }

                if let Some(path) = navigate_to {
                    let (items, err) = load_directory(&path);
                    self.current_path = path;
                    self.items = items;
                    self.error_message = err;
                    self.selected_index = None;
                    self.search_query.clear();
                }
            });

            ui.separator();
            if let Some(idx) = self.selected_index {
                if let Some(item) = self.items.get(idx) {
                    let kind = if item.is_dir { "directory" } else { "file" };
                    ui.label(format!(
                        "Selected: {} ({}, {} bytes)",
                        item.name, kind, item.size
                    ));
                }
            }
            if let Some(ref err) = self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }
}
