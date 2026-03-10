use crate::abilities::Ability;
use anyhow::Error;
use clap::{ArgMatches, Command, arg};
use css_color::Srgb;
use futures::FutureExt;
use image::{ImageFormat, Rgba, RgbaImage};
use matrix_sdk::attachment::{AttachmentInfo, BaseImageInfo};
use matrix_sdk::ruma::UInt;
use matrix_sdk::{
    Room,
    attachment::AttachmentConfig,
    room::reply::{EnforceThread, Reply},
    ruma::events::room::message::{OriginalSyncRoomMessageEvent, TextMessageEventContent},
};
use std::io::Cursor;

fn generate_image(width: u32, height: u32, rgb: Rgba<u8>) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    RgbaImage::from_pixel(width, height, rgb)
        .write_to(&mut buf, ImageFormat::Png)
        .expect("failed to generate png");
    buf.into_inner()
}

pub static COLOUR_ABILITY: Ability = Ability {
    name: "colour",
    aliases: &["color"],
    description: "Send an image of some colour.",
    command: || {
        Some(
            Command::new("colour").arg(
                arg!(<colour> "The colour in css notation")
                    .num_args(1..)
                    .trailing_var_arg(true),
            ),
        )
    },
    execute: |args: &ArgMatches, ev: &OriginalSyncRoomMessageEvent, room: &Room| {
        async move {
            let colour_arg = args
                .get_many::<String>("colour")
                .unwrap()
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            let colour: Srgb = match colour_arg.parse() {
                Ok(colour) => colour,
                Err(_) => {
                    return Err(Error::msg("not a valid css colour"));
                }
            };
            let rgba = Rgba([
                (colour.red * 255.0) as u8,
                (colour.green * 255.0) as u8,
                (colour.blue * 255.0) as u8,
                (colour.alpha * 255.0) as u8,
            ]);
            let png = generate_image(128, 128, rgba);
            let colour_hex = match rgba[3] {
                255 => format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]),
                _ => format!("#{:02X}{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2], rgba[3]),
            };
            room.send_attachment(
                format!("{}.png", colour_hex),
                &mime::IMAGE_PNG,
                png,
                AttachmentConfig::new()
                    .caption(Some(TextMessageEventContent::plain(colour_hex)))
                    .reply(Some(Reply {
                        event_id: ev.event_id.clone(),
                        enforce_thread: EnforceThread::MaybeThreaded,
                    }))
                    .info(AttachmentInfo::Image(BaseImageInfo {
                        height: Some(UInt::from(128u32)),
                        width: Some(UInt::from(128u32)),
                        size: None,
                        blurhash: None,
                        is_animated: None,
                    })),
            )
            .await
            .expect("failed to send response");
            Ok(())
        }
        .boxed()
    },
};
