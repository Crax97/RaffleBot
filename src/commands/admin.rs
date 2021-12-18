use std::ops::Add;

use serde::{Deserialize, Serialize};
use teloxide::{prelude::*, RequestError};
use userdb::db::{RaffleDB, CodeUseCount, UserID, Partecipant};
use super::dialogues::*;
use crate::commands::Context;
use crate::utils::*;

#[derive(Serialize, Deserialize, Clone)]
pub enum RaffleDescription {
    Text(String),
    Photo {
        caption: Option<String>,
        file_id: String
    }
}

impl RaffleDescription {
    fn from_message(msg: &Message) -> Option<RaffleDescription> {
        if msg.text().is_some() {
            Some(RaffleDescription::Text(msg.text().unwrap().to_owned()))
        } else {
            if msg.photo().is_none() {
                return None;
            }
            let photo = msg.photo().unwrap().get(0).unwrap();
            Some(RaffleDescription::Photo {
                file_id: photo.file_id.clone(),
                caption: match msg.caption() {
                    Some(str) => Some(str.to_owned()),
                    None => None
                }
            })
        }
    }
}

pub async fn create_raffle(ctx: Context)
    -> TransitionOut<Dialogue> {
    let user = ctx.update.from();
    if user.is_none() {
        return next(Dialogue::Begin(NoData));
    }
    let user = user.unwrap().id;
    if !is_admin(user) {
        ctx.answer("You must be an admin to run this command.").await?;
        return next(Dialogue::Begin(NoData));
    }
    
    let creation_status = {
        let raffle_db = crate::db_instance.lock().await;
        raffle_db.get_ongoing_raffle()
    };
    match creation_status {
        Err(e) => {
            ctx.answer(format!("A fatal error occurred while creating the raffle:
{}

Please send this error to my manager.", e.to_string())).await?;
            return next(Dialogue::Begin(NoData));
        },
        _ => {}
    }
    let creation_status = creation_status.unwrap();
    if creation_status.is_some() {
        ctx.answer("There is a raffle ongoing already, please end it before starting a new one.").await?;
        return next(Dialogue::Begin(NoData));
    }
    ctx.answer("Sure! Send me the title of the raffle.").await?;
    next(Dialogue::AwaitRaffleTitle(AwaitingRaffleTitleState))
}


#[teloxide(subtransition)]
async fn raffle_get_title(
    state: AwaitingRaffleTitleState,
    cx: TransitionIn<AutoSend<Bot>>,
    _ans: String
) -> TransitionOut<Dialogue> {
    
    let message_content = cx.update.text();
    if message_content.is_none() {
        cx.answer("Please provide a text message with the title of the raffle").await?;
        return next(state);
    }
    cx.answer("Good! Now send me a message with the raffle's description.
This message must contain at least one photo or some text: those will be copied and sent to each user each time they want to join a raffle").await?;
    next(Dialogue::AwaitingRaffleMessage( AwaitingRaffleMessageState{
        title: message_content.unwrap().to_owned()
    }))
}

#[teloxide(subtransition)]
async fn raffle_get_desc_message(
    state: AwaitingRaffleMessageState,
    cx: TransitionIn<AutoSend<Bot>>,
    _ans: String
) -> TransitionOut<Dialogue> {
    let message_serialized = serde_json::to_string(&RaffleDescription::from_message(&cx.update))
        .expect("Failure in serializing the message from the user");
    let creation_status = {
        let mut raffle_db = crate::db_instance.lock().await;
        match raffle_db.create_raffle(state.title.as_str(), message_serialized.as_str()) {
            Ok(status) => status,
            Err(e) => {
                cx.answer(format!("A fatal error occurred while creating the raffle:
{}

Please send this error to my manager.", e.to_string())).await?;
                return next(Dialogue::Begin(NoData));
            }
        }
    };

    match creation_status {
        userdb::db::RaffleCreationResult::Success(_) => {
            cx.answer("Success! A new raffle was started!").await?;
        },
        userdb::db::RaffleCreationResult::OngoingRaffleExists(_) => {
            cx.answer("There is a new raffle already, maybe someone else created it before you?").await?;
        }
    };
    exit()
}

async fn send_winner_notification(place: usize, winner: &Partecipant, bot: &AutoSend<Bot>) -> Result<(), RequestError> {
    let msg = format!("Congraulations! You placed {} in the current raffle, with a toal of {} points, contact the raffle manager for your prize.",
    place,
    winner.priority);
    bot.send_message(winner.user_id, msg).await?;
    Ok(())
}

pub async fn end_raffle(ctx: Context)
    -> TransitionOut<Dialogue> {
        let user = ctx.update.from();
        if user.is_none() {
            return next(Dialogue::Begin(NoData));
        }
        let user = user.unwrap().id;
        if !is_admin(user) {
            ctx.answer("You must be an admin to run this command.").await?;
            return next(Dialogue::Begin(NoData));
        }
        
        const WINNER_COUNT : usize = 1;
        let winners = {
            let mut raffle_db = crate::db_instance.lock().await;
            raffle_db.stop_raffle(WINNER_COUNT)
        };
        match winners {
            Err(e) => {
                ctx.answer(format!("A fatal error occurred while stopping the raffle:
{}

Please send this error to my manager.", e.to_string())).await?;
                return next(Dialogue::Begin(NoData));
            },
            _ => {}
        };
        let winners = winners.unwrap();
        let mut winner_str = String::new();
        for (i, winner) in winners.iter().enumerate() {
            let tag = match get_user_tag(winner.user_id, target_chat(), &ctx.requester).await {
                Ok(n) => n,
                Err(_) => format!("user id {}, ask crax", winner.user_id)
            };
            let place =  i + 1;
            winner_str = winner_str.add(format!("{}. {}", place, tag).add("\n").as_str());
            let _ = send_winner_notification(place, &winner, &ctx.requester).await; // Best to ignore the error
        }
        ctx.answer(format!("TODO WINNERS:\n{}", winner_str)).await?;
        next(Dialogue::AwaitRaffleTitle(AwaitingRaffleTitleState))

}