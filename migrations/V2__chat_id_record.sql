PRAGMA foreign_keys = OFF;

ALTER TABLE `playground_record` RENAME TO `temp_playground_record`;

CREATE TABLE playground_record (
    `id` INTEGER NOT NULL PRIMARY KEY,
    `created_at` TEXT NOT NULL DEFAULT  (strftime('%Y-%m-%d  %H:%M:%f', 'now')),
    `chat_id` INTEGER NOT NULL DEFAULT 0,  -- New column added first
    `user_msg_id` INTEGER NOT NULL,        -- Removed UNIQUE constraint
    `eval_msg_id` INTEGER NULL,
    `created_by_user_id` INTEGER NOT NULL,
    `revision_id` INTEGER NOT NULL,
    `page_state` INTEGER NOT NULL
);

INSERT INTO playground_record (
    `id`, `created_at`, `chat_id`, `user_msg_id`, `eval_msg_id`, 
    `created_by_user_id`, `revision_id`, `page_state`
)
SELECT 
    `id`, `created_at`, 0 AS `chat_id`, `user_msg_id`, `eval_msg_id`,
    `created_by_user_id`, `revision_id`, `page_state` 
FROM `temp_playground_record`;

DROP TABLE `temp_playground_record`;

CREATE UNIQUE INDEX playground_record_user_msg_id_chat_id ON `playground_record`(`user_msg_id`, `chat_id`);

PRAGMA foreign_keys = ON;
