use gtk::{gdk, gio};
use relm4::gtk::{self, gdk::Texture, glib::Bytes, prelude::FileExt};
use widgets::gif_paintable::GifPaintable;

use crate::widgets;
use simple_base64::decode;

pub fn get_image_from_path(path: Option<String>, css_classes: &[&str]) -> gtk::Image {
    let image = gtk::Image::builder().css_classes(css_classes).build();

    match path {
        Some(p) => {
            let image_file = gio::File::for_path(p);
            match gdk::Texture::from_file(&image_file) {
                Ok(image_asset_paintable) => {
                    image.set_paintable(Option::from(&image_asset_paintable));
                }
                Err(_) => (),
            }
        }
        None => (),
    }
    image
}

pub fn get_gif_from_path(gif_path: Option<String>) -> GifPaintable {
    let paintable = GifPaintable::new();

    match gif_path {
        Some(path) => {
            let image_file = gio::File::for_path(path);
            match image_file.load_contents(gio::Cancellable::NONE) {
                Ok((bytes, _)) => {
                    let _ = paintable.load_from_bytes(&bytes);
                }
                Err(_) => (),
            };
        }
        None => (),
    }
    paintable
}

pub fn get_texture_from_base64(
    base64_string: String,
) -> Result<Texture, Box<dyn std::error::Error>> {
    let texture_value = match decode(base64_string) {
        Ok(response) => {
            let bytes = Bytes::from(&response);
            let texture = match gdk::Texture::from_bytes(&bytes) {
                Ok(response) => response,
                Err(texture_error) => {
                    eprintln!("ERROR::texture_error {:?} ", texture_error);
                    return Err(Box::new(texture_error));
                }
            };
            texture
        }
        Err(decode_error) => {
            eprintln!("ERROR::DecodeError {:?} ", decode_error);
            return Err(Box::new(decode_error));
        }
    };
    Ok(texture_value)
}
