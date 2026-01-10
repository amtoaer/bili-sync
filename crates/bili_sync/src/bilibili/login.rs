use anyhow::{Context, Result, ensure};
use cookie::Cookie;
use reqwest::{Method, header};
use serde::{Deserialize, Serialize};

use super::{Client, Credential, Validate};

/// 二维码生成响应
#[derive(Debug, Serialize, Deserialize)]
pub struct QrcodeLoginResponse {
    pub url: String,
    pub qrcode_key: String,
}

/// B站扫码登录轮询状态码
///
/// 参考: https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/docs/login/login_action/QR.md
mod qrcode_status_code {
    /// 登录成功
    pub const SUCCESS: i64 = 0;
    /// 二维码未扫描
    pub const NOT_SCANNED: i64 = 86101;
    /// 已扫描未确认
    pub const SCANNED_UNCONFIRMED: i64 = 86090;
    /// 二维码已过期
    pub const EXPIRED: i64 = 86038;
}

/// 轮询登录状态
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollStatus {
    /// 登录成功
    Success { credential: Credential },
    /// 等待中（未扫描或已扫描未确认）
    Pending {
        message: String,
        #[serde(default)]
        scanned: bool,
    },
    /// 二维码已过期
    Expired { message: String },
}

impl Client {
    /// 生成扫码登录二维码
    ///
    /// 调用 B 站 API 获取二维码信息。注意：返回的 `url` 不是直接的图片链接，
    /// 需要使用二维码库将其转换为二维码图片。
    ///
    /// # 返回
    ///
    /// 返回 `QrcodeLoginResponse`，包含：
    /// - `url`: 需要转换为二维码的 URL 字符串
    /// - `qrcode_key`: 用于轮询登录状态的认证 token（32 字符）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let response = client.generate_qrcode().await?;
    /// // 使用 qrcode 库将 response.url 转换为二维码图片
    /// let qr_image = qrcode::to_data_url(&response.url)?;
    /// ```
    pub async fn generate_qrcode(&self) -> Result<QrcodeLoginResponse> {
        let mut res = self
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/qrcode/generate",
                None,
            )
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .validate()?;

        let data = &mut res["data"];
        let response = QrcodeLoginResponse {
            url: data["url"].take().as_str().context("missing 'url' field")?.to_string(),
            qrcode_key: data["qrcode_key"]
                .take()
                .as_str()
                .context("missing 'qrcode_key' field")?
                .to_string(),
        };

        Ok(response)
    }

    /// 轮询扫码登录状态
    ///
    /// 使用 `qrcode_key` 查询登录状态，根据 B 站返回的状态码判断：
    /// - `0`: 登录成功，从响应头提取 cookies 和响应体提取 refresh_token
    /// - `86101`: 未扫描
    /// - `86090`: 已扫描但未确认
    /// - `86038`: 二维码已过期
    ///
    /// # 参数
    ///
    /// * `qrcode_key` - 二维码认证 token（由 `generate_qrcode` 返回）
    ///
    /// # 返回
    ///
    /// 返回 `PollStatus` 枚举，表示当前登录状态
    ///
    /// # 注意
    ///
    /// - 登录成功时，如果响应中没有 `buvid3`，会自动调用 API 获取
    /// - 必须先克隆 `headers`，因为后续 `json()` 会消耗 response
    pub async fn poll_qrcode(&self, qrcode_key: &str) -> Result<PollStatus> {
        let resp = self
            .request(
                Method::GET,
                "https://passport.bilibili.com/x/passport-login/web/qrcode/poll",
                None,
            )
            .query(&[("qrcode_key", qrcode_key)])
            .send()
            .await?
            .error_for_status()?;

        // 先克隆 headers，因为之后 json() 会消耗 response
        let headers = resp.headers().clone();

        // 解析 JSON 响应以获取状态码
        let json = resp.json::<serde_json::Value>().await?.validate()?;
        let code = json["data"]["code"].as_i64().context("missing 'code' field in data")?;

        match code {
            // 登录成功 - 从响应头提取 cookies 和响应体提取 refresh_token
            qrcode_status_code::SUCCESS => {
                let mut credential = extract_credential(&headers, &json)?;

                // 如果 buvid3 为空，主动获取
                if credential.buvid3.is_empty() {
                    credential.buvid3 = self.get_buvid3().await?;
                }

                Ok(PollStatus::Success { credential })
            }
            // 未扫描
            qrcode_status_code::NOT_SCANNED => Ok(PollStatus::Pending {
                message: "未扫描".to_string(),
                scanned: false,
            }),
            // 已扫描但未确认
            qrcode_status_code::SCANNED_UNCONFIRMED => Ok(PollStatus::Pending {
                message: "已扫描，请在手机上确认登录".to_string(),
                scanned: true,
            }),
            // 二维码已过期
            qrcode_status_code::EXPIRED => Ok(PollStatus::Expired {
                message: "二维码已过期".to_string(),
            }),
            // 其他未知状态码
            _ => {
                let message = json["data"]["message"].as_str().unwrap_or("未知错误").to_string();
                anyhow::bail!("未知的轮询状态码: {}, 消息: {}", code, message)
            }
        }
    }

    /// 获取 buvid3 浏览器指纹
    ///
    /// 当扫码登录成功后，B 站通常不会在 Set-Cookie 中返回 `buvid3`。
    /// 需要调用此 API 主动获取。
    ///
    /// # 参考
    ///
    /// - [B 站 API 文档](https://github.com/SocialSisterYi/bilibili-API-collect/blob/master/docs/misc/buvid3_4.md)
    ///
    /// # 返回
    ///
    /// 返回 buvid3 字符串，格式如：`"54E5EFC1-3C8F-F690-2261-439E4F6A20A979439infoc"`
    ///
    /// # 错误
    ///
    /// - API 返回 code 不为 0
    /// - 响应中缺少 `data.buvid` 字段
    async fn get_buvid3(&self) -> Result<String> {
        let resp = self
            .request(Method::GET, "https://api.bilibili.com/x/web-frontend/getbuvid", None)
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        let code = resp["code"].as_i64().context("missing 'code' field")?;
        ensure!(code == 0, "获取 buvid3 失败: code = {}", code);

        resp["data"]["buvid"]
            .as_str()
            .context("missing 'buvid' field in data")
            .map(|s| s.to_string())
    }
}

/// 从响应头和响应体中提取凭证
/// - 从 Set-Cookie headers 中提取：SESSDATA, bili_jct, buvid3（如果存在）, DedeUserID
/// - 从 JSON 响应的 data.refresh_token 中提取：ac_time_value
/// 注意：buvid3 可能为空（扫码登录通常不会返回），需要额外获取
fn extract_credential(headers: &header::HeaderMap, json: &serde_json::Value) -> Result<Credential> {
    let mut credential = Credential::default();

    // 从 Set-Cookie headers 中提取 cookies
    for cookie_str in headers.get_all(header::SET_COOKIE) {
        let cookie_str = cookie_str.to_str().context("invalid cookie header")?;
        let cookie = Cookie::parse(cookie_str).context("failed to parse cookie")?;

        match cookie.name() {
            "SESSDATA" => credential.sessdata = cookie.value().to_string(),
            "bili_jct" => credential.bili_jct = cookie.value().to_string(),
            "buvid3" => credential.buvid3 = cookie.value().to_string(),
            "DedeUserID" => credential.dedeuserid = cookie.value().to_string(),
            _ => {}
        }
    }

    // 从 JSON 响应体中提取 refresh_token 作为 ac_time_value
    if let Some(refresh_token) = json["data"]["refresh_token"].as_str() {
        credential.ac_time_value = refresh_token.to_string();
    }

    // 验证凭证完整性（buvid3 可以为空，需要额外获取）
    ensure!(!credential.sessdata.is_empty(), "SESSDATA not found in cookies");
    ensure!(!credential.bili_jct.is_empty(), "bili_jct not found in cookies");
    ensure!(!credential.dedeuserid.is_empty(), "DedeUserID not found in cookies");
    ensure!(
        !credential.ac_time_value.is_empty(),
        "refresh_token not found in response data"
    );

    Ok(credential)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试凭证提取 - 成功场景
    #[test]
    fn test_extract_credential_success() {
        // Mock headers 和 json
        let mut headers = header::HeaderMap::new();
        headers.append(
            header::SET_COOKIE,
            "SESSDATA=test_sessdata; Path=/; Domain=bilibili.com".parse().unwrap(),
        );
        headers.append(
            header::SET_COOKIE,
            "bili_jct=test_jct; Path=/; Domain=bilibili.com".parse().unwrap(),
        );
        headers.append(
            header::SET_COOKIE,
            "DedeUserID=123456; Path=/; Domain=bilibili.com".parse().unwrap(),
        );

        let json = serde_json::json!({
            "data": {
                "refresh_token": "test_refresh_token"
            }
        });

        let credential = extract_credential(&headers, &json).unwrap();

        assert_eq!(credential.sessdata, "test_sessdata");
        assert_eq!(credential.bili_jct, "test_jct");
        assert_eq!(credential.dedeuserid, "123456");
        assert_eq!(credential.ac_time_value, "test_refresh_token");
        // buvid3 可以为空
        assert!(credential.buvid3.is_empty());
    }

    /// 测试凭证提取 - 缺少必需字段
    #[test]
    fn test_extract_credential_missing_sessdata() {
        let headers = header::HeaderMap::new();
        // 没有 SESSDATA
        let json = serde_json::json!({
            "data": {
                "refresh_token": "test_refresh_token"
            }
        });

        let result = extract_credential(&headers, &json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SESSDATA not found"));
    }

    /// 测试凭证提取 - 包含 buvid3
    #[test]
    fn test_extract_credential_with_buvid3() {
        let mut headers = header::HeaderMap::new();
        headers.append(header::SET_COOKIE, "SESSDATA=test_sessdata".parse().unwrap());
        headers.append(header::SET_COOKIE, "bili_jct=test_jct".parse().unwrap());
        headers.append(header::SET_COOKIE, "buvid3=test_buvid3".parse().unwrap());
        headers.append(header::SET_COOKIE, "DedeUserID=123456".parse().unwrap());

        let json = serde_json::json!({
            "data": {
                "refresh_token": "test_refresh_token"
            }
        });

        let credential = extract_credential(&headers, &json).unwrap();
        assert_eq!(credential.buvid3, "test_buvid3");
    }

    /// 测试凭证提取 - 缺少 refresh_token
    #[test]
    fn test_extract_credential_missing_refresh_token() {
        let mut headers = header::HeaderMap::new();
        headers.append(header::SET_COOKIE, "SESSDATA=test_sessdata".parse().unwrap());
        headers.append(header::SET_COOKIE, "bili_jct=test_jct".parse().unwrap());
        headers.append(header::SET_COOKIE, "DedeUserID=123456".parse().unwrap());

        let json = serde_json::json!({
            "data": {
                // 没有 refresh_token
            }
        });

        let result = extract_credential(&headers, &json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("refresh_token not found"));
    }

    /// 完整的扫码登录流程测试（需要手动扫码）
    #[ignore = "requires manual testing with real QR code scan"]
    #[tokio::test]
    async fn test_qrcode_login_flow() -> Result<()> {
        let client = Client::new();

        // 1. 生成二维码
        let qr_response = client.generate_qrcode().await?;
        println!("二维码 URL: {}", qr_response.url);
        println!("qrcode_key: {}", qr_response.qrcode_key);
        println!("\n请使用 B 站 APP 扫描二维码...\n");

        // 2. 轮询登录状态（最多轮询 90 次，每 2 秒一次，共 180 秒）
        for i in 1..=90 {
            println!("第 {} 次轮询...", i);
            let status = client.poll_qrcode(&qr_response.qrcode_key).await?;

            match status {
                PollStatus::Success { credential } => {
                    println!("\n登录成功！");
                    println!("SESSDATA: {}", credential.sessdata);
                    println!("bili_jct: {}", credential.bili_jct);
                    println!("buvid3: {}", credential.buvid3);
                    println!("DedeUserID: {}", credential.dedeuserid);
                    println!("ac_time_value: {}", credential.ac_time_value);
                    return Ok(());
                }
                PollStatus::Pending { message, scanned } => {
                    println!("状态: {}, 已扫描: {}", message, scanned);
                }
                PollStatus::Expired { message } => {
                    println!("\n二维码已过期: {}", message);
                    anyhow::bail!("二维码过期");
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        anyhow::bail!("轮询超时")
    }
}
