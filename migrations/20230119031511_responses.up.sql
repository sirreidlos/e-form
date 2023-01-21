-- Add up migration script here
CREATE TABLE `responses` (
    `id` int AUTO_INCREMENT NOT NULL,
    `form_id` VARCHAR(255) NOT NULL,
    `question_number` int NOT NULL,
    `answer` TEXT NOT NULL,
    `created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP(),
    UNIQUE(id),
    PRIMARY KEY (id),
    FOREIGN KEY (form_id) REFERENCES forms(id)
)