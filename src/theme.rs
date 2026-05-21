use eframe::egui::{Color32, Visuals, style::Selection};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Lite,
    Dark,
    Cli,
    Cyanogen,
    Cyberpunk,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

impl Theme {
    pub fn all() -> &'static [Theme] {
        &[Theme::Lite, Theme::Dark, Theme::Cli, Theme::Cyanogen, Theme::Cyberpunk]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Lite => "Lite",
            Theme::Dark => "Dark",
            Theme::Cli => "CLI",
            Theme::Cyanogen => "Cyanogen",
            Theme::Cyberpunk => "Cyberpunk",
        }
    }

    pub fn visuals(&self) -> Visuals {
        let mut v = match self {
            Theme::Lite => Visuals::light(),
            Theme::Dark => Visuals::dark(),
            Theme::Cli => {
                let mut v = Visuals::dark();
                v.window_fill = Color32::from_rgb(10, 10, 10);
                v.panel_fill = Color32::from_rgb(5, 5, 5);
                v.override_text_color = Some(Color32::from_rgb(0, 255, 0));
                v.selection = Selection {
                    bg_fill: Color32::from_rgb(0, 100, 0),
                    stroke: eframe::egui::Stroke::new(1.0, Color32::from_rgb(0, 255, 0)),
                };
                v
            },
            Theme::Cyanogen => {
                let mut v = Visuals::dark();
                v.window_fill = Color32::from_rgb(5, 10, 20);
                v.panel_fill = Color32::from_rgb(2, 5, 10);
                v.override_text_color = Some(Color32::from_rgb(200, 240, 255));
                v.selection = Selection {
                    bg_fill: Color32::from_rgb(0, 100, 150),
                    stroke: eframe::egui::Stroke::new(1.0, Color32::from_rgb(0, 255, 255)),
                };
                v
            },
            Theme::Cyberpunk => {
                let mut v = Visuals::dark();
                v.window_fill = Color32::from_rgb(20, 5, 20);
                v.panel_fill = Color32::from_rgb(15, 2, 15);
                v.override_text_color = Some(Color32::from_rgb(255, 200, 255));
                v.selection = Selection {
                    bg_fill: Color32::from_rgb(150, 0, 150),
                    stroke: eframe::egui::Stroke::new(1.0, Color32::from_rgb(255, 0, 255)),
                };
                v
            },
        };

        // Make widgets fit the theme
        match self {
            Theme::Cli => {
                v.widgets.noninteractive.bg_fill = Color32::BLACK;
                v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(0, 200, 0);
            }
            Theme::Cyanogen => {
                v.widgets.noninteractive.bg_fill = Color32::from_rgb(10, 20, 30);
                v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(0, 200, 255);
            }
            Theme::Cyberpunk => {
                v.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 10, 30);
                v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(255, 100, 255);
            }
            _ => {}
        }
        
        v
    }

    /// Determines if the markdown viewer should use light mode variants.
    pub fn is_light(&self) -> bool {
        matches!(self, Theme::Lite)
    }

    /// The primary accent color for this theme (used for active indicators, buttons, borders).
    pub fn accent_color(&self) -> Color32 {
        match self {
            Theme::Lite     => Color32::from_rgb(0, 100, 200),
            Theme::Dark     => Color32::from_rgb(0, 200, 255),
            Theme::Cli      => Color32::from_rgb(0, 255, 0),
            Theme::Cyanogen => Color32::from_rgb(0, 255, 255),
            Theme::Cyberpunk => Color32::from_rgb(255, 0, 255),
        }
    }

    /// A subtle separator / panel border color for this theme.
    pub fn separator_color(&self) -> Color32 {
        match self {
            Theme::Lite     => Color32::from_rgb(200, 200, 200),
            Theme::Dark     => Color32::from_rgb(50, 50, 60),
            Theme::Cli      => Color32::from_rgb(0, 80, 0),
            Theme::Cyanogen => Color32::from_rgb(0, 50, 80),
            Theme::Cyberpunk => Color32::from_rgb(80, 0, 80),
        }
    }

    /// Colors for the raw markdown syntax highlighter
    pub fn syntax_colors(&self) -> SyntaxColors {
        match self {
            Theme::Lite => SyntaxColors {
                text: Color32::BLACK,
                header: Color32::from_rgb(0, 50, 150),
                bold: Color32::from_rgb(150, 0, 0),
                italic: Color32::from_rgb(0, 100, 0),
                code: Color32::from_rgb(80, 80, 80),
                quote: Color32::from_rgb(100, 100, 100),
            },
            Theme::Dark => SyntaxColors {
                text: Color32::from_rgb(200, 200, 200),
                header: Color32::from_rgb(100, 200, 255),
                bold: Color32::from_rgb(255, 100, 100),
                italic: Color32::from_rgb(100, 255, 100),
                code: Color32::from_rgb(180, 180, 180),
                quote: Color32::from_rgb(150, 150, 150),
            },
            Theme::Cli => SyntaxColors {
                text: Color32::from_rgb(0, 255, 0),
                header: Color32::from_rgb(200, 255, 200),
                bold: Color32::WHITE,
                italic: Color32::from_rgb(0, 150, 0),
                code: Color32::from_rgb(0, 100, 0),
                quote: Color32::from_rgb(50, 150, 50),
            },
            Theme::Cyanogen => SyntaxColors {
                text: Color32::from_rgb(150, 220, 255),
                header: Color32::from_rgb(0, 255, 255),
                bold: Color32::from_rgb(255, 255, 255),
                italic: Color32::from_rgb(0, 150, 255),
                code: Color32::from_rgb(0, 100, 150),
                quote: Color32::from_rgb(100, 150, 200),
            },
            Theme::Cyberpunk => SyntaxColors {
                text: Color32::from_rgb(220, 180, 255),
                header: Color32::from_rgb(255, 0, 255),    // Neon pink
                bold: Color32::from_rgb(255, 255, 0),      // Acid yellow
                italic: Color32::from_rgb(0, 255, 255),    // Cyan
                code: Color32::from_rgb(100, 50, 100),     // Purple bg
                quote: Color32::from_rgb(150, 50, 200),    // Deep purple
            },
        }
    }
}

pub struct SyntaxColors {
    pub text: Color32,
    pub header: Color32,
    pub bold: Color32,
    pub italic: Color32,
    pub code: Color32,
    pub quote: Color32,
}
