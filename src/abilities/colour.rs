use crate::abilities::Ability;
use clap::{arg, ArgMatches, Command};
use css_color::Srgb;
use futures::FutureExt;
use image::{ImageFormat, Rgba, RgbaImage};
use matrix_sdk::{
    attachment::AttachmentConfig,
    room::reply::{EnforceThread, Reply},
    ruma::events::room::message::{OriginalSyncRoomMessageEvent, TextMessageEventContent},
    Room
};
use std::io::Cursor;

fn generate_image(width: u32, height: u32, rgb: Rgba<u8>) -> Vec<u8> {
    let mut png_buffer = Cursor::new(Vec::new());
    RgbaImage::from_pixel(width, height, rgb).write_to(&mut png_buffer, ImageFormat::Png).expect("failed to generate png");
    png_buffer.into_inner()
}

pub static COLOUR_ABILITY: Ability = Ability {
    name: "colour",
    aliases: &["color"],
    description: "Send an image of some colour.",
    command: || Some(Command::new("colour").arg(arg!(<colour> "The colour in css notation").num_args(1..).trailing_var_arg(true))),
    execute: |args: &ArgMatches, ev: &OriginalSyncRoomMessageEvent, room: &Room| {
        async move {
            let colour_arg = args.get_many::<String>("colour").unwrap().cloned().collect::<Vec<_>>().join(" ");
            let colour: Srgb = match colour_arg.parse() {
                Ok(colour) => colour,
                Err(_) => {
                    return Err("not a valid css colour".to_string());
                }
            };
            let rgba_value = Rgba([(colour.red * 255.0) as u8, (colour.green * 255.0) as u8, (colour.blue * 255.0) as u8, (colour.alpha * 255.0) as u8]);
            let png_data = generate_image(128, 128, rgba_value);
            let colour_name = match rgba_value[3] {
                255 => format!("#{:02X}{:02X}{:02X}", rgba_value[0], rgba_value[1], rgba_value[2]),
                _ => format!("#{:02X}{:02X}{:02X}{:02X}", rgba_value[0], rgba_value[1], rgba_value[2], rgba_value[3]),
            };
            room.send_attachment(
                format!("{}.png", colour_name),
                &mime::IMAGE_PNG,
                png_data,
                AttachmentConfig::new().caption(Some(TextMessageEventContent::plain(colour_name))).reply(Some(Reply{event_id: ev.event_id.clone(), enforce_thread: EnforceThread::MaybeThreaded}))
            ).await.expect("failed to send response");
            Ok(())
        }.boxed()
    }
};
