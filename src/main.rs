mod cache;
mod cli;
mod client;
mod error;
mod export;
mod models;

use std::collections::BTreeMap;
use std::path::Path;

use chrono::Local;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use indicatif_log_bridge::LogWrapper;
use log::info;

use cache::Cache;
use cli::{Args, Format};
use client::BangumiClient;
use error::Result;
use models::{
    Collection, ExportRecord, SimpleRecord, SubjectDetail, UserProgress, collection_status_name,
    run_length_encode, subject_type_name,
};

const CACHE_DIR: &str = ".bgm_cache";

fn load_token() -> Result<String> {
    if let Ok(token) = std::env::var("BANGUMI_ACCESS_TOKEN")
        && !token.is_empty()
    {
        return Ok(token.trim().to_string());
    }
    match std::fs::read_to_string(".bgm_token") {
        Ok(token) if !token.trim().is_empty() => Ok(token.trim().to_string()),
        _ => Err(error::AppError::NoToken),
    }
}

fn init_logger(debug: bool, multi: MultiProgress) {
    use std::io::Write;

    let level = if debug { "debug" } else { "warn" };
    let logger = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format(|buf, record| {
            let now = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(
                buf,
                "[{} {} {}] {}",
                now,
                record.level(),
                record.module_path().unwrap_or(""),
                record.args()
            )
        })
        .build();
    LogWrapper::new(multi, logger).try_init().ok();
}

/// Fetch all collections, using cache for each page.
async fn fetch_collections(
    client: &BangumiClient,
    cache: &Cache,
    uid: u64,
    username: &str,
    multi: &MultiProgress,
) -> Result<Vec<Collection>> {
    let mut collections = Vec::new();
    let mut offset = 0u64;
    let limit = 30u64;

    let pb = multi.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Fetching collections...");

    // First page to get total
    let cache_key = format!("{}/collections/{}", uid, offset);
    let first_page = match cache.get(&cache_key) {
        Some(page) => page,
        None => {
            let page = client.get_collections(username, limit, offset).await?;
            cache.set(&cache_key, &page)?;
            page
        }
    };
    let total = first_page.total;
    collections.extend(first_page.data);
    offset += limit;

    pb.finish_and_clear();
    multi.remove(&pb);

    let pb = multi.add(ProgressBar::new(total));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} collections")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_position(collections.len() as u64);

    while offset < total {
        let cache_key = format!("{}/collections/{}", uid, offset);
        let page = match cache.get(&cache_key) {
            Some(page) => page,
            None => {
                let page = client.get_collections(username, limit, offset).await?;
                cache.set(&cache_key, &page)?;
                page
            }
        };
        collections.extend(page.data);
        pb.set_position(collections.len() as u64);
        offset += limit;
    }
    pb.finish_with_message(format!("Fetched {} collections", collections.len()));
    multi.remove(&pb);
    Ok(collections)
}

/// Fetch subject detail with cache.
async fn fetch_subject(
    client: &BangumiClient,
    cache: &Cache,
    uid: u64,
    subject_id: u64,
) -> Result<SubjectDetail> {
    let cache_key = format!("{}/subjects/{}", uid, subject_id);
    if let Some(detail) = cache.get(&cache_key) {
        return Ok(detail);
    }
    let detail = client.get_subject(subject_id).await?;
    cache.set(&cache_key, &detail)?;
    Ok(detail)
}

/// Fetch all episodes for a subject with cache.
async fn fetch_all_episodes(
    client: &BangumiClient,
    cache: &Cache,
    uid: u64,
    subject_id: u64,
) -> Result<Vec<models::Episode>> {
    let cache_key = format!("{}/episodes/{}", uid, subject_id);
    if cache.has(&cache_key) {
        return Ok(cache
            .get::<Vec<models::Episode>>(&cache_key)
            .unwrap_or_default());
    }
    let mut all_episodes = Vec::new();
    let mut offset = 0u64;
    let limit = 100u64;
    loop {
        let page = client.get_episodes(subject_id, limit, offset).await?;
        let total = page.total;
        all_episodes.extend(page.data);
        offset += limit;
        if offset >= total {
            break;
        }
    }
    if all_episodes.is_empty() {
        cache.set_empty(&cache_key)?;
    } else {
        cache.set(&cache_key, &all_episodes)?;
    }
    Ok(all_episodes)
}

/// Fetch user progress for a subject with cache.
async fn fetch_progress(
    client: &BangumiClient,
    cache: &Cache,
    uid: u64,
    subject_id: u64,
) -> Result<Option<UserProgress>> {
    let cache_key = format!("{}/progress/{}", uid, subject_id);
    if cache.has(&cache_key) {
        return Ok(cache.get::<UserProgress>(&cache_key));
    }
    let progress = client.get_progress(uid, subject_id).await?;
    match &progress {
        Some(p) => cache.set(&cache_key, p)?,
        None => cache.set_empty(&cache_key)?,
    }
    Ok(progress)
}

/// Build a SimpleRecord from collection data only.
fn build_simple_record(col: &Collection) -> SimpleRecord {
    let updated_local = col.updated_at.with_timezone(&Local);
    SimpleRecord {
        name: col.subject.name.clone(),
        name_cn: col.subject.name_cn.clone(),
        subject_type: subject_type_name(col.subject.subject_type).to_string(),
        url: format!("https://bgm.tv/subject/{}", col.subject_id),
        status: collection_status_name(col.collection_type, col.subject.subject_type).to_string(),
        collection_type: col.collection_type,
        updated_at: updated_local.format("%Y-%m-%d %H:%M:%S").to_string(),
        rating: if col.rate == 0 {
            String::new()
        } else {
            col.rate.to_string()
        },
        tags: col.tags.join(", "),
        comment: col.comment.clone().unwrap_or_default(),
    }
}

/// Build an ExportRecord with full detail.
fn build_detail_record(
    col: &Collection,
    detail: &SubjectDetail,
    all_episodes: &[models::Episode],
    progress: &Option<UserProgress>,
) -> ExportRecord {
    let sid = col.subject_id;
    let total_eps = detail.total_episodes.max(detail.eps);

    let main_eps: Vec<_> = all_episodes
        .iter()
        .filter(|e| e.episode_type == 0)
        .collect();
    let main_ep_count = main_eps.len() as u64;

    let watched_ep_ids: Vec<u64> = progress
        .as_ref()
        .map(|p| {
            p.eps
                .iter()
                .filter(|ep| ep.status.id == 2)
                .map(|ep| ep.id)
                .collect()
        })
        .unwrap_or_default();

    let watched_sort_nums: Vec<u64> = main_eps
        .iter()
        .filter(|e| watched_ep_ids.contains(&e.id))
        .map(|e| e.sort as u64)
        .collect();

    let watched_count = watched_sort_nums.len() as u64;
    let completeness = format!("{}/{}", watched_count, main_ep_count);
    let completeness_pct = if main_ep_count > 0 {
        format!(
            "{:.0}%",
            watched_count as f64 / main_ep_count as f64 * 100.0
        )
    } else if total_eps > 0 {
        format!("{:.0}%", col.ep_status as f64 / total_eps as f64 * 100.0)
    } else {
        "N/A".to_string()
    };

    let watched_eps_str = run_length_encode(&watched_sort_nums);
    let updated_local = col.updated_at.with_timezone(&Local);

    ExportRecord {
        name: col.subject.name.clone(),
        name_cn: col.subject.name_cn.clone(),
        subject_type: subject_type_name(col.subject.subject_type).to_string(),
        url: format!("https://bgm.tv/subject/{}", sid),
        status: collection_status_name(col.collection_type, col.subject.subject_type).to_string(),
        updated_at: updated_local.format("%Y-%m-%d %H:%M:%S").to_string(),
        completeness,
        completeness_pct,
        watched_eps: watched_eps_str,
        rating: if col.rate == 0 {
            String::new()
        } else {
            col.rate.to_string()
        },
        tags: col.tags.join(", "),
        comment: col.comment.clone().unwrap_or_default(),
    }
}

/// Print collections grouped by logical status to terminal.
fn print_summary(records: &[SimpleRecord]) {
    let group_order: [(u8, &str); 5] = [
        (3, "在看/在玩/在读/在听"),
        (1, "想看/想玩/想读/想听"),
        (2, "看过/玩过/读过/听过"),
        (4, "搁置"),
        (5, "抛弃"),
    ];

    let mut by_type: BTreeMap<u8, Vec<&SimpleRecord>> = BTreeMap::new();
    for r in records {
        by_type.entry(r.collection_type).or_default().push(r);
    }

    for (ctype, label) in &group_order {
        let Some(items) = by_type.get(ctype) else {
            continue;
        };

        let mut by_status: BTreeMap<&str, Vec<&&SimpleRecord>> = BTreeMap::new();
        for r in items {
            by_status.entry(&r.status).or_default().push(r);
        }

        println!("\n== {} ({}) ==", label, items.len());
        for (status, sub_items) in &by_status {
            println!("  --- {} ({}) ---", status, sub_items.len());
            for r in sub_items {
                let display_name = if r.name_cn.is_empty() {
                    &r.name
                } else {
                    &r.name_cn
                };
                let rating_part = if r.rating.is_empty() {
                    String::new()
                } else {
                    format!(" [{}分]", r.rating)
                };
                println!("    {} [{}]{}", display_name, r.subject_type, rating_part);
            }
        }
    }
    println!();
}

/// Fetch detail for each collection item with progress bar and resume support.
async fn fetch_detail_records(
    client: &BangumiClient,
    cache: &Cache,
    multi: &MultiProgress,
    uid: u64,
    collections: &[Collection],
) -> Result<Vec<ExportRecord>> {
    let done_key = format!("{}/done_records", uid);
    let mut records: Vec<ExportRecord> = cache.get(&done_key).unwrap_or_default();
    let start_index = records.len();

    if start_index > 0 {
        println!("Resuming from record {}/{}", start_index, collections.len());
    }

    let pb = multi.add(ProgressBar::new(collections.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
    pb.set_position(start_index as u64);

    for (i, col) in collections.iter().enumerate() {
        if i < start_index {
            continue;
        }

        let sid = col.subject_id;
        let display_name = if col.subject.name_cn.is_empty() {
            &col.subject.name
        } else {
            &col.subject.name_cn
        };
        pb.set_message(display_name.clone());
        pb.set_position(i as u64);

        let detail = fetch_subject(client, cache, uid, sid).await?;
        let all_episodes = fetch_all_episodes(client, cache, uid, sid).await?;
        let progress = fetch_progress(client, cache, uid, sid).await?;

        let record = build_detail_record(col, &detail, &all_episodes, &progress);
        records.push(record);

        cache.set(&done_key, &records)?;
    }
    pb.finish_with_message("Done processing");
    multi.remove(&pb);

    Ok(records)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let multi = MultiProgress::new();
    init_logger(args.debug, multi.clone());
    let token = load_token()?;
    let client = BangumiClient::new(token)?;

    let cache = Cache::new(Path::new(CACHE_DIR))?;
    if args.no_cache {
        cache.clear()?;
        info!("Cache cleared");
    }

    let me = client.get_me().await?;
    println!("Logged in as {} ({})", me.nickname, me.username);

    let collections = fetch_collections(&client, &cache, me.id, &me.username, &multi).await?;

    let out_dir = Path::new(&args.output);
    std::fs::create_dir_all(out_dir)?;

    if args.detail {
        let records = fetch_detail_records(&client, &cache, &multi, me.id, &collections).await?;

        match args.format {
            Format::Json => export::write_json(&records, out_dir)?,
            Format::Csv => export::write_csv(&records, out_dir)?,
            Format::All => {
                export::write_json(&records, out_dir)?;
                export::write_csv(&records, out_dir)?;
            }
        }

        println!("Done! Exported {} records.", records.len());
    } else {
        let records: Vec<SimpleRecord> = collections.iter().map(build_simple_record).collect();

        print_summary(&records);

        match args.format {
            Format::Json => export::write_simple_json(&records, out_dir)?,
            Format::Csv => export::write_simple_csv(&records, out_dir)?,
            Format::All => {
                export::write_simple_json(&records, out_dir)?;
                export::write_simple_csv(&records, out_dir)?;
            }
        }

        println!("Done! Exported {} records.", records.len());
    }

    Ok(())
}
