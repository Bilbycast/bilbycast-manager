use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db;
use crate::models::{Event, EventQuery, UserInfo};

/// Full system export format.
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemExport {
    pub version: u32,
    pub exported_at: String,
    pub exported_by: String,
    pub data: ExportData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub users: Vec<UserInfo>,
    pub nodes: Vec<NodeExport>,
    pub settings: Vec<(String, String)>,
    pub config_templates: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<Event>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_log: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeExport {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub device_type: String,
    pub status: String,
    pub software_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Export all system data.
pub async fn export_all(
    pool: &SqlitePool,
    username: &str,
    include_events: bool,
    events_days: Option<u32>,
    include_audit: bool,
) -> Result<SystemExport, anyhow::Error> {
    let users = db::users::list_users(pool).await?;
    let user_infos: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();

    let nodes = db::nodes::list_nodes(pool).await?;
    let node_exports: Vec<NodeExport> = nodes
        .into_iter()
        .map(|n| NodeExport {
            id: n.id,
            name: n.name,
            description: n.description,
            device_type: n.device_type,
            status: n.status.as_str().to_string(),
            software_version: n.software_version,
            metadata: n.metadata,
        })
        .collect();

    let settings = db::settings::get_all_settings(pool).await?;

    let events = if include_events {
        let query = EventQuery {
            from: events_days.map(|days| {
                chrono::Utc::now() - chrono::Duration::days(days as i64)
            }),
            per_page: Some(10000),
            ..Default::default()
        };
        Some(db::events::query_events(pool, &query).await?)
    } else {
        None
    };

    let audit_log = if include_audit {
        let entries = db::audit::query_audit_log(pool, 10000, 0).await?;
        Some(
            entries
                .into_iter()
                .map(|e| serde_json::to_value(e).unwrap_or_default())
                .collect(),
        )
    } else {
        None
    };

    Ok(SystemExport {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        exported_by: username.to_string(),
        data: ExportData {
            users: user_infos,
            nodes: node_exports,
            settings,
            config_templates: Vec::new(), // TODO: implement template export
            events,
            audit_log,
        },
    })
}
