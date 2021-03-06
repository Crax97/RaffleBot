use std::ops::Add;

use serde::{Deserialize, Serialize};
use teloxide::types::ParseMode;
use teloxide::{prelude::*, RequestError};
use userdb::db::{RaffleDB, Partecipant};
use super::{dialogues::*, RaffleBot};
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
        match (msg.text(), msg.photo()) {
            (Some(text), _) => Some(RaffleDescription::Text(text.to_owned())),
            (_, Some(photos)) => {
                let photo = photos.get(0).expect("Got into Some(photos) but photos is empty");
                Some(RaffleDescription::Photo {
                    file_id: photo.file_id.clone(),
                    caption: match msg.caption() {
                        Some(str) => Some(str.to_owned()),
                        None => None
                    }
                })
            },
            _ => unreachable!()
        }
    }
}



pub async fn stats(ctx: Context)
    -> TransitionOut<Dialogue> {
        let user = match ctx.update.from() {
            Some(u) => u.id,
            None => { 
                return next(Dialogue::Begin(NoData));
            }
        };
        if !is_admin(user) {
            ctx.answer("You must be an admin to run this command.").await?;
            return next(Dialogue::Begin(NoData));
        }

        let partecipants = {
            let raffle_db = crate::DB_INSTANCE.lock().await;
            raffle_db.get_partecipants()
        };
        let mut partecipants = match partecipants {
            Ok(partecipants) => Vec::from_iter(partecipants.iter().map(|p| p.clone())),
            Err(e) => {
                on_error(e, &ctx.update, &ctx.requester, "stats: fetch partecipants").await;
                return next(Dialogue::Begin(NoData));
            }
        };
        partecipants.sort_by(|a, b|  b.priority.cmp(&a.priority));

        let count_partecipants = partecipants.len();
        let top_ten= partecipants.iter().take(10);

        let mut msg = String::default();
        for (i, part) in top_ten.enumerate() {
            let tag = match get_user_tag(part.user_id, target_chat(), &ctx.requester).await {
                Ok(n) => n,
                Err(_) => format!("user id {}, ask crax", part.user_id)
            };
            let place =  i + 1;
            msg = msg.add(format!("{}. {} - {} point(s)", place, tag, part.priority).add("\n").as_str());
        }

        let msg = format!("<b>Top ten:</b>\n{}\n\n<b>Raffle stats:</b>\nNumber of partecipants: {}", msg, count_partecipants);
        ctx.answer(msg)
            .parse_mode(ParseMode::Html)
            .await?;

        next(Dialogue::Begin(NoData))
}

pub async fn create_raffle(ctx: Context)
    -> TransitionOut<Dialogue> {
    let user = match ctx.update.from() {
        Some(u) => u.id,
        None => { 
            return next(Dialogue::Begin(NoData));
        }
    };
    if !is_admin(user) {
        ctx.answer("You must be an admin to run this command.").await?;
        return next(Dialogue::Begin(NoData));
    }
    
    let creation_status = {
        let raffle_db = crate::DB_INSTANCE.lock().await;
        raffle_db.get_ongoing_raffle()
    };
    let creation_status = match creation_status {
        Err(e) => {
            on_error(e, &ctx.update, &ctx.requester, "on raffle creation: begin").await;
            return next(Dialogue::Begin(NoData));
        },
        Ok(status) => status
    };
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
    cx: TransitionIn<RaffleBot>,
    _ans: String
) -> TransitionOut<Dialogue> {
    match cx.update.text() {
        Some(text) => {
            cx.answer("Good! Now send me a message with the raffle's description.
            This message must contain at least one photo or some text: those will be copied and sent to each user each time they want to join a raffle").await?;
                next(Dialogue::AwaitingRaffleMessage( AwaitingRaffleMessageState{
                    title: text.to_owned()
                }))
        }
        None => {
            cx.answer("Please provide a text message with the title of the raffle").await?;
            return next(state);
        }
    }
}

#[teloxide(subtransition)]
async fn raffle_get_desc_message(
    state: AwaitingRaffleMessageState,
    cx: TransitionIn<RaffleBot>,
    _ans: String
) -> TransitionOut<Dialogue> {
    let message_serialized = serde_json::to_string(&RaffleDescription::from_message(&cx.update))
        .expect("Failure in serializing the message from the user");
    let creation_status = {
        let mut raffle_db = crate::DB_INSTANCE.lock().await;
        match raffle_db.create_raffle(state.title.as_str(), message_serialized.as_str()) {
            Ok(status) => status,
            Err(e) => {
                on_error(e, &cx.update, &cx.requester, "on raffle: await raffle desc").await;
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

async fn send_winner_notification(place: usize, winner: &Partecipant, bot: &RaffleBot) -> Result<(), RequestError> {
    let msg = format!("Congraulations! You placed {} in the current raffle, with a toal of {} points, contact the raffle manager for your prize.",
    place,
    winner.priority);
    bot.send_message(winner.user_id, msg).await?;
    Ok(())
}

pub async fn end_raffle(ctx: Context)
    -> TransitionOut<Dialogue> {
        let user = match ctx.update.from() {
            Some(u) => u.id,
            None => { 
                return next(Dialogue::Begin(NoData));
            }
        };
        if !is_admin(user) {
            ctx.answer("You must be an admin to run this command.").await?;
            return next(Dialogue::Begin(NoData));
        }
        
        const WINNER_COUNT : usize = 1;
        let winners = {
            let mut raffle_db = crate::DB_INSTANCE.lock().await;
            raffle_db.stop_raffle(WINNER_COUNT)
        };
        let winners = match winners {
            Err(e) => {
                on_error(e, &ctx.update, &ctx.requester, "on raffle end").await;
                return next(Dialogue::Begin(NoData));
            },
            Ok(vector) => vector
        };
        let mut winner_str = String::new();
        for (i, winner) in winners.iter().enumerate() {
            let tag = match get_user_tag(winner.user_id, target_chat(), &ctx.requester).await {
                Ok(n) => n,
                Err(_) => format!("user id {}, ask crax", winner.user_id)
            };
            let place =  i + 1;
            winner_str = winner_str.add(format!("{}. {} - {} point(s)", place, tag, winner.priority).add("\n").as_str());
            let _ = send_winner_notification(place, &winner, &ctx.requester).await; // Best to ignore the error
        }
        ctx.answer(format!("Okay! Here are the winners i picked for this raffle:\n{}", winner_str)).await?;
        next(Dialogue::AwaitRaffleTitle(AwaitingRaffleTitleState))

}