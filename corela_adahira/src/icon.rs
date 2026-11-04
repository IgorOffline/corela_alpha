use std::sync::atomic::{AtomicU32, Ordering};

use macroquad::prelude::ImageFormat;
use macroquad::texture::Image;

const BYTES_PER_PIXEL: usize = 4;

static NEXT_ICON_ID: AtomicU32 = AtomicU32::new(0);

pub struct Icon {
    id: u32,
    image: Image,
}

impl Icon {
    pub fn load_png(bytes: &[u8]) -> Result<Self, String> {
        let mut image = match Image::from_file_with_format(bytes, Some(ImageFormat::Png)) {
            Ok(image) => image,
            Err(error) => return Err(error.to_string()),
        };
        if image.width == 0 || image.height == 0 {
            return Err(String::from("the_image_is_empty"));
        }
        make_white(&mut image);
        Ok(Self {
            id: NEXT_ICON_ID.fetch_add(1, Ordering::Relaxed),
            image,
        })
    }

    pub(crate) const fn id(&self) -> u32 {
        self.id
    }

    pub(crate) const fn image(&self) -> &Image {
        &self.image
    }
}

fn make_white(image: &mut Image) {
    let (pixels, _leftover) = image.bytes.as_chunks_mut::<BYTES_PER_PIXEL>();
    for pixel in pixels {
        let [red, green, blue, _alpha] = pixel;
        *red = u8::MAX;
        *green = u8::MAX;
        *blue = u8::MAX;
    }
}
