use crate::abilities::Ability;
use crate::config;
use anyhow::Error;
use clap::{ArgMatches, Command, arg};
use futures::FutureExt;
use matrix_sdk::Room;
use matrix_sdk::ruma::events::room::message::{
    AddMentions, ForwardThread, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};

pub static CONFIG_ABILITY: Ability = Ability {
    name: "config",
    aliases: &[],
    description: "Manage bot configuration.",
    command: || {
        Some(
            Command::new("config")
                .subcommand_required(true)
                .subcommand(
                    Command::new("get")
                        .about("Query a configuration value")
                        .arg(arg!(<key> "The config key")),
                )
                .subcommand(
                    Command::new("set")
                        .about("Set a configuration value")
                        .arg(arg!(<key> "The config key"))
                        .arg(arg!(<value> "The value").num_args(1..).trailing_var_arg(true)),
                ),
        )
    },
    execute: |args: &ArgMatches, ev: &OriginalSyncRoomMessageEvent, room: &Room| {
        async move {
            match args.subcommand() {
                Some(("get", sub_args)) => {
                    let key_name = sub_args
                        .get_one::<String>("key")
                        .ok_or(Error::msg("failed to get requested key from args"))?;
                    let value = config::get_json_by_name(key_name.as_str())?;
                    let message = RoomMessageEventContent::text_plain(format!("{} = {}", key_name, value))
                        .make_reply_to(ev, ForwardThread::Yes, AddMentions::Yes);
                    room.send(message).await.expect("failed to send message");
                }
                Some(("set", sub_args)) => {
                    let key_name = sub_args
                        .get_one::<String>("key")
                        .ok_or(Error::msg("failed to get requested key from args"))?;
                    let value = sub_args
                        .get_many::<String>("value")
                        .ok_or(Error::msg("failed to get requested value from args"))?
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(" ");
                    let json_value: serde_json::Value = serde_json::from_str(value.as_str())?;
                    config::set_json_by_name(key_name, &json_value)?;
                    let message =
                        RoomMessageEventContent::text_plain(format!("{} = {}", key_name, json_value.to_string()))
                            .make_reply_to(ev, ForwardThread::Yes, AddMentions::Yes);
                    room.send(message).await.expect("failed to send message");
                }
                _ => {}
            }
            Ok(())
        }
        .boxed()
    },
};
