use std::future::Future;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::{id::Id, Repository, RepositoryResult};

pub type PlaygroundRecordId = Id<PlaygroundRecord>;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlaygroundRecord {
    pub id: PlaygroundRecordId,
    pub created_at: DateTime<Utc>,
    pub user_msg_id: i64,
    pub eval_msg_id: i64,
    pub created_by_user_id: i64,
    pub revision_id: PlaygroundRecordRevisionId,
    pub page_state: PlaygroundRecordPageState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PlaygroundRecordPageState {
    #[default]
    Output,
    Stderr,
    Miri,
}

pub type PlaygroundRecordRevisionId = Id<PlaygroundRecordRevision>;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlaygroundRecordRevision {
    pub revision_id: PlaygroundRecordRevisionId,
    pub record_revision_count: u32,
    pub perma_link: Option<String>,
    pub rendered_code: String,
    // pub code_formatted: String,
    pub warning_count: u32,
    pub error_count: u32,
    pub result_success: bool,
    pub result_code: String,
    pub result_exit_detail: String,
    pub result_stdout: String,
    pub result_stderr: String,
    pub playground_error: String,
    // pub rust_edition: PlaygroundRustEdition,
    // pub rust_channel: PlaygroundRustChannel,
    // pub rust_profile: PlaygroundRustProfile,
}

// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
// pub enum PlaygroundRustEdition {
//     Rust2015,
//     Rust2018,
//     #[default]
//     Rust2021,
//     Rust2024,
// }
//
// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
// pub enum PlaygroundRustChannel {
//     #[default]
//     Stable,
//     Beta,
//     Nightly,
// }
//
// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
// pub enum PlaygroundRustProfile {
//     #[default]
//     Debug,
//     Release,
// }
//
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRevisionUpsertRecordResult {
    pub revision_id: PlaygroundRecordRevisionId,
    pub eval_msg_id: Option<i64>,
    pub page_state: PlaygroundRecordPageState,
}
pub trait IPlaygroundRecordRepository {
    fn create_revision_upsert_record(
        &self,
        chat_id: i64,
        user_msg_id: i64,
        created_by_user_id: i64,
        rendered_code: String,
        page_state: PlaygroundRecordPageState,
    ) -> impl Future<Output = RepositoryResult<CreateRevisionUpsertRecordResult>>;
    fn update_eval_msg_id_for_revision_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
        eval_msg_id: i64,
    ) -> impl Future<Output = RepositoryResult<()>>;
    fn update_revision_for_revision_count_and_is_latest(
        &self,
        revision: &mut PlaygroundRecordRevision,
    ) -> impl Future<Output = RepositoryResult<bool>>;

    fn delete_record_by_revision_id_if_match(
        &self,
        eval_msg_id: i64,
        created_by_user_id: i64,
        revision_id: PlaygroundRecordRevisionId,
    ) -> impl Future<Output = RepositoryResult<bool>>;

    fn get_revision_update_page_state_if_match(
        &self,
        eval_msg_id: i64,
        created_by_user_id: i64,
        revision_id: PlaygroundRecordRevisionId,
        page_state: PlaygroundRecordPageState,
    ) -> impl Future<Output = RepositoryResult<Option<PlaygroundRecordRevision>>>;

    fn get_revision_by_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
    ) -> impl Future<
        Output = RepositoryResult<Option<(PlaygroundRecordRevision, PlaygroundRecordPageState)>>,
    >;
    fn update_perma_link_for_revision_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
        perma_link: String,
    ) -> impl Future<Output = RepositoryResult<()>>;
}

impl IPlaygroundRecordRepository for Repository {
    async fn create_revision_upsert_record(
        &self,
        chat_id: i64,
        user_msg_id: i64,
        created_by_user_id: i64,
        rendered_code: String,
        page_state: PlaygroundRecordPageState,
    ) -> RepositoryResult<CreateRevisionUpsertRecordResult> {
        const INSERT_REVISION_SQL: &str =
            "INSERT INTO `playground_revision` (`record_id`, `rendered_code`) VALUES (0, ?)";
        const UPSERT_RECORD_SQL: &str = "INSERT INTO `playground_record` (`chat_id`, `user_msg_id`, `created_by_user_id`, `revision_id`, `page_state`)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT (`user_msg_id`, `chat_id`)
                DO UPDATE SET `revision_id` = excluded.revision_id, `page_state` = excluded.page_state
            RETURNING `id`, `eval_msg_id`, `page_state`";
        const UPDATE_REVISION_RECORD_ID_SQL: &str =
            "UPDATE `playground_revision` SET `record_id` = ? WHERE `id` = ?";
        let revision_id = self
            .with_db(move |conn| {
                let tx = conn.transaction()?;
                let res = {
                    let mut insert_revision_stmt = tx.prepare_cached(INSERT_REVISION_SQL)?;
                    let mut upsert_record_stmt = tx.prepare_cached(UPSERT_RECORD_SQL)?;
                    let mut update_revision_record_id_stmt =
                        tx.prepare_cached(UPDATE_REVISION_RECORD_ID_SQL)?;
                    insert_revision_stmt.execute(params![rendered_code])?;
                    let revision_id =
                        PlaygroundRecordRevisionId::try_from(tx.last_insert_rowid()).unwrap();
                    let (record_id, eval_msg_id, page_state) = upsert_record_stmt.query_row(
                        params![
                            chat_id,
                            user_msg_id,
                            created_by_user_id,
                            revision_id,
                            page_state as i64
                        ],
                        |row| {
                            Ok((
                                PlaygroundRecordId::try_from(row.get::<_, i64>(0)?).unwrap(),
                                row.get(1)?,
                                row.get::<_, u8>(2)?,
                            ))
                        },
                    )?;
                    update_revision_record_id_stmt.execute(params![record_id, revision_id])?;
                    CreateRevisionUpsertRecordResult {
                        revision_id,
                        eval_msg_id,
                        page_state: decode_page_state(page_state),
                    }
                };
                tx.commit()?;
                Ok(res)
            })
            .await?;
        Ok(revision_id)
    }

    async fn update_eval_msg_id_for_revision_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
        eval_msg_id: i64,
    ) -> RepositoryResult<()> {
        const UPDATE_EVAL_MSG_ID_SQL: &str = "UPDATE `playground_record`
            SET `eval_msg_id` = ?
            WHERE `id` = (SELECT `record_id` FROM `playground_revision` WHERE `id` = ?)";
        self.with_db(move |conn| {
            let mut update_eval_msg_id_stmt = conn.prepare_cached(UPDATE_EVAL_MSG_ID_SQL)?;
            update_eval_msg_id_stmt.execute(params![eval_msg_id, revision_id])?;
            Ok(())
        })
        .await?;
        Ok(())
    }

    async fn update_revision_for_revision_count_and_is_latest(
        &self,
        revision: &mut PlaygroundRecordRevision,
    ) -> RepositoryResult<bool> {
        const UPDATE_REVISION_SQL: &str = "UPDATE `playground_revision`
            SET `perma_link` = ?, `warning_count` = ?, `error_count` = ?, `result_success` = ?, `result_code` = ?, `result_exit_detail` = ?, `result_stdout` = ?, `result_stderr` = ?, `playground_error` = ?
            WHERE `id` = ?";
        const SELECT_RECORD_REVISION_COUNT_SQL: &str = "SELECT
            COUNT(REV.`id`)
            FROM `playground_revision` REV
            INNER JOIN `playground_record` ON REV.`record_id` = `playground_record`.`id`
            WHERE
                `playground_record`.`id` = (SELECT `record_id` FROM `playground_revision` REV1 WHERE REV1.`id` = ?1)
                AND `playground_record`.`revision_id` = ?1
            GROUP BY `playground_record`.`id`
            LIMIT 1";
        let record_revision_count = self
            .with_db({
                let revision = revision.clone();
                move |conn| {
                    {
                        let mut update_revision_stmt = conn.prepare_cached(UPDATE_REVISION_SQL)?;
                        update_revision_stmt.execute(params![
                            revision.perma_link,
                            revision.warning_count,
                            revision.error_count,
                            revision.result_success,
                            revision.result_code,
                            revision.result_exit_detail,
                            revision.result_stdout,
                            revision.result_stderr,
                            revision.playground_error,
                            revision.revision_id
                        ])?;
                    }
                    let record_revision_count = conn
                        .query_row(
                            SELECT_RECORD_REVISION_COUNT_SQL,
                            params![revision.revision_id],
                            |row| row.get(0),
                        )
                        .optional()?;

                    Ok(record_revision_count)
                }
            })
            .await?;
        match record_revision_count {
            Some(count) => {
                revision.record_revision_count = count;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn delete_record_by_revision_id_if_match(
        &self,
        eval_msg_id: i64,
        created_by_user_id: i64,
        revision_id: PlaygroundRecordRevisionId,
    ) -> RepositoryResult<bool> {
        const DELETE_RECORD_SQL: &str = "UPDATE `playground_record`
            SET `eval_msg_id` = NULL
            WHERE `id` = (SELECT `record_id` FROM `playground_revision` WHERE `id` = ?)
            AND `eval_msg_id` = ?
            AND `created_by_user_id` = ?";
        let res = self
            .with_db(move |conn| {
                let mut delete_record_stmt = conn.prepare_cached(DELETE_RECORD_SQL)?;
                let affected = delete_record_stmt.execute(params![
                    revision_id,
                    eval_msg_id,
                    created_by_user_id
                ])?;

                Ok(affected > 0)
            })
            .await?;
        Ok(res)
    }

    async fn get_revision_update_page_state_if_match(
        &self,
        eval_msg_id: i64,
        created_by_user_id: i64,
        revision_id: PlaygroundRecordRevisionId,
        page_state: PlaygroundRecordPageState,
    ) -> RepositoryResult<Option<PlaygroundRecordRevision>> {
        const UPDATE_PAGE_STATE_IF_MATCH_SQL: &str = "UPDATE `playground_record`
            SET `page_state` = ?1
            WHERE `id` = (SELECT `record_id` FROM `playground_revision` WHERE `id` = ?2)
            AND `revision_id` = ?2
            AND `eval_msg_id` = ?3
            AND `created_by_user_id` = ?4";
        const SELECT_REVISION_SQL: &str = "SELECT
            REV.`id`,
            (
                SELECT COUNT(REV1.`id`) FROM `playground_revision` REV1 WHERE REV1.`record_id` = REV.`record_id` GROUP BY REV1.`record_id`
            ) AS `record_revision_count`,
            REV.`perma_link`,
            REV.`rendered_code`,
            REV.`warning_count`,
            REV.`error_count`,
            REV.`result_success`,
            REV.`result_code`,
            REV.`result_exit_detail`,
            REV.`result_stdout`,
            REV.`result_stderr`,
            REV.`playground_error`
            FROM `playground_revision` REV
            WHERE `id` = ?
            LIMIT 1";
        let res = self
            .with_db(move |conn| {
                let tx = conn.transaction()?;
                let res = {
                    let mut update_page_state_if_match_stmt =
                        tx.prepare_cached(UPDATE_PAGE_STATE_IF_MATCH_SQL)?;
                    let mut select_revision_stmt = tx.prepare_cached(SELECT_REVISION_SQL)?;
                    let affected = update_page_state_if_match_stmt.execute(params![
                        page_state as i64,
                        revision_id,
                        eval_msg_id,
                        created_by_user_id
                    ])?;
                    if affected == 0 {
                        return Ok(None);
                    }
                    let revision = select_revision_stmt
                        .query_row(params![revision_id], map_record_revision_rows)
                        .optional()?;
                    Ok(revision)
                };
                tx.commit()?;
                res
            })
            .await?;
        Ok(res)
    }

    async fn get_revision_by_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
    ) -> RepositoryResult<Option<(PlaygroundRecordRevision, PlaygroundRecordPageState)>> {
        const SELECT_REVISION_SQL: &str = "SELECT
                REV.`id`,
                (
                    SELECT COUNT(REV1.`id`) FROM `playground_revision` REV1 WHERE REV1.`record_id` = REV.`record_id` GROUP BY REV1.`record_id`
                ) AS `record_revision_count`,
                REV.`perma_link`,
                REV.`rendered_code`,
                REV.`warning_count`,
                REV.`error_count`,
                REV.`result_success`,
                REV.`result_code`,
                REV.`result_exit_detail`,
                REV.`result_stdout`,
                REV.`result_stderr`,
                REV.`playground_error`,
                `playground_record`.`page_state`
                FROM `playground_revision` REV
                INNER JOIN `playground_record` ON REV.`record_id` = `playground_record`.`id`
                WHERE REV.`id` = ?
                LIMIT 1";
        let res = self
            .with_db(move |conn| {
                let mut select_revision_stmt = conn.prepare_cached(SELECT_REVISION_SQL)?;

                let res = select_revision_stmt
                    .query_row(params![revision_id], |rows| {
                        let revision = map_record_revision_rows(rows)?;
                        let page_state = decode_page_state(rows.get(12)?);
                        Ok((revision, page_state))
                    })
                    .optional()?;
                Ok(res)
            })
            .await?;
        Ok(res)
    }
    async fn update_perma_link_for_revision_id(
        &self,
        revision_id: PlaygroundRecordRevisionId,
        perma_link: String,
    ) -> RepositoryResult<()> {
        const UPDATE_PERMA_LINK_SQL: &str = "UPDATE `playground_revision`
                SET `perma_link` = ?
                WHERE `id` = ?";
        self.with_db(move |conn| {
            let mut update_perma_link_stmt = conn.prepare_cached(UPDATE_PERMA_LINK_SQL)?;
            update_perma_link_stmt.execute(params![perma_link, revision_id])?;
            Ok(())
        })
        .await?;
        Ok(())
    }
}

fn decode_page_state(page_state: u8) -> PlaygroundRecordPageState {
    match page_state {
        1 => PlaygroundRecordPageState::Stderr,
        2 => PlaygroundRecordPageState::Miri,
        _ => PlaygroundRecordPageState::Output,
    }
}

fn map_record_revision_rows(
    row: &rusqlite::Row<'_>,
) -> Result<PlaygroundRecordRevision, rusqlite::Error> {
    let revision_id: i64 = row.get(0)?;
    Ok(PlaygroundRecordRevision {
        revision_id: PlaygroundRecordRevisionId::try_from(revision_id).unwrap(),
        record_revision_count: row.get(1)?,
        perma_link: row.get(2)?,
        rendered_code: row.get(3)?,
        warning_count: row.get(4)?,
        error_count: row.get(5)?,
        result_success: row.get(6)?,
        result_code: row.get(7)?,
        result_exit_detail: row.get(8)?,
        result_stdout: row.get(9)?,
        result_stderr: row.get(10)?,
        playground_error: row.get(11)?,
    })
}
