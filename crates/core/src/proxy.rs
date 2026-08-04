use std::borrow::Cow;

pub fn with_remote_dns(proxy_url: &str) -> Cow<'_, str> {
    match proxy_url.strip_prefix("socks5://") {
        Some(authority) => Cow::Owned(format!("socks5h://{authority}")),
        None => Cow::Borrowed(proxy_url),
    }
}

#[cfg(test)]
mod tests {
    use super::with_remote_dns;

    #[test]
    fn socks5_uses_proxy_dns_without_changing_other_proxy_schemes() {
        assert_eq!(
            with_remote_dns("socks5://127.0.0.1:1080"),
            "socks5h://127.0.0.1:1080"
        );
        assert_eq!(
            with_remote_dns("http://127.0.0.1:7890"),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            with_remote_dns("socks5h://127.0.0.1:1080"),
            "socks5h://127.0.0.1:1080"
        );
    }
}
