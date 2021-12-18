use teloxide::prelude::*;
use userdb::db::{RaffleDB, CodeUseCount};
use crate::commands::Context;
use crate::utils::*;

use super::dialogues::*;
pub async fn generate_code_cmd(
    usage_string: String,
    ctx: Context) -> TransitionOut<Dialogue> {
    let from_user = ctx.update.from();
    if from_user.is_none() {
        return exit();
    }
    let user_id = from_user.unwrap().id;
    if !is_admin(user_id) {
        // show admin keyboard
        ctx.answer("This command can only be used by an admin.")
        .await?;
        return next(Dialogue::Begin(NoData));
    }

    let usage = match usage_string.to_lowercase().as_str(){
        "illimited" => CodeUseCount::Illimited,
        "once" | "1" | "" => CodeUseCount::Once,
        num_string => {
            let n = num_string.parse::<i32>();
            if n.is_err() {
                ctx.answer("Sorry, but i couldn't parse the usage argument as a number.")
                .await?;
                return next(Dialogue::Begin(NoData)); 
            }
            let n = n.unwrap();
            CodeUseCount::Counted(n)
        }
    };
    let mut raffle_db = crate::db_instance.lock().await;
    match raffle_db.generate_raffle_code(usage) {
        Ok(code) => {
            ctx.answer(format!("Ok, i generated a code which can be used {} times.\nThe code is:", code.remaining_uses)).await?;
            ctx.answer(code.code).await?;
        },
        Err(e) => {
            ctx.answer(format!("Sorry, something failed while creating the code: {}", e.to_string())).await?;
            log::info!("While creating raffle code: {:?}", e);
        }
    }
    next(Dialogue::Begin(NoData))
}
pub async fn redeem_code_cmd(
    code_string: String,
    cx: Context) -> TransitionOut<Dialogue> {
    let from_user = cx.update.from();
    if from_user.is_none() {
        return exit();
    }
    let user_id = from_user.unwrap().id;
    if is_admin(user_id) {
        // show admin keyboard
        cx.answer("You can't redeem a code as an admin, silly..")
        .await?;
        return next(Dialogue::Begin(NoData));
    }
    let code = {
        let raffle_db = crate::db_instance.lock().await;
        raffle_db.get_raffle_code_by_name(code_string.as_str())
    };
    if code.is_err() {
        cx.answer("Sorry, a fatal error happened, please report this to my maintainer").await?;
        return next(Dialogue::Begin(NoData));
    }
    let code = code.unwrap();
    match code {
        Some(code_id) => {
            let result = {
                let mut raffle_db = crate::db_instance.lock().await;
                raffle_db.redeem_code(user_id, code_id.unique_id)
            };
            if result.is_err() {
                cx.answer("Sorry, a fatal error happened, please report this to my maintainer").await?;
                return next(Dialogue::Begin(NoData));
            }
            let result = result.unwrap();
            match result {
                userdb::db::CodeRedeemalResult::Redeemed => {
                    cx.answer("Success! You reedemed the code successfully, as a result you gained one more point!\n\nCheck your points with /points").await?;
                },
                userdb::db::CodeRedeemalResult::AlreadyRedeemed => {
                    cx.answer("Sorry, it looks like you already redeemed this code.").await?;
                },
                userdb::db::CodeRedeemalResult::NonExistingUser => {
                    cx.answer("Please join the raffle with /join or use a referral link before trying to redeem this code.").await?;
                },
                userdb::db::CodeRedeemalResult::NonExistingCode => {
                    cx.answer("Sorry, it looks like this code does not exist.").await?;
                },
            }
            cx.reply_to("Welcome to this raffle! You gained one point for joining, use /redeem to redeem additional codes and /points to see your points!").await?;
            cx.answer("PLACE REFERRAL LINK HERE").await?;
        },
        None => {
            cx.reply_to("Sorry, code not found. be sure to have written it correctly").await?;
        }
    }
    next(Dialogue::Begin(NoData))
}