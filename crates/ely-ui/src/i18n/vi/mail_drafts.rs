//! Vietnamese ElyMail drafts, here and on the server.

pub(super) const VI: &[(&str, &str)] = &[
    ("mail.boxes.drafts", "Thư nháp"),
    ("mail.drafts.pick", "Chọn một thư nháp để xem"),
    ("mail.drafts.to", "Đến"),
    ("mail.drafts.cc", "Cc"),
    ("mail.drafts.bcc", "Bcc"),
    ("mail.drafts.saved", "Đã lưu"),
    ("mail.drafts.continue", "Viết tiếp"),
    ("mail.drafts.delete", "Xóa thư nháp…"),
    ("mail.drafts.kept", "Thư được giữ trong Thư nháp"),
    ("mail.drafts.unkept", "Chưa lưu được thư nháp: {error}"),
    ("mail.drafts.undropped", "Chưa xóa được thư nháp: {error}"),
    ("mail.drafts.unread", "Không đọc được thư nháp: {error}"),
    ("mail.drafts.up", "Đồng thời giữ thư nháp trong thư mục Thư nháp trên máy chủ"),
    ("mail.drafts.unsaved_choice", "Chưa lưu được lựa chọn về thư nháp: {error}"),
    ("mail.drafts.no_folder", "Đã giữ trên máy tính này. Máy chủ chưa có thư mục Thư nháp nào."),
    ("mail.drafts.server_failed", "Đã giữ trên máy tính này. Bản trên máy chủ chưa được lưu: {error}"),
    ("mail.drafts.server_stale", "Một bản cũ hơn có thể vẫn còn trong Thư nháp trên máy chủ: {error}"),
];
