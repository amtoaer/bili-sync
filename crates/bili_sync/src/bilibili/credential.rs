use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use cookie::Cookie;
use regex::Regex;
use reqwest::{header, Method};
use rsa::pkcs8::DecodePublicKey;
use rsa::sha2::Sha256;
use rsa::{Oaep, RsaPublicKey};
use serde::{Deserialize, Serialize};

use crate::bilibili::{Client, Validate};

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38,
    41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36,
    20, 34, 44, 52,
];

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub sessdata: String,
    pub bili_jct: String,
    pub buvid3: String,
    pub dedeuserid: String,
    pub ac_time_value: String,
}

#[derive(Debug, Deserialize)]
pub struct WbiImg {
    img_url: String,
    sub_url: String,
}

impl WbiImg {
    pub fn into_mixin_key(self) -> Option<String> {
        get_mixin_key(self)
    }
}

impl Credential {
    pub async fn wbi_img(&self, client: &Client) -> Result<WbiImg> {
        let mut res = client
            .request(Method::GET, "https://api.bilibili.com/x/web-interface/nav", Some(self))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(serde_json::from_value(res["data"]["wbi_img"].take())?)
    }

    /// 检查凭据是否有效
    pub async fn need_refresh(&self, client: &Client) -> Result<bool> {
        let res = client
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/cookie/info",
                Some(self),
            )
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        res["data"]["refresh"].as_bool().ok_or(anyhow!("check refresh failed"))
    }

    /// 需要使用一个需要鉴权的接口来检查是否登录
    /// 此处使用查看用户状态数的接口，该接口返回内容少，请求成本低
    pub async fn is_login(&self, client: &Client) -> Result<()> {
        client
            .request(
                Method::GET,
                "https://api.bilibili.com/x/web-interface/nav/stat",
                Some(self),
            )
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(())
    }

    pub async fn refresh(&self, client: &Client) -> Result<Self> {
        let correspond_path = Self::get_correspond_path();
        let csrf = self.get_refresh_csrf(client, correspond_path).await?;
        let new_credential = self.get_new_credential(client, &csrf).await?;
        self.confirm_refresh(client, &new_credential).await?;
        Ok(new_credential)
    }

    fn get_correspond_path() -> String {
        // 调用频率很低，让 key 在函数内部构造影响不大
        let key = RsaPublicKey::from_public_key_pem(
            "-----BEGIN PUBLIC KEY-----
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDLgd2OAkcGVtoE3ThUREbio0Eg
Uc/prcajMKXvkCKFCWhJYJcLkcM2DKKcSeFpD/j6Boy538YXnR6VhcuUJOhH2x71
nzPjfdTcqMz7djHum0qSZA0AyCBDABUqCrfNgCiJ00Ra7GmRj+YCK1NJEuewlb40
JNrRuoEUXpabUzGB8QIDAQAB
-----END PUBLIC KEY-----",
        )
        .unwrap();
        let ts = chrono::Local::now().timestamp_millis();
        let data = format!("refresh_{}", ts).into_bytes();
        let mut rng = rand::rngs::OsRng;
        let encrypted = key.encrypt(&mut rng, Oaep::new::<Sha256>(), &data).unwrap();
        hex::encode(encrypted)
    }

    async fn get_refresh_csrf(&self, client: &Client, correspond_path: String) -> Result<String> {
        let res = client
            .request(
                Method::GET,
                format!("https://www.bilibili.com/correspond/1/{}", correspond_path).as_str(),
                Some(self),
            )
            .header(header::COOKIE, "Domain=.bilibili.com")
            .send()
            .await?
            .error_for_status()?;
        regex_find(r#"<div id="1-name">(.+?)</div>"#, res.text().await?.as_str())
    }

    async fn get_new_credential(&self, client: &Client, csrf: &str) -> Result<Credential> {
        let mut res = client
            .request(
                Method::POST,
                "https://passport.bilibili.com/x/passport-login/web/cookie/refresh",
                Some(self),
            )
            .header(header::COOKIE, "Domain=.bilibili.com")
            .form(&[
                // 这里不是 json，而是 form data
                ("csrf", self.bili_jct.as_str()),
                ("refresh_csrf", csrf),
                ("refresh_token", self.ac_time_value.as_str()),
                ("source", "main_web"),
            ])
            .send()
            .await?
            .error_for_status()?;
        // 必须在 .json 前取出 headers，否则 res 会被消耗
        let headers = std::mem::take(res.headers_mut());
        let res = res.json::<serde_json::Value>().await?.validate()?;
        let set_cookies = headers.get_all(header::SET_COOKIE);
        let mut credential = Self {
            buvid3: self.buvid3.clone(),
            ..Self::default()
        };
        let required_cookies = HashSet::from(["SESSDATA", "bili_jct", "DedeUserID"]);
        let cookies: Vec<Cookie> = set_cookies
            .iter()
            .filter_map(|x| x.to_str().ok())
            .filter_map(|x| Cookie::parse(x).ok())
            .filter(|x| required_cookies.contains(x.name()))
            .collect();
        if cookies.len() != required_cookies.len() {
            bail!("not all required cookies found");
        }
        for cookie in cookies {
            match cookie.name() {
                "SESSDATA" => credential.sessdata = cookie.value().to_string(),
                "bili_jct" => credential.bili_jct = cookie.value().to_string(),
                "DedeUserID" => credential.dedeuserid = cookie.value().to_string(),
                _ => unreachable!(),
            }
        }
        if !res["data"]["refresh_token"].is_string() {
            bail!("refresh_token not found");
        }
        credential.ac_time_value = res["data"]["refresh_token"].as_str().unwrap().to_string();
        Ok(credential)
    }

    async fn confirm_refresh(&self, client: &Client, new_credential: &Credential) -> Result<()> {
        client
            .request(
                Method::POST,
                "https://passport.bilibili.com/x/passport-login/web/confirm/refresh",
                // 此处用的是新的凭证
                Some(new_credential),
            )
            .form(&[
                ("csrf", new_credential.bili_jct.as_str()),
                ("refresh_token", self.ac_time_value.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;
        Ok(())
    }
}

// 用指定的 pattern 正则表达式在 doc 中查找，返回第一个匹配的捕获组
fn regex_find(pattern: &str, doc: &str) -> Result<String> {
    let re = Regex::new(pattern)?;
    Ok(re
        .captures(doc)
        .ok_or(anyhow!("pattern not match"))?
        .get(1)
        .unwrap()
        .as_str()
        .to_string())
}

fn get_filename(url: &str) -> Option<&str> {
    url.rsplit_once('/')
        .and_then(|(_, s)| s.rsplit_once('.'))
        .map(|(s, _)| s)
}

fn get_mixin_key(wbi_img: WbiImg) -> Option<String> {
    let key = match (
        get_filename(wbi_img.img_url.as_str()),
        get_filename(wbi_img.sub_url.as_str()),
    ) {
        (Some(img_key), Some(sub_key)) => img_key.to_string() + sub_key,
        _ => return None,
    };
    let key = key.as_bytes();
    Some(MIXIN_KEY_ENC_TAB.iter().take(32).map(|&x| key[x] as char).collect())
}

pub fn encoded_query<'a>(params: Vec<(&'a str, impl Into<String>)>, mixin_key: &str) -> Vec<(&'a str, String)> {
    let params = params.into_iter().map(|(k, v)| (k, v.into())).collect();
    _encoded_query(params, mixin_key, chrono::Local::now().timestamp().to_string())
}

fn _encoded_query<'a>(params: Vec<(&'a str, String)>, mixin_key: &str, timestamp: String) -> Vec<(&'a str, String)> {
    let mut params: Vec<(&'a str, String)> = params
        .into_iter()
        .map(|(k, v)| (k, v.chars().filter(|&x| !"!'()*".contains(x)).collect::<String>()))
        .collect();
    params.push(("wts", timestamp));
    params.sort_by(|a, b| a.0.cmp(b.0));
    let query = serde_urlencoded::to_string(&params).unwrap().replace('+', "%20");
    params.push(("w_rid", format!("{:x}", md5::compute(query.clone() + mixin_key))));
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_find() {
        let doc = r#"
        <html lang="zh-Hans">
            <body>
                <div id="1-name">b0cc8411ded2f9db2cff2edb3123acac</div>
        </body>
        </html>
        "#;
        assert_eq!(
            regex_find(r#"<div id="1-name">(.+?)</div>"#, doc).unwrap(),
            "b0cc8411ded2f9db2cff2edb3123acac",
        );
    }

    #[test]
    fn test_encode_query() {
        let query = vec![
            ("bar", "五一四".to_string()),
            ("baz", "1919810".to_string()),
            ("foo", "one one four".to_string()),
        ];
        assert_eq!(
            serde_urlencoded::to_string(query).unwrap().replace('+', "%20"),
            "bar=%E4%BA%94%E4%B8%80%E5%9B%9B&baz=1919810&foo=one%20one%20four"
        );
    }

    #[test]
    fn test_wbi_key() {
        let key = WbiImg {
            img_url: "https://i0.hdslb.com/bfs/wbi/7cd084941338484aae1ad9425b84077c.png".to_string(),
            sub_url: "https://i0.hdslb.com/bfs/wbi/4932caff0ff746eab6f01bf08b70ac45.png".to_string(),
        };
        let mixin_key = get_mixin_key(key);
        assert_eq!(mixin_key, Some("ea1db124af3c7062474693fa704f4ff8".to_string()));
        assert_eq!(
            _encoded_query(
                vec![
                    ("foo", "114".to_string()),
                    ("bar", "514".to_string()),
                    ("zab", "1919810".to_string())
                ],
                &mixin_key.unwrap(),
                "1702204169".to_string(),
            ),
            vec![
                ("bar", "514".to_string()),
                ("foo", "114".to_string()),
                ("wts", "1702204169".to_string()),
                ("zab", "1919810".to_string()),
                ("w_rid", "8f6f2b5b3d485fe1886cec6a0be8c5d4".to_string()),
            ]
        )
    }
}
