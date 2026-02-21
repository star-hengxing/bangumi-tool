use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- API response types ---

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub nickname: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagedCollection {
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
    pub data: Vec<Collection>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Collection {
    pub subject_id: u64,
    #[serde(rename = "type")]
    pub collection_type: u8,
    pub rate: u8,
    pub ep_status: u64,
    pub updated_at: DateTime<Utc>,
    pub comment: Option<String>,
    pub tags: Vec<String>,
    pub subject: CollectionSubject,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CollectionSubject {
    pub id: u64,
    pub name: String,
    pub name_cn: String,
    #[serde(rename = "type")]
    pub subject_type: u8,
    pub eps: u64,
    pub volumes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubjectDetail {
    pub id: u64,
    pub name: String,
    pub name_cn: String,
    #[serde(rename = "type")]
    pub subject_type: u8,
    pub eps: u64,
    pub total_episodes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagedEpisodes {
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
    pub data: Vec<Episode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Episode {
    pub id: u64,
    #[serde(rename = "type")]
    pub episode_type: u8,
    pub sort: f64,
    pub ep: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EpisodeProgress {
    pub id: u64,
    pub status: ProgressStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProgressStatus {
    pub id: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserProgress {
    pub subject_id: u64,
    pub eps: Vec<EpisodeProgress>,
}

// --- Export types ---

/// Full record with episode/progress detail.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRecord {
    pub name: String,
    pub name_cn: String,
    pub subject_type: String,
    pub url: String,
    pub status: String,
    pub updated_at: String,
    pub completeness: String,
    pub completeness_pct: String,
    pub watched_eps: String,
    pub rating: String,
    pub tags: String,
    pub comment: String,
}

/// Simple record built from collection data only (no extra API calls).
#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleRecord {
    pub name: String,
    pub name_cn: String,
    pub subject_type: String,
    pub url: String,
    pub status: String,
    /// Raw collection type (1=wish, 2=done, 3=doing, 4=on_hold, 5=dropped).
    #[serde(skip_serializing)]
    pub collection_type: u8,
    pub updated_at: String,
    pub rating: String,
    pub tags: String,
    pub comment: String,
}

// --- Helpers ---

pub fn subject_type_name(t: u8) -> &'static str {
    match t {
        1 => "书籍",
        2 => "动画",
        3 => "音乐",
        4 => "游戏",
        6 => "三次元",
        _ => "未知",
    }
}

pub fn collection_status_name(collection_type: u8, subject_type: u8) -> &'static str {
    match (collection_type, subject_type) {
        // 书籍：想读/在读/读过
        (1, 1) => "想读",
        (2, 1) => "读过",
        (3, 1) => "在读",
        // 音乐：想听/在听/听过
        (1, 3) => "想听",
        (2, 3) => "听过",
        (3, 3) => "在听",
        // 游戏：想玩/在玩/玩过
        (1, 4) => "想玩",
        (2, 4) => "玩过",
        (3, 4) => "在玩",
        // 动画/三次元/其他：想看/在看/看过
        (1, _) => "想看",
        (2, _) => "看过",
        (3, _) => "在看",
        // 搁置/抛弃 不区分类型
        (4, _) => "搁置",
        (5, _) => "抛弃",
        _ => "未知",
    }
}

/// Encode a sorted list of episode numbers into run-length format like "1-5,7,9-12".
pub fn run_length_encode(eps: &[u64]) -> String {
    if eps.is_empty() {
        return String::new();
    }
    let mut sorted = eps.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut parts: Vec<String> = Vec::new();
    let mut start = sorted[0];
    let mut end = sorted[0];

    for &ep in &sorted[1..] {
        if ep == end + 1 {
            end = ep;
        } else {
            if start == end {
                parts.push(start.to_string());
            } else {
                parts.push(format!("{}-{}", start, end));
            }
            start = ep;
            end = ep;
        }
    }
    if start == end {
        parts.push(start.to_string());
    } else {
        parts.push(format!("{}-{}", start, end));
    }

    parts.join(",")
}
