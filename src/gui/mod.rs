mod app;
mod editor;
mod state;
mod themes;

pub use app::QuickNotepadApp;

// Entry point for GUI mode
pub fn run(file_path: Option<String>) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    
    let _ = eframe::run_native(
        "Quick Notepad",
        options,
        Box::new(move |cc| {
            // Setup custom fonts if needed
            setup_custom_fonts(&cc.egui_ctx);
            
            Ok(Box::new(QuickNotepadApp::new(cc, file_path)))
        }),
    );
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    
    // Add monospace font for editor
    // You can add custom fonts here if you want
    
    ctx.set_fonts(fonts);
}

fn load_icon() -> egui::IconData {
    // Load icon from embedded resource or file
    egui::IconData {
        rgba: vec![0; 4],
        width: 1,
        height: 1,
    }
}