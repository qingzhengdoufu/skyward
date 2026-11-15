use super::*;

pub(super) fn message(l: Lang, error: decrypt::UnlockError) -> String {
    use skyward_crypto::credential::Rejection;
    match error.rejection {
        Some(Rejection::Incorrect) => l
            .pick(["密码错误，请重试", "密碼錯誤，請重試", "Incorrect password. Try again."])
            .to_string(),
        Some(Rejection::Retry {
            seconds: Some(seconds),
            incorrect: true,
        }) => skyward_ui::tr!(
            l,
            "密码错误，请等待 {seconds} 秒后重试",
            "密碼錯誤，請等待 {seconds} 秒後重試",
            "Incorrect password. Try again in {seconds} seconds."
        ),
        Some(Rejection::Retry {
            seconds: Some(seconds),
            incorrect: false,
        }) => skyward_ui::tr!(
            l,
            "请等待 {seconds} 秒后重试",
            "請等待 {seconds} 秒後重試",
            "Try again in {seconds} seconds."
        ),
        Some(Rejection::Retry {
            seconds: None,
            incorrect: true,
        }) => l
            .pick([
                "密码错误，请稍后重试",
                "密碼錯誤，請稍後重試",
                "Incorrect password. Try again later.",
            ])
            .to_string(),
        Some(Rejection::Retry {
            seconds: None,
            incorrect: false,
        }) => l
            .pick([
                "尝试过于频繁，请稍后重试",
                "嘗試過於頻繁，請稍後重試",
                "Too many attempts. Try again later.",
            ])
            .to_string(),
        None => error.details,
    }
}
