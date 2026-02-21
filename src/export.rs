use std::path::Path;

use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::models::{ExportRecord, SimpleRecord};

/// Common fields shared by all export record types.
trait Exportable {
    fn name(&self) -> &str;
    fn name_cn(&self) -> &str;
    fn subject_type(&self) -> &str;
    fn status(&self) -> &str;
    fn updated_at(&self) -> &str;
    fn rating(&self) -> &str;
    fn tags(&self) -> &str;
    fn comment(&self) -> &str;

    /// CSV column headers.
    fn csv_headers() -> &'static [&'static str];
    /// CSV row values, in the same order as `csv_headers`.
    fn csv_row(&self) -> Vec<&str>;

    /// Extra JSON fields beyond the common ones. Default: none.
    fn extra_json_fields(&self, _m: &mut Map<String, Value>) {}
}

impl Exportable for SimpleRecord {
    fn name(&self) -> &str {
        &self.name
    }
    fn name_cn(&self) -> &str {
        &self.name_cn
    }
    fn subject_type(&self) -> &str {
        &self.subject_type
    }
    fn status(&self) -> &str {
        &self.status
    }
    fn updated_at(&self) -> &str {
        &self.updated_at
    }
    fn rating(&self) -> &str {
        &self.rating
    }
    fn tags(&self) -> &str {
        &self.tags
    }
    fn comment(&self) -> &str {
        &self.comment
    }

    fn csv_headers() -> &'static [&'static str] {
        &[
            "名称",
            "名称(中文)",
            "条目类型",
            "地址",
            "状态",
            "最后标注",
            "我的评分",
            "我的标签",
            "我的评论",
        ]
    }

    fn csv_row(&self) -> Vec<&str> {
        vec![
            &self.name,
            &self.name_cn,
            &self.subject_type,
            &self.url,
            &self.status,
            &self.updated_at,
            &self.rating,
            &self.tags,
            &self.comment,
        ]
    }
}

impl Exportable for ExportRecord {
    fn name(&self) -> &str {
        &self.name
    }
    fn name_cn(&self) -> &str {
        &self.name_cn
    }
    fn subject_type(&self) -> &str {
        &self.subject_type
    }
    fn status(&self) -> &str {
        &self.status
    }
    fn updated_at(&self) -> &str {
        &self.updated_at
    }
    fn rating(&self) -> &str {
        &self.rating
    }
    fn tags(&self) -> &str {
        &self.tags
    }
    fn comment(&self) -> &str {
        &self.comment
    }

    fn csv_headers() -> &'static [&'static str] {
        &[
            "名称",
            "名称(中文)",
            "条目类型",
            "地址",
            "状态",
            "最后标注",
            "完成度",
            "完成度(百分比)",
            "完成单集",
            "我的评分",
            "我的标签",
            "我的评论",
        ]
    }

    fn csv_row(&self) -> Vec<&str> {
        vec![
            &self.name,
            &self.name_cn,
            &self.subject_type,
            &self.url,
            &self.status,
            &self.updated_at,
            &self.completeness,
            &self.completeness_pct,
            &self.watched_eps,
            &self.rating,
            &self.tags,
            &self.comment,
        ]
    }

    fn extra_json_fields(&self, m: &mut Map<String, Value>) {
        if !self.completeness.is_empty() {
            m.insert("progress".into(), json!(self.completeness));
        }
        if !self.completeness_pct.is_empty() && self.completeness_pct != "N/A" {
            m.insert("progress_pct".into(), json!(self.completeness_pct));
        }
        if !self.watched_eps.is_empty() {
            m.insert("watched".into(), json!(self.watched_eps));
        }
    }
}

/// Build a compact JSON value, omitting empty optional fields.
fn to_compact(r: &impl Exportable) -> Value {
    let name = if r.name_cn().is_empty() {
        r.name()
    } else {
        r.name_cn()
    };
    let mut m = Map::new();
    m.insert("name".into(), json!(name));
    if !r.name_cn().is_empty() && r.name() != r.name_cn() {
        m.insert("name_orig".into(), json!(r.name()));
    }
    m.insert("type".into(), json!(r.subject_type()));
    m.insert("status".into(), json!(r.status()));
    m.insert("updated".into(), json!(r.updated_at()));
    r.extra_json_fields(&mut m);
    if !r.rating().is_empty() {
        m.insert(
            "rating".into(),
            json!(r.rating().parse::<u8>().unwrap_or(0)),
        );
    }
    if !r.tags().is_empty() {
        m.insert("tags".into(), json!(r.tags()));
    }
    if !r.comment().is_empty() {
        m.insert("comment".into(), json!(r.comment()));
    }
    Value::Object(m)
}

fn write_json_impl(records: &[impl Exportable], dir: &Path) -> Result<()> {
    let path = dir.join("bangumi_export.json");
    let compact: Vec<Value> = records.iter().map(to_compact).collect();
    let file = std::fs::File::create(&path)?;
    serde_json::to_writer(file, &compact)?;
    println!("JSON exported to {}", path.display());
    Ok(())
}

fn write_csv_impl<T: Exportable>(records: &[T], dir: &Path) -> Result<()> {
    let path = dir.join("bangumi_export.csv");
    let mut file = std::fs::File::create(&path)?;
    std::io::Write::write_all(&mut file, b"\xEF\xBB\xBF")?;

    let mut wtr = csv::Writer::from_writer(file);
    wtr.write_record(T::csv_headers())?;
    for r in records {
        wtr.write_record(r.csv_row())?;
    }
    wtr.flush()?;
    println!("CSV exported to {}", path.display());
    Ok(())
}

pub fn write_json(records: &[ExportRecord], dir: &Path) -> Result<()> {
    write_json_impl(records, dir)
}

pub fn write_csv(records: &[ExportRecord], dir: &Path) -> Result<()> {
    write_csv_impl(records, dir)
}

pub fn write_simple_json(records: &[SimpleRecord], dir: &Path) -> Result<()> {
    write_json_impl(records, dir)
}

pub fn write_simple_csv(records: &[SimpleRecord], dir: &Path) -> Result<()> {
    write_csv_impl(records, dir)
}
