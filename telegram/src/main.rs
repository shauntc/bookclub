use std::collections::HashSet;

use anyhow::Result;
use chrono::{offset::Local, DateTime};
use chrono_english::{parse_date_string, Dialect};

use teloxide::{
    dispatching::dialogue::{serializer::Json, Dialogue, SqliteStorage},
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Me},
    utils::command::{BotCommands, ParseError},
};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize, Debug)]
enum State {
    #[default]
    Start,
    Polling {
        start: DateTime<Local>,
        end: DateTime<Local>,
        selected: HashSet<String>,
    },
}

type DialogState = Dialogue<State, SqliteStorage<Json>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv::dotenv().ok();

    let storage = SqliteStorage::open("telegram.sqlite", Json).await.unwrap();

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .enter_dialogue::<Update, SqliteStorage<Json>, State>()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "echo the text")]
    Echo(String),
    #[command(
        description = "create a poll for dates between start and end",
        parse_with = split_dates
    )]
    PollDate {
        start: DateTime<Local>,
        end: DateTime<Local>,
    },
    #[command(description = "display this text")]
    Help,
}

async fn callback_handler(bot: Bot, dialog: DialogState, q: CallbackQuery) -> Result<()> {
    match dialog.get_or_default().await? {
        State::Polling {
            start,
            end,
            selected,
        } => {
            if let Some(data) = q.data {
                println!("callback_handler: {data} {start} {end}");
            }
            let message_id = if let Some(id) = q.inline_message_id {
                id
            } else if let Some(msg) = q.message {
                msg.id.to_string()
            } else {
                println!("No message id");
                return Ok(());
            };

            println!("callback_handler: {message_id}");

            let markup = make_keyboard(start, end, &selected);
            bot.edit_message_reply_markup_inline(message_id)
                .reply_markup(markup)
                .await?;
        }
        State::Start => {
            println!("callback_handler: start")
        }
    }
    Ok(())
}

async fn message_handler(bot: Bot, dialog: DialogState, msg: Message, me: Me) -> Result<()> {
    println!("message_handler");

    let text = msg.text().ok_or(anyhow::anyhow!("No text in message"))?;
    match BotCommands::parse(text, me.username()) {
        Ok(Command::Echo(text)) => {
            bot.send_message(msg.chat.id, format!("you said '{text}'"))
                .await?;
        }
        Ok(Command::PollDate { start, end }) => {
            bot.send_message(msg.chat.id, format!("start: {start}, end: {end}"))
                .await?;
            let selected = HashSet::new();
            let buttons = make_keyboard(start, end, &selected);
            dialog
                .update(State::Polling {
                    start,
                    end,
                    selected,
                })
                .await?;
            bot.send_message(msg.chat.id, "Select Days")
                .reply_markup(buttons)
                .await?;
        }
        Ok(Command::Help) => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Err(err) => {
            bot.send_message(msg.chat.id, err.to_string()).await?;
        }
    };

    Ok(())
}

fn make_keyboard(
    start: DateTime<Local>,
    end: DateTime<Local>,
    selected: &HashSet<String>,
) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let dur = end - start;
    let days = dur.num_days();

    for d in 0..days {
        let date = start + chrono::Duration::days(d);

        let date_id = date.format("%Y-%m-%d").to_string();
        let display_date = if selected.contains(&date_id) {
            format!("{} ✅", date.format("%a %d %b"))
        } else {
            date.format("%a %d %b").to_string()
        };

        keyboard.push(vec![InlineKeyboardButton::callback(display_date, date_id)]);
    }

    InlineKeyboardMarkup::new(keyboard)
}

fn split_dates(s: String) -> Result<(DateTime<Local>, DateTime<Local>), ParseError> {
    let mut iter = s.split(" to ");
    let start = iter
        .next()
        .ok_or(ParseError::Custom("dates must be separated by 'to'".into()))?;
    let start = parse_date_string(start, Local::now(), Dialect::Uk).map_err(|e| {
        ParseError::Custom(format!("unable to parse start date: {e:?} '{start}'").into())
    })?;

    let end = iter
        .next()
        .ok_or(ParseError::Custom("dates must be separated by 'to'".into()))?;
    let end = parse_date_string(end, Local::now(), Dialect::Uk).map_err(|e| {
        ParseError::Custom(format!("unable to parse end date: {e:?} '{end}'").into())
    })?;

    Ok((start, end))
}
