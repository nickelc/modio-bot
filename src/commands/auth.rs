use crate::bot::LoginUrl;
use crate::commands::prelude::*;
use crate::db::Users;

#[command]
#[description = "Authorize modbot to access your subscriptions."]
#[bucket = "simple"]
#[max_args(0)]
async fn login(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let users = data.get::<Users>().expect("get users failed");
    let login_url = data.get::<LoginUrl>().expect("get login url failed");
    match users.find_token(msg.author.id)? {
        None => {
            msg.channel_id
                .say(ctx, format!("Visit <{}> to authorize modbot.", login_url))
                .await?
        }
        Some(_) => msg.channel_id.say(ctx, "Already authorized.").await?,
    };
    Ok(())
}

#[command]
#[description = "De-authorize modbot to access your subscriptions."]
#[bucket = "simple"]
#[max_args(0)]
async fn logout(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let users = data.get::<Users>().expect("get users failed");
    users.delete(msg.author.id)?;
    Ok(())
}
