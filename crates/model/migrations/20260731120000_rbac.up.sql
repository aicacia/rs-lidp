CREATE TABLE management_roles (
    `id` INTEGER PRIMARY KEY,

    `name` TEXT NOT NULL UNIQUE,
    `description` TEXT,

    `created_at` INTEGER NOT NULL DEFAULT (unixepoch()),
    `updated_at` INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE management_user_roles (
    `id` INTEGER PRIMARY KEY,

    `user_id` INTEGER NOT NULL
        REFERENCES `users`(`id`) ON DELETE CASCADE,

    `role_id` INTEGER NOT NULL
        REFERENCES `management_roles`(`id`) ON DELETE CASCADE,

    `created_at` INTEGER NOT NULL DEFAULT (unixepoch()),
    `updated_at` INTEGER NOT NULL DEFAULT (unixepoch()),

    UNIQUE(`user_id`, `role_id`)
);

CREATE INDEX `idx_management_user_roles_user_id`
    ON `management_user_roles`(`user_id`);

CREATE INDEX `idx_management_user_roles_role_id`
    ON `management_user_roles`(`role_id`);
