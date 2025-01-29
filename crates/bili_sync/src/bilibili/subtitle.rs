use std::fmt::Display;

#[derive(Debug, serde::Deserialize)]
pub struct SubTitlesInfo {
    pub subtitles: Vec<SubTitleInfo>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleInfo {
    pub lan: String,
    pub subtitle_url: String,
}

pub struct SubTitle {
    pub lan: String,
    pub body: SubTitleBody,
}

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleBody(pub Vec<SubTitleItem>);

#[derive(Debug, serde::Deserialize)]
pub struct SubTitleItem {
    from: f64,
    to: f64,
    content: String,
}

impl SubTitleInfo {
    pub fn is_ai_sub(&self) -> bool {
        // ai： aisubtitle.hdslb.com/bfs/ai_subtitle/xxxx
        // 非 ai： aisubtitle.hdslb.com/bfs/subtitle/xxxx
        self.subtitle_url.contains("ai_subtitle")
    }
}

impl Display for SubTitleBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, item) in self.0.iter().enumerate() {
            writeln!(f, "{}", idx)?;
            writeln!(f, "{} --> {}", format_time(item.from), format_time(item.to))?;
            writeln!(f, "{}", item.content)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

fn format_time(time: f64) -> String {
    let (second, millisecond) = (time.trunc(), (time.fract() * 1e3) as u32);
    let (hour, minute, second) = (
        (second / 3600.0) as u32,
        ((second % 3600.0) / 60.0) as u32,
        (second % 60.0) as u32,
    );
    format!("{:02}:{:02}:{:02},{:03}", hour, minute, second, millisecond)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_format_time() {
        // float 解析会有精度问题，但误差几毫秒应该不太关键
        // 想再健壮一点就得手写 serde_json 解析拆分秒和毫秒，然后分别处理了
        let testcases = [
            (0.0, "00:00:00,000"),
            (1.5, "00:00:01,500"),
            (206.45, "00:03:26,449"),
            (360001.23, "100:00:01,229"),
        ];
        for (time, expect) in testcases.iter() {
            assert_eq!(super::format_time(*time), *expect);
        }
    }
}
