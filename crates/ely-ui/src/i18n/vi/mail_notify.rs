//! Vietnamese notices of new mail, and its files, for ElyMail.

pub(super) const VI: &[(&str, &str)] = &[
    ("mail.notify.title", "Thư mới"),
    ("mail.notify.body.one", "Từ {senders}"),
    ("mail.notify.body", "{count} thư mới từ {senders}"),
    ("mail.notify.more", "{senders} và những người khác"),
    ("mail.file.downloading", "Đang tải thư cùng các tệp của nó…"),
    ("mail.file.download_failed", "Chưa tải được các tệp: {error}"),
    ("mail.file.unread", "Không đọc được tệp: {error}"),
    ("mail.file.unsaved", "Chưa lưu được tệp: {error}"),
    ("mail.file.saved", "Đã lưu {name}"),
    ("mail.file.unopened", "Tệp không mở được: {error}"),
    ("mail.file.unsafe", "Loại tệp này có thể chạy chương trình, nên không được mở ở đây. Hãy lưu nó, rồi tự mở nếu bạn tin người gửi."),
];
