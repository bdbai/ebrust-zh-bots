CREATE TABLE `playground_record` (
    `id` INTEGER NOT NULL PRIMARY KEY,
    `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    `user_msg_id` INTEGER NOT NULL UNIQUE,
    `eval_msg_id` INTEGER NULL,
    `created_by_user_id` INTEGER NOT NULL,
    `revision_id` INTEGER NOT NULL,
    `page_state` INTEGER NOT NULL
);

CREATE TABLE `playground_revision` (
    `id` INTEGER NOT NULL PRIMARY KEY,
    `record_id` INTEGER NOT NULL REFERENCES `playground_record`(`id`) ON DELETE CASCADE ON UPDATE CASCADE DEFERRABLE INITIALLY DEFERRED,
    `created_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    `updated_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    `perma_link` TEXT NULL,
    `rendered_code` TEXT NOT NULL DEFAULT '',
    `warning_count` INTEGER NOT NULL DEFAULT 0,
    `error_count` INTEGER NOT NULL DEFAULT 0,
    `result_success` INTEGER NOT NULL DEFAULT 0,
    `result_code` TEXT NOT NULL DEFAULT '',
    `result_exit_detail` TEXT NOT NULL DEFAULT '',
    `result_stdout` TEXT NOT NULL DEFAULT '',
    `result_stderr` TEXT NOT NULL DEFAULT '',
    `playground_error` TEXT NOT NULL DEFAULT ''
);

CREATE TRIGGER `playground_revision_updated`
AFTER UPDATE ON `playground_revision`
FOR EACH ROW
WHEN old.`updated_at` IS NOT new.`updated_at`
BEGIN
UPDATE `playground_revision` SET `updated_at` = (strftime('%Y-%m-%d %H:%M:%f', 'now')) WHERE `id` = old.`id`;
END;

CREATE INDEX playground_revision_record_id ON `playground_revision` (`record_id`);
