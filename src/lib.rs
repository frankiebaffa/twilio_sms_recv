use {
    actix_web::{
        HttpResponse,
        Responder,
    },
    serde::Deserialize,
    slog::LogContext,
    std::{
        env::var as env_var,
        future::Future,
    },
};
const LOG_DIR: &'static str = "TWILIO_RECV_LOG_DIR";
const LOG_BASE_NAME: &'static str = "TWILIO_RECV_LOG_NAME";
#[derive(Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "MessageSid")]
    message_sid: String,
    #[serde(rename = "AccountSid")]
    account_sid: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Body")]
    body: String,
}
impl InboundMessage {
    #[cfg(test)]
    pub fn new(
        message_sid: String, account_sid: String, from: String,
        to: String, body: String
    ) -> Self {
        Self { message_sid, account_sid, from, to, body, }
    }
    pub fn get_message_sid(&self) -> String {
        self.message_sid.clone()
    }
    pub fn get_account_sid(&self) -> String {
        self.account_sid.clone()
    }
    pub fn get_from(&self) -> String {
        self.from.clone()
    }
    pub fn get_to(&self) -> String {
        self.to.clone()
    }
    pub fn get_body(&self) -> String {
        self.body.clone()
    }
}
enum NumberAcceptance<'a> {
    All,
    Specific(Vec<&'a str>),
    Single(&'a str),
}
fn check_num(acc: NumberAcceptance, msg: &InboundMessage) -> bool {
    match acc {
        NumberAcceptance::All => true,
        NumberAcceptance::Single(n) => {
            if msg.from.eq(n) {
                true
            } else {
                false
            }
        },
        NumberAcceptance::Specific(ns) => {
            if ns.contains(&msg.from.as_str()) {
                true
            } else {
                false
            }
        },
    }
}
pub fn recv_callback_log(ctx: &LogContext, msg: impl AsRef<str>) {
    let message = msg.as_ref();
    ctx.log(format!("Callback - {}", message));
}
pub fn recv_callback_error(ctx: &LogContext, msg: impl AsRef<str>) {
    let message = msg.as_ref();
    ctx.error(format!("Callback - {}", message));
}
pub async fn recv<'a, F>(
    msg: InboundMessage, func: &'static dyn Fn(LogContext, InboundMessage) -> F
) -> impl Responder
where
    F: Future<Output = bool> + 'static
{
    let ctx = LogContext::from_env(LOG_DIR, LOG_BASE_NAME);
    ctx.log("Received request");
    const ACCEPTED_KEY: &'static str = "TWILIO_RECV_ACCEPTED_NUMS";
    let accepted_nums = match env_var(ACCEPTED_KEY) {
        Ok(an) => an,
        Err(e) => {
            ctx.error(format!(
                "Failed to retrieve variable from environment {}: {}",
                ACCEPTED_KEY, e
            ));
            return HttpResponse::InternalServerError().body(
                "Failed to retrieve accepted numbers from environment"
            );
        },
    };
    let nums;
    if accepted_nums.eq(&"*") {
        ctx.log("Accepting all numbers");
        nums = NumberAcceptance::All;
    } else if accepted_nums.contains(',') {
        ctx.log("Accepting specific numbers");
        nums = NumberAcceptance::Specific(
            accepted_nums.split(',').collect::<Vec<&str>>()
        );
    } else {
        ctx.log("Accepting single number");
        nums = NumberAcceptance::Single(
            accepted_nums.as_str()
        );
    }
    if !check_num(nums, &msg) {
        ctx.error("From number failed check against accepted numbers");
        HttpResponse::InternalServerError().body(
            "Invalid \"from\" number"
        )
    } else if func(ctx.clone(), msg).await {
        ctx.log("Handler succeeded");
        HttpResponse::Ok().body(
            "Message handler succeeded"
        )
    } else {
        ctx.error("Handler failed");
        HttpResponse::InternalServerError().body(
            "Message handler failed"
        )
    }
}
#[cfg(test)]
mod test_function {
    use {
        crate::InboundMessage,
        slog::LogContext,
    };
    pub async fn test_receive(_: LogContext, _: InboundMessage) -> bool {
        true
    }
}
#[cfg(test)]
mod tests {
    use crate::{ InboundMessage, recv, test_function::test_receive, };
    #[test]
    fn test() {
        let inbound = InboundMessage::new(
            "TEST".to_string(),
            "TEST".to_string(),
            "+15555555555".to_string(),
            "+15554443333".to_string(),
            "Hello, World!".to_string()
        );
        recv(inbound, &test_receive);
    }
}
