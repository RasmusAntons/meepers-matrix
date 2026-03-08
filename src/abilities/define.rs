use futures::FutureExt;
use clap::{arg, ArgMatches, Command};
use matrix_sdk::Room;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use crate::abilities::Ability;

pub static DEFINE_ABILITY: Ability = Ability {
    name: "define",
    aliases: &[],
    description: "Define a word.",
    command: || Some(Command::new("define").arg(arg!(--word <VALUE>))),
    execute: |args: &ArgMatches, room: &Room| {
        async move {
            let message = RoomMessageEventContent::text_plain("definition...");
            room.send(message).await.expect("failed to send message");
            Ok(())
        }.boxed()
    }
};
