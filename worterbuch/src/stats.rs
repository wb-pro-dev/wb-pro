use crate::{server::common::CloneableWbApi, INTERNAL_CLIENT_ID};
use serde_json::json;
use std::time::Duration;
use tokio::{
    select,
    time::{interval, Instant},
};
use tokio_graceful_shutdown::SubsystemHandle;
#[cfg(feature = "foss")]
use worterbuch_common::SYSTEM_TOPIC_SOURCES;
use worterbuch_common::{
    error::WorterbuchResult, topic, SYSTEM_TOPIC_LICENSE, SYSTEM_TOPIC_ROOT, SYSTEM_TOPIC_VERSION,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(feature = "foss")]
pub const LICENSE: &str = env!("CARGO_PKG_LICENSE");
#[cfg(not(feature = "foss"))]
pub const LICENSE: &str = "PROPRIETARY";
#[cfg(feature = "foss")]
pub const REPO: &str = env!("CARGO_PKG_REPOSITORY");

pub async fn track_stats(wb: CloneableWbApi, subsys: SubsystemHandle) -> WorterbuchResult<()> {
    let start = Instant::now();

    wb.set(
        topic!(SYSTEM_TOPIC_ROOT, SYSTEM_TOPIC_VERSION),
        json!(VERSION),
        INTERNAL_CLIENT_ID.to_owned(),
    )
    .await?;

    wb.set(
        topic!(SYSTEM_TOPIC_ROOT, SYSTEM_TOPIC_LICENSE),
        json!(LICENSE),
        INTERNAL_CLIENT_ID.to_owned(),
    )
    .await?;

    #[cfg(feature = "foss")]
    wb.set(
        topic!(SYSTEM_TOPIC_ROOT, SYSTEM_TOPIC_SOURCES),
        json!(format!("{REPO}/releases/tag/v{VERSION}")),
        INTERNAL_CLIENT_ID.to_owned(),
    )
    .await?;

    let mut interval = interval(Duration::from_secs(1));

    loop {
        select! {
            _ = interval.tick() => update_stats(&wb, start).await?,
            _ = subsys.on_shutdown_requested() => break,
        }
    }

    Ok(())
}

async fn update_stats(wb: &CloneableWbApi, start: Instant) -> WorterbuchResult<()> {
    update_uptime(wb, start.elapsed()).await?;
    update_message_count(wb).await?;
    Ok(())
}

async fn update_uptime(wb: &CloneableWbApi, uptime: Duration) -> WorterbuchResult<()> {
    wb.set(
        format!("{SYSTEM_TOPIC_ROOT}/uptime"),
        json!(uptime.as_secs()),
        INTERNAL_CLIENT_ID.to_owned(),
    )
    .await
}

async fn update_message_count(wb: &CloneableWbApi) -> WorterbuchResult<()> {
    let len = wb.len().await?;
    wb.set(
        format!("{SYSTEM_TOPIC_ROOT}/store/values/count"),
        json!(len),
        INTERNAL_CLIENT_ID.to_owned(),
    )
    .await
}
