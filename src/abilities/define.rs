use crate::abilities::Ability;
use clap::{ArgMatches, Command, arg};
use futures::FutureExt;
use matrix_sdk::Room;
use matrix_sdk::ruma::events::room::message::{OriginalSyncRoomMessageEvent, RoomMessageEventContent};

pub static DEFINE_ABILITY: Ability = Ability {
    name: "define",
    aliases: &[],
    description: "Define a word.",
    command: || Some(Command::new("define").arg(arg!(--word <VALUE>))),
    execute: |args: &ArgMatches, ev: &OriginalSyncRoomMessageEvent, room: &Room| {
        async move {
            let message = RoomMessageEventContent::text_plain("definition...");
            room.send(message).await.expect("failed to send message");
            Ok(())
        }
        .boxed()
    },
};
