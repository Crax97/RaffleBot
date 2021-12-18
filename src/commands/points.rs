use teloxide::prelude::*;
use userdb::db::{RaffleDB, CodeUseCount};
use crate::commands::Context;
use crate::utils::*;

use super::dialogues::*;

pub async fn get_points_cdm(
    ctx: Context) -> TransitionOut<Dialogue> {
    let from_user = ctx.update.from();
    if from_user.is_none() {
        return exit();
    }
    let user_id = from_user.unwrap().id;
    if is_admin(user_id) {
        // show admin keyboard
        ctx.answer("You can't really have points as an admin...")
        .await?;
        return next(Dialogue::Begin(NoData));
    }
    let partecipant = {
        let raffle_db = crate::db_instance.lock().await;
        raffle_db.get_partecipant(user_id)
    };
    let partecipant = match partecipant {
        Ok(part_maybe) => match part_maybe {
            Some(part) => part,
            None => {
                ctx.answer("Sorry, you must be a member of the raffle in order to get points.")
                .await?;
                return next(Dialogue::Begin(NoData));
            }
        },
        Err(e) => {
            ctx.answer("Sorry, please tell my admin a fatal error happened")
            .await?;
            log::error!("While getting user points: {}", e.to_string());
            return next(Dialogue::Begin(NoData));
        }
    };

    ctx.answer(format!("Sure! You have {} points.", partecipant.priority))
    .await?;
    next(Dialogue::Begin(NoData))
}