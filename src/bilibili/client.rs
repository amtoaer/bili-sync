use reqwest::Method;

pub struct Credential {
    sessdata: String,
    bili_jct: String,
    buvid3: String,
    dedeuserid: String,
    ac_time_value: String,
}

impl Credential {
    pub fn new(
        sessdata: String,
        bili_jct: String,
        buvid3: String,
        dedeuserid: String,
        ac_time_value: String,
    ) -> Self {
        Self {
            sessdata,
            bili_jct,
            buvid3,
            dedeuserid,
            ac_time_value,
        }
    }
}

pub struct BiliClient {
    credential: Option<Credential>,
    client: reqwest::Client,
}

impl BiliClient {
    pub fn anonymous() -> Self {
        let credential = None;
        let client = reqwest::Client::new();
        Self { credential, client }
    }

    pub fn authenticated(credential: Credential) -> Self {
        let credential = Some(credential);
        let client = reqwest::Client::new();
        Self { credential, client }
    }

    fn set_header(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let req =req.header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54")
        .header("Referer", "https://www.bilibili.com");
        if let Some(credential) = &self.credential {
            return req.header("cookie", format!("SESSDATA={}", credential.sessdata))
            .header("cookie", format!("bili_jct={}", credential.bili_jct))
            .header("cookie", format!("buvid3={}", credential.buvid3))
            .header(
                "cookie",
                format!("DedeUserID={}", credential.dedeuserid),
            )
            .header(
                "cookie",
                format!("ac_time_value={}", credential.ac_time_value),
            ).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36 Edg/116.0.1938.54")
            .header("Referer", "https://www.bilibili.com");
        }
        req
    }

    pub fn request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.set_header(self.client.request(method, url))
    }
}
