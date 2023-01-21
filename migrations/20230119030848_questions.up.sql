-- Add up migration script here
CREATE TABLE `questions`(
    `id` int NOT NULL AUTO_INCREMENT,
    `form_id` VARCHAR(255) NOT NULL,
    `text` TEXT NOT NULL,
    `type` ENUM(
        "TextAnswer",
        "MultipleChoice",
        "Checkboxes",
        "Dropdown",
        "Date",
        "Time",
        "DateTime"
    ) NOT NULL,
    `option` TEXT,
    UNIQUE(id),
    PRIMARY KEY (id),
    FOREIGN KEY (form_id) REFERENCES forms(id)
);