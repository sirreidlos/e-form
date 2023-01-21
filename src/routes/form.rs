struct Form {
    id: u64,
    title: String,
    description: String,
    questions: Vec<Question>,
}

struct Question {
    id: u64,
    text: String,
    kind: QuestionType,
    input: Option<Vec<String>>,
}

enum QuestionType {
    TextAnswer,
    MultipleChoice,
    Checkboxes,
    Dropdown,
    Date,
    Time,
}
