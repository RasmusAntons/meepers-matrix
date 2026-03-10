use anyhow::Result;
use clap::{ArgMatches, Command};
use futures::future::BoxFuture;
use lazy_static::lazy_static;
use matrix_sdk::{
    Room,
    ruma::events::room::message::{AddMentions, ForwardThread, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
};
use std::collections::HashMap;

pub mod colour;
pub mod config;
pub mod define;

pub struct Ability<'a> {
    name: &'a str,
    aliases: &'a [&'a str],
    description: &'a str,
    command: fn() -> Option<Command>,
    execute: for<'b> fn(&'b ArgMatches, &'b OriginalSyncRoomMessageEvent, &'b Room) -> BoxFuture<'b, Result<()>>,
}

impl Ability<'_> {
    fn parse_args(&self, args: Vec<String>) -> Result<ArgMatches, String> {
        match (self.command)() {
            None => Err("oof".to_string()),
            Some(command) => match command.try_get_matches_from(args) {
                Ok(matches) => Ok(matches),
                Err(err) => Err(err.to_string()),
            },
        }
    }
}

static ABILITIES: &[&Ability] = &[
    &colour::COLOUR_ABILITY,
    &config::CONFIG_ABILITY,
    &define::DEFINE_ABILITY,
];

lazy_static! {
    static ref ABILITY_MAP: HashMap<&'static str, &'static Ability<'static>> = {
        let mut m = HashMap::new();
        for &ability in ABILITIES {
            m.insert(ability.name, ability);
            for alias in ability.aliases.to_vec() {
                m.insert(alias, ability);
            }
        }
        m
    };
}

pub async fn on_message(ev: OriginalSyncRoomMessageEvent, room: Room) {
    if ev.content.body().starts_with("!") {
        let raw_command = ev.content.body().strip_prefix("!").unwrap();
        let unparsed_args: Vec<String> = raw_command.split_ascii_whitespace().map(String::from).collect();
        let ability = match ABILITY_MAP.get(unparsed_args[0].as_str()) {
            Some(ability) => *ability,
            None => {
                let message = RoomMessageEventContent::text_markdown("invalid command, use `!help` to list commands")
                    .make_reply_to(&ev, ForwardThread::Yes, AddMentions::Yes);
                room.send(message).await.unwrap();
                return;
            }
        };
        let unparsed_args: Vec<String> = ev.content.body().split_ascii_whitespace().map(String::from).collect();
        match ability.parse_args(unparsed_args) {
            Ok(args) => {
                if let Err(err) = (ability.execute)(&args, &ev, &room).await {
                    let message = RoomMessageEventContent::text_plain(err.to_string()).make_reply_to(
                        &ev,
                        ForwardThread::Yes,
                        AddMentions::Yes,
                    );
                    room.send(message).await.unwrap();
                }
            }
            Err(err) => {
                let message =
                    RoomMessageEventContent::text_plain(err).make_reply_to(&ev, ForwardThread::Yes, AddMentions::Yes);
                room.send(message).await.unwrap();
            }
        }
    }
}
