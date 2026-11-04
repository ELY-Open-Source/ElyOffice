//! Vietnamese local rules for ElyMail.

pub(super) const VI: &[(&str, &str)] = &[
    ("mail.rules.field", "Xem trường"),
    ("mail.rules.from", "Từ"),
    ("mail.rules.to", "Đến hoặc Cc"),
    ("mail.rules.subject", "Chủ đề"),
    ("mail.rules.words", "Từ ngữ cần tìm"),
    ("mail.rules.action", "Thì"),
    ("mail.rules.flag", "gắn cờ"),
    ("mail.rules.archive", "lưu trữ"),
    ("mail.rules.move", "chuyển vào {name}"),
    ("mail.rules.add", "Thêm quy tắc"),
    ("mail.rules.remove", "Gỡ quy tắc"),
    ("mail.rules.said", "Thư mới có trường {field} chứa “{words}”: {action}"),
    ("mail.rules.unsaved", "Chưa lưu được quy tắc: {error}"),
    ("mail.rules.unread", "Không đọc được các quy tắc: {error}"),
    ("mail.rules.refused", "Một quy tắc không thực hiện được: {error}"),
];
