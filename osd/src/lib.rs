
use gtk4::{Application, ApplicationWindow, CssProvider, Label, Orientation::Vertical, gio::prelude::{ApplicationExt, ApplicationExtManual}, prelude::{BoxExt, GtkWindowExt, WidgetExt}, subclass::widget::WidgetAction};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub struct OSD {
    
}

impl OSD {
    pub fn new() -> OSD {
        // tokio::task::spawn_blocking(|| {
            let app = Application::builder().application_id("io.github.feois.fmp.osd").build();
            
            app.connect_activate(Self::build);
            
            app.run();
        // });
        
        OSD {
            
        }
    }
    
    fn build(app: &Application) {
        let window = ApplicationWindow::builder().application(app).build();
        
        window.init_layer_shell();
        
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_margin(Edge::Top, 32);
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(KeyboardMode::None);
        
        let cont = gtk4::Box::new(Vertical, 0);
        
        cont.append(&Label::new(Some("test")));
        cont.append(&Label::new(Some("foo")));
        cont.set_margin_top(16);
        cont.set_margin_bottom(16);
        cont.set_margin_start(8);
        cont.set_margin_end(24);
        
        window.set_child(Some(&cont));
        
        let provider = CssProvider::new();
        
        provider.load_from_data("
            window {
                background-color: rgba(127, 127, 127, 0.5);
            }
            
            box {
                background-color: rgba(191, 191, 191, 0.5);
            }
            
            label {
                color: rgba(0, 0, 0, 1);
                font-size: 14px;
            }
        ");
        
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().unwrap(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        
        window.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test() {
        let osd = OSD::new();
    }
}
