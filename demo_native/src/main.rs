use demo::MyApp;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    eframe::run_native(
        "MyApp",
        Default::default(),
        Box::new(|_cc| {
            /*let ctx = &cc.egui_ctx;
            let mut fonts = eframe::egui::FontDefinitions::default();
            fonts.font_data.insert(
                "inter_medium".to_owned(),
                std::sync::Arc::new(eframe::egui::FontData::from_static(include_bytes!("../assets/fonts/inter/Inter-Medium.otf"))),
            ); // .ttf and .otf supported

            // Put the Inter Medium font first (highest priority):
            fonts.families.get_mut(&eframe::egui::FontFamily::Proportional).unwrap().insert(0, "inter_medium".to_owned());
            ctx.set_fonts(fonts);*/
            Ok(Box::new(MyApp::new(['⬆', '⬆'], '◀')))
        }),
    )
}
