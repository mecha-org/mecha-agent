use gtk::{gdk, gio};
use relm4::gtk::{self, glib::Bytes, prelude::FileExt};
use widgets::gif_paintable::GifPaintable;

use crate::widgets;

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

// pub async fn get_img_from_url(path: Option<String>, css_classes: &[&str]) -> gtk::Image {
//     let image = gtk::Image::builder().css_classes(css_classes).build();

//     let response = reqwest::get(path.unwrap()).await;
//     let image_bytes = response.unwrap().bytes().await.expect("Failed to get image bytes");
//     let bytes = Bytes::from(&image_bytes);

//     match gdk::Texture::from_bytes(&bytes) {
//         Ok(image_asset_paintable) => {
//             image.set_paintable(Option::from(&image_asset_paintable));
//         },
//         Err(_) => (),
//     }
//     image
// }

pub async fn get_image_bytes(path: Option<String>) -> Option<relm4::gtk::glib::Bytes> {
    let response = reqwest::get(path.unwrap()).await;
    let image_bytes = response
        .unwrap()
        .bytes()
        .await
        .expect("Failed to get image bytes");
    let bytes = Bytes::from(&image_bytes);
    Some(bytes)
}

pub fn get_image_from_url(bytes: Option<Bytes>, css_classes: &[&str]) -> gdk::Texture {
    match gdk::Texture::from_bytes(&bytes.unwrap()) {
        Ok(image_asset_paintable) => image_asset_paintable,
        Err(_) => {
            todo!()
        }
    }
}
